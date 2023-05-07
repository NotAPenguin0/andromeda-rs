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

/// Either a reference to an asset, or a marker indicating that the asset is still loading.
pub enum AssetRef<'a, A> {
    Pending,
    Ready(&'a A),
}

// An entry in the asset storage
enum AssetEntry<A: Send + 'static> {
    Pending(Option<Promise<Result<A>>>),
    Ready(A),
}

impl<A: Send + 'static> AssetEntry<A> {
    /// Obtain an AssetRef from this entry by stripping away access to the contained
    /// Promise object.
    pub fn as_ref(&self) -> AssetRef<A> {
        match self {
            AssetEntry::Pending(_) => AssetRef::Pending,
            AssetEntry::Ready(asset) => AssetRef::Ready(asset),
        }
    }
}

/// Stores all assets of a given type.
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
    /// Create a new container for a given asset type and acquire a reader lock to it.
    /// Calls the given function with this reader lock.
    fn with_new_container<A, F, R>(&mut self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockReadGuard<AssetContainer<A>>) -> R, {
        // Create a new container and put it inside the registry
        self.containers
            .put_sync::<AssetContainer<A>>(AssetContainer::new());
        // Acquire a reader lock and pass it to the callback
        let container = self.containers.read_sync::<AssetContainer<A>>().unwrap();
        f(container)
    }

    /// Create a new container for a given asset type and acquire a writer lock to it.
    /// Calls the given function with this writer lock.
    fn with_new_mut_container<A, F, R>(&mut self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockWriteGuard<AssetContainer<A>>) -> R, {
        self.containers
            .put_sync::<AssetContainer<A>>(AssetContainer::new());
        let container = self.containers.write_sync::<AssetContainer<A>>().unwrap();
        f(container)
    }
}

/// Holds all assets and exposes utilities to load them asynchronously
pub struct AssetStorage {
    inner: RwLock<AssetStorageInner>,
    bus: EventBus<DI>,
}

impl AssetStorage {
    // Simple logging for now, we can add an event for this later and let systems subscribe to it.
    fn report_failure(error: &anyhow::Error) {
        error!("Error loading asset: {error}");
    }

    /// Blocks the calling thread and returns a resulting `AssetEntry` if the load operation succeeded.
    /// Propagates the error otherwise.
    fn take_promise_result<A: Send + 'static>(
        promise: Promise<Result<A>>,
    ) -> Result<AssetEntry<A>> {
        let asset = promise.block_and_take()?;
        Ok(AssetEntry::Ready(asset))
    }

    /// Polls the asset's promise if it is pending. If completed, replace it in the storage if load was successful.
    /// If completed with an eror this returns `false`, otherwise this function always returns `true`.
    fn poll_and_report<A: Send + 'static>(asset: &mut AssetEntry<A>) -> bool {
        // Only need to poll if this entry is a promise
        let AssetEntry::Pending(promise) = asset else { return true; };
        // Keep all pending promises
        if promise.as_ref().unwrap().poll().is_pending() {
            return true;
        }
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

    /// Goes over all assets and polls promises, removing those that completed with an error.
    fn garbage_collect<A: Send + 'static>(&self) {
        self.with_mut_container::<A, _, _>(|mut container| {
            container
                .items
                .retain(|_, asset| Self::poll_and_report(asset));
        });
    }

    /// Periodically cleans up errored asset loads and polls asset promises.
    async fn asset_gc_task<A: Send + 'static>(bus: EventBus<DI>) {
        const GC_PERIOD: u64 = 250;
        loop {
            tokio::time::sleep(Duration::from_millis(GC_PERIOD)).await;
            let inject = bus.data().read().unwrap();
            let assets = inject.get::<AssetStorage>().unwrap();
            assets.garbage_collect::<A>();
        }
    }

    /// Acquire a read lock to the asset container and call the given callback with this lock.
    /// Potentially expensive on the first call, since it must create a new container and spawn a new GC thread
    /// for this asset type.
    fn with_container<A, F, R>(&self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockReadGuard<AssetContainer<A>>) -> R, {
        // First acquire a read lock to the inner state so we can check if the container exists already
        let lock = self.inner.read().unwrap();
        // Acquire a read lock to the container if it exists.
        let maybe_container = lock.containers.read_sync::<AssetContainer<A>>();
        match maybe_container {
            None => {
                // We need a writer lock to the inner state so we can insert the new container.
                // To be able to get this lock we need to explicitly drop the reader lock.
                // Since `maybe_container` borrows from this lock, we also need to explicitly drop it.
                drop(maybe_container);
                drop(lock);
                let mut lock = self.inner.write().unwrap();
                // The container does not exist yet, so we create a new container and call our callback.
                // We also need to spawn our garbage collector and promise resolve threads.
                tokio::spawn(Self::asset_gc_task::<A>(self.bus.clone()));
                lock.with_new_container(f)
            }
            // If the container already existed, we now have a read lock to it and we can call the provided callback.
            Some(container) => f(container),
        }
    }

    /// Acquire a write lock to the asset container and call the given callback with this lock.
    /// Potentially expensive on the first call, since it must create a new container and spawn a new GC thread
    /// for this asset type.
    fn with_mut_container<A, F, R>(&self, f: F) -> R
    where
        A: Send + 'static,
        F: FnOnce(RwLockWriteGuard<AssetContainer<A>>) -> R, {
        // First acquire a read lock to the inner state so we can check if the container exists already.
        let lock = self.inner.read().unwrap();
        // Acquire a write lock to the container if it exists.
        let maybe_container = lock.containers.write_sync::<AssetContainer<A>>();
        match maybe_container {
            None => {
                // We need a writer lock to the inner state so we can insert the new container.
                // To be able to get this lock we need to explicitly drop the reader lock.
                // Since `maybe_container` borrows from this lock, we also need to explicitly drop it.
                drop(maybe_container);
                drop(lock);
                let mut lock = self.inner.write().unwrap();
                // The container does not exist yet, so we create a new container and call our callback.
                // We also need to spawn our garbage collector and promise resolve threads.
                tokio::spawn(Self::asset_gc_task::<A>(self.bus.clone()));
                lock.with_new_mut_container(f)
            }
            // If the container already existed, we now have a write lock to it and we can call the provided callback.
            Some(container) => f(container),
        }
    }

    /// Create a new instance of the asset manager and register it inside the DI system
    pub fn new_in_inject(bus: EventBus<DI>) {
        let this = Self {
            inner: RwLock::new(AssetStorageInner::default()),
            bus: bus.clone(),
        };
        // Synchronization is handled internally already, so we do not use
        // `put_sync`.
        bus.data().write().unwrap().put(this);
    }

    /// Calls the provided callback function with the asset corresponding to the given handle and returns its
    /// result.
    /// Does not call the function if the asset was not found, and returns None instead.
    pub fn with<A, R, F>(&self, handle: Handle<A>, f: F) -> Option<R>
    where
        A: Asset + Send + 'static,
        F: FnOnce(AssetRef<A>) -> R, {
        // To access an asset we only need read access, so acquire a read lock to
        // the correct container
        self.with_container(|container| {
            // Look up the entry in the container, and call the function on a reference
            // to it if it exists.
            let entry = container.items.get(handle);
            entry.map(|entry| f(entry.as_ref()))
        })
    }

    /// Calls the provided callback function with the asset corresponding to the given handle
    /// and returns its result.
    /// Does not call the function if the asset was not found, or if it is not ready yet, and
    /// returns None instead.
    pub fn with_if_ready<A, R, F>(&self, handle: Handle<A>, f: F) -> Option<R>
    where
        A: Asset + Send + 'static,
        F: FnOnce(&A) -> R, {
        // We can easily implement this in terms of `Self::with()`
        self.with(handle, |asset| match asset {
            AssetRef::Pending => None,
            AssetRef::Ready(asset) => Some(f(asset)),
        })
        // Flatten the Option<Option<R>> into an Option<R>
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

    /// Load a new asset and return a handle to it. This will spawn a new blocking task in a background thread.
    /// This means that this function is not blocking, and returns a handle immediately.
    pub fn load<A: Asset + Send + 'static>(&self, info: A::LoadInfo) -> Handle<A> {
        let bus = self.bus.clone();
        // Spawn a load task
        let promise = Promise::spawn_blocking(|| A::load(info, bus));
        // Get a writer lock to the correct container and insert a pending asset into it.
        self.with_mut_container(|mut container| {
            container.items.insert(AssetEntry::Pending(Some(promise)))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use anyhow::bail;
    use inject::DI;
    use log::info;
    use scheduler::EventBus;
    use tokio::time::sleep;

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
