use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

use anyhow::Result;
use dyn_inject::Registry;
use log::error;
use inject::DI;
use poll_promise::Promise;
use scheduler::EventBus;
use slotmap::HopSlotMap;

use crate::asset::Asset;
use crate::handle::Handle;

enum AssetEntry<A: Send + 'static> {
    Pending(Option<Promise<Result<A>>>),
    Failed(anyhow::Error),
    Ready(A),
}

pub enum AssetRef<'a, A> {
    Failed(&'a anyhow::Error),
    Pending,
    Ready(&'a A),
}

struct AssetContainer<A: Send + 'static> {
    items: HopSlotMap<Handle<A>, AssetEntry<A>>,
}

impl<A: Send + 'static> Default for AssetContainer<A> {
    fn default() -> Self {
        Self {
            items: HopSlotMap::default(),
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
    fn with_new_container<'a, A, F, R>(&'a mut self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockReadGuard<'a, AssetContainer<A>>) -> R, {
        self.containers
            .put_sync::<AssetContainer<A>>(AssetContainer::new());
        let container = self.containers.read_sync::<AssetContainer<A>>().unwrap();
        f(container)
    }

    fn with_new_mut_container<'a, A, F, R>(&'a mut self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockWriteGuard<'a, AssetContainer<A>>) -> R, {
        self.containers
            .put_sync::<AssetContainer<A>>(AssetContainer::new());
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

    fn resolve_promises<A: Send + 'static>(container: &mut RwLockWriteGuard<AssetContainer<A>>) {
        container.items.values_mut().for_each(|asset| {
            if let AssetEntry::Pending(promise) = asset {
                if promise.as_ref().unwrap().ready().is_some() {
                    let promise = promise.take().unwrap();
                    let result = promise.block_and_take();
                    *asset = match result {
                        Err(error) => AssetEntry::Failed(error),
                        Ok(value) => AssetEntry::Ready(value),
                    };
                }
            }
        });
    }

    fn report_failure(error: &anyhow::Error) {
        error!("Error loading asset: {error}");
    }

    fn report_and_remove_failed<A: Send + 'static>(container: &mut RwLockWriteGuard<AssetContainer<A>>) {
        container.items.retain(|_, asset| {
            match asset {
                AssetEntry::Pending(_) => { true }
                AssetEntry::Failed(error) => {
                    Self::report_failure(error);
                    false
                }
                AssetEntry::Ready(_) => { true }
            }
        });
    }

    fn garbage_collect<A: Send + 'static>(&self) {
        self.with_mut_container::<A, _, _>(|mut container| {
            Self::resolve_promises(&mut container);
            Self::report_and_remove_failed(&mut container);
        });
    }

    async fn asset_gc_task<A: Send + 'static>(bus: EventBus<DI>) {
        const RESOLVE_INTERVAL_MS: u64 = 500;
        loop {
            tokio::time::sleep(Duration::from_secs(RESOLVE_INTERVAL_MS)).await;
            let inject = bus.data().read().unwrap();
            let assets = inject.get::<AssetStorage>().unwrap();
            assets.garbage_collect::<A>();
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
                // The container does not exist yet, so we create a new container and call our callback.
                // We also need to spawn our garbage collector and promise resolve threads.
                tokio::spawn(Self::asset_gc_task::<A>(self.bus.clone()));
                lock.with_new_container(f)
            }
            Some(container) => f(container),
        }
    }

    fn with_mut_container<A, F, R>(&self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockWriteGuard<AssetContainer<A>>) -> R, {
        let lock = self.inner.read().unwrap();
        let maybe_container = lock.containers.write_sync::<AssetContainer<A>>();
        match maybe_container {
            None => {
                drop(maybe_container);
                drop(lock);
                let mut lock = self.inner.write().unwrap();
                // The container does not exist yet, so we create a new container and call our callback.
                // We also need to spawn our garbage collector and promise resolve threads.
                tokio::spawn(Self::asset_gc_task::<A>(self.bus.clone()));
                lock.with_new_mut_container(f)
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
        self.with_mut_container::<A, _, _>(|mut container| {
            container.items.insert(AssetEntry::Pending(Some(promise)))
        })
    }
}
