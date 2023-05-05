use std::cell::{Cell, Ref, RefCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use anyhow::{anyhow, Result};
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

pub enum AssetRef<'a, A: Send + 'static> {
    Invalid,
    Failed(anyhow::Error),
    Pending,
    Ready(&'a A),
}

pub struct AssetReadLock<'a, A: Send + 'static> {
    lock: RwLockReadGuard<'a, AssetContainer<A>>,
    handle: Handle<A>,
    asset: RefCell<Option<AssetRef<'a, A>>>,
}

impl<'a, A: Send + 'static> AssetReadLock<'a, A> {
    fn poll_entry(entry: &AssetEntry<A>) -> AssetRef<A> {
        match entry {
            AssetEntry::Pending(_) => AssetRef::Pending,
            AssetEntry::Failed(err) => AssetRef::Failed(anyhow!("{err}")),
            AssetEntry::Ready(asset) => AssetRef::Ready(asset),
        }
    }

    fn lookup(&self) {
        // If we have not looked this up yet, perform a lookup and
        // exchange the value inside
        if self.asset.borrow().is_none() {
            let entry = self.lock.items.get(self.handle);
            let asset_ref = match entry {
                None => AssetRef::Invalid,
                Some(entry) => Self::poll_entry(entry),
            };
            self.asset.replace(Some(asset_ref));
        }
    }

    pub fn new(lock: RwLockReadGuard<'a, AssetContainer<A>>, handle: Handle<A>) -> Self {
        Self {
            lock,
            handle,
            asset: RefCell::new(None),
        }
    }
}

impl<'a, A: Send + 'static> Deref for AssetReadLock<'a, A> {
    type Target = AssetRef<'a, A>;

    fn deref(&self) -> &Self::Target {
        self.lookup();
        self.asset.borrow().as_ref().unwrap()
    }
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

pub struct AssetStorage {
    inner: RwLock<AssetStorageInner>,
    bus: EventBus<DI>,
}

impl AssetStorage {
    fn get_or_create_container<A: Send + 'static>(&self) -> RwLockReadGuard<AssetContainer<A>> {
        let lock = self.inner.read().unwrap();
        if lock.containers.read_sync::<AssetContainer<A>>().is_some() {
            lock.containers.read_sync::<AssetContainer<A>>().unwrap()
        } else {
            drop(lock);
            let mut lock = self.inner.write().unwrap();
            lock.containers.put_sync(AssetContainer::<A>::new());
            lock.containers
                .get::<SyncAssetContainer<A>>()
                .cloned()
                .unwrap()
        }
    }

    pub fn new(bus: EventBus<DI>) -> Self {
        Self {
            inner: RwLock::new(AssetStorageInner::default()),
            bus,
        }
    }

    pub fn get<A: Asset + Send + 'static>(&self, handle: Handle<A>) -> AssetReadLock<A> {
        let container = self.get_or_create_container::<A>();
        AssetReadLock::new(container.read().unwrap(), handle)
    }

    pub fn load<A: Asset + Send + 'static>(&self, info: A::LoadInfo) -> Handle<A> {
        let bus = self.bus.clone();
        let promise = Promise::spawn_blocking(|| A::load(info, bus));
        let container = self.get_or_create_container::<A>();
        let mut container = container.write().unwrap();
        let key = container.items.insert(AssetEntry::Pending(promise));
        key
    }
}
