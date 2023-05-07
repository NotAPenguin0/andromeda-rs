use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

use anyhow::Result;
use dyn_inject::Registry;
use inject::DI;
use log::error;
use poll_promise::Promise;
use scheduler::EventBus;
use slotmap::HopSlotMap;

use crate::asset::Asset;
use crate::handle::Handle;

enum AssetEntry<A: Send + 'static> {
    Pending(Option<Promise<Result<A>>>),
    Ready(A),
}

impl<A: Send + 'static> AssetEntry<A> {
    pub fn as_ref(&self) -> AssetRef<A> {
        match self {
            AssetEntry::Pending(_) => AssetRef::Pending,
            AssetEntry::Ready(asset) => AssetRef::Ready(asset),
        }
    }
}

pub enum AssetRef<'a, A> {
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
    fn report_failure(error: &anyhow::Error) {
        error!("Error loading asset: {error}");
    }

    fn take_promise_result<A: Send + 'static>(promise: Promise<Result<A>>) -> Result<AssetEntry<A>> {
        let result = promise.block_and_take();
        result.map(|asset| AssetEntry::Ready(asset))
    }

    fn poll_and_report<A: Send + 'static>(asset: &mut AssetEntry<A>) -> bool {
        // Only need to poll if this entry is a promise
        let AssetEntry::Pending(promise) = asset else { return true; };
        // Keep all pending promises
        if promise.as_ref().unwrap().poll().is_pending() { return true; }
        let result = Self::take_promise_result(promise.take().unwrap());
        match result {
            Err(error) => {
                Self::report_failure(&error);
                // Remove asset if completed with failure
                false
            }
            Ok(value) => {
                *asset = value;
                // Retain asset if completed successfully
                true
            }
        }
    }

    fn garbage_collect<A: Send + 'static>(&self) {
        self.with_mut_container::<A, _, _>(|mut container| {
            container
                .items
                .retain(|_, asset| Self::poll_and_report(asset));
        });
    }

    async fn asset_gc_task<A: Send + 'static>(bus: EventBus<DI>) {
        const GC_PERIOD: u64 = 250;
        loop {
            tokio::time::sleep(Duration::from_millis(GC_PERIOD)).await;
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

    /// Create a new instance of the asset manager and register it inside the DI system
    pub fn new_in_inject(bus: EventBus<DI>) {
        let this = Self {
            inner: RwLock::new(AssetStorageInner::default()),
            bus: bus.clone(),
        };
        bus.data().write().unwrap().put(this);
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
            entry.map(|entry| f(entry.as_ref()))
        })
    }

    pub fn with_if_ready<A, R, F>(&self, handle: Handle<A>, f: F) -> Option<R>
    where
        A: Asset + Send + 'static,
        F: FnOnce(&A) -> R, {
        self.with(handle, |asset| match asset {
            AssetRef::Pending => None,
            AssetRef::Ready(asset) => Some(f(asset)),
        })
        .flatten()
    }

    /// Check if an asset is ready or still pending
    /// # Returns
    /// * `true` if the asset is currently ready
    /// * `false` if the asset is still pending
    /// * `false` if the asset failed to load.
    pub fn is_ready<A: Asset + Send + 'static>(&self, handle: Handle<A>) -> bool {
        // Since `with_if_ready` only calls the closure if the asset is Ready with a non-failure status,
        // we can simply check if the closure was called using `is_some()`.
        self.with_if_ready(handle, |_| {}).is_some()
    }

    pub fn load<A: Asset + Send + 'static>(&self, info: A::LoadInfo) -> Handle<A> {
        let bus = self.bus.clone();
        let promise = Promise::spawn_blocking(|| A::load(info, bus));
        self.with_mut_container::<A, _, _>(|mut container| {
            container.items.insert(AssetEntry::Pending(Some(promise)))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use anyhow::bail;
    use log::info;
    use tokio::time::sleep;
    use inject::DI;
    use scheduler::EventBus;

    use crate::asset::Asset;
    use crate::storage::AssetStorage;

    struct MyAsset {
        data: String,
    }

    impl Asset for MyAsset {
        type LoadInfo = String;

        fn load(info: Self::LoadInfo, bus: EventBus<DI>) -> anyhow::Result<Self> {
            info!("Hi");
            if info == "fail" {
                bail!("invalid load info");
            } else {
                Ok(MyAsset {
                    data: info,
                })
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_load_success() {
        let inject = DI::new();
        let bus = EventBus::new(inject.clone());
        AssetStorage::new_in_inject(bus);
        let di = inject.read().unwrap();
        let assets = di.get::<AssetStorage>().unwrap();
        let handle = assets.load::<MyAsset>("success".to_owned());
        // Wait for load to be completed
        sleep(Duration::from_secs(1)).await;
        // Should be successful now
        assert!(assets.is_ready(handle));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_load_fail() {
        let inject = DI::new();
        let bus = EventBus::new(inject.clone());
        AssetStorage::new_in_inject(bus);
        let di = inject.read().unwrap();
        let assets = di.get::<AssetStorage>().unwrap();
        let handle = assets.load::<MyAsset>("fail".to_owned());
        // Wait for load to be completed
        sleep(Duration::from_secs(1)).await;
        // Should have failed by now
        assert!(assets.with_if_ready(handle, |_| {}).is_none());
    }
}
