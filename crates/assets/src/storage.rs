use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use anyhow::Result;
use dyn_inject::Registry;
use inject::DI;
use poll_promise::Promise;
use scheduler::EventBus;
use slotmap::SlotMap;

use crate::asset::Asset;
use crate::handle::Handle;

enum AssetEntry<A: Send + 'static> {
    Pending(Promise<Result<A>>),
    Failed(anyhow::Error),
    Ready(A),
}

pub enum AssetRef<'a, A> {
    Failed(&'a anyhow::Error),
    Pending,
    Ready(&'a A),
}

struct AssetContainer<A: Send + 'static> {
    items: SlotMap<Handle<A>, AssetEntry<A>>,
}

impl<A: Send + 'static> Default for AssetContainer<A> {
    fn default() -> Self {
        Self {
            items: SlotMap::default(),
        }
    }
}

impl<A: Send + 'static> AssetContainer<A> {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
struct AssetStorageInner {
    containers: Registry,
}

impl AssetStorageInner {
    // Does not insert a new container if none existed
    fn with_container<'a, A, F, R>(&'a self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockReadGuard<'a, AssetContainer<A>>) -> R, {
        let container = self.containers.read_sync::<AssetContainer<A>>().unwrap();
        f(container)
    }

    fn with_new_container<'a, A, F, R>(&'a mut self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockReadGuard<'a, AssetContainer<A>>) -> R, {
        self.containers
            .put_sync::<AssetContainer<A>>(AssetContainer::new());
        let container = self.containers.read_sync::<AssetContainer<A>>().unwrap();
        f(container)
    }

    fn with_mut_container<'a, A, F, R>(&'a mut self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockWriteGuard<'a, AssetContainer<A>>) -> R, {
        if self.containers.read_sync::<AssetContainer<A>>().is_none() {
            self.containers
                .put_sync::<AssetContainer<A>>(AssetContainer::new());
        }
        let container = self.containers.write_sync::<AssetContainer<A>>().unwrap();
        f(container)
    }
}

pub struct AssetStorage {
    inner: RwLock<AssetStorageInner>,
    bus: EventBus<DI>,
}

impl AssetStorage {
    fn poll_entry<A: Send>(entry: &AssetEntry<A>) -> AssetRef<A> {
        match entry {
            AssetEntry::Pending(_) => AssetRef::Pending,
            AssetEntry::Failed(error) => AssetRef::Failed(error),
            AssetEntry::Ready(asset) => AssetRef::Ready(asset),
        }
    }

    fn with_container<A, F, R>(&self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockReadGuard<AssetContainer<A>>) -> R, {
        let lock = self.inner.read().unwrap();
        let maybe_container = lock.containers.read_sync::<AssetContainer<A>>();
        match maybe_container {
            None => {
                drop(maybe_container);
                drop(lock);
                let mut lock = self.inner.write().unwrap();
                lock.with_new_container(f)
            }
            Some(container) => f(container),
        }
    }

    pub fn new(bus: EventBus<DI>) -> Self {
        Self {
            inner: RwLock::new(AssetStorageInner::default()),
            bus,
        }
    }

    /// Calls the provided callback function with the asset corresponding to given handle and return its
    /// result.
    /// Does not call the function if the asset was not found, and returns None instead.
    pub fn with<A, R, F>(&self, handle: Handle<A>, f: F) -> Option<R>
    where
        A: Asset + Send + 'static,
        F: FnOnce(AssetRef<A>) -> R, {
        self.with_container(|container| {
            let entry = container.items.get(handle);
            entry.map(|entry| {
                let asset = Self::poll_entry(entry);
                f(asset)
            })
        })
    }

    pub fn with_if_ready<A, R, F>(&self, handle: Handle<A>, f: F) -> Option<R>
    where
        A: Asset + Send + 'static,
        F: FnOnce(&A) -> R, {
        self.with(handle, |asset| {
            match asset {
                // Failed assets will be collected by the asset garbage collector
                AssetRef::Failed(_) => None,
                AssetRef::Pending => None,
                AssetRef::Ready(asset) => Some(f(asset)),
            }
        })
        .flatten()
    }

    pub fn load<A: Asset + Send + 'static>(&self, info: A::LoadInfo) -> Handle<A> {
        let bus = self.bus.clone();
        let promise = Promise::spawn_blocking(|| A::load(info, bus));
        let mut lock = self.inner.write().unwrap();
        lock.with_mut_container::<A, _, _>(|mut container| {
            container.items.insert(AssetEntry::Pending(promise))
        })
    }
}
