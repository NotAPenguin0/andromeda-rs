use anyhow::Result;
use inject::{ErasedStorage, DI};
use log::error;
use scheduler::EventBus;
use slotmap::HopSlotMap;
use tokio::task::JoinHandle;
use util::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::asset::Asset;
use crate::handle::Handle;

/// Either a reference to an asset, or a marker indicating that the asset is still loading.
pub enum AssetRef<'a, A> {
    Pending,
    Failed(&'a anyhow::Error),
    Ready(&'a A),
}

/// A message sent from the asset loading thread when it finishes.
#[derive(Debug, Copy, Clone)]
enum AssetLoadMessage {
    Success,
    Fail,
}

type AssetMessageSender = tokio::sync::broadcast::Sender<AssetLoadMessage>;
type AssetMessageReceiver = tokio::sync::broadcast::Receiver<AssetLoadMessage>;

// An entry in the asset storage
enum AssetEntry<A: Send + 'static> {
    Pending(JoinHandle<()>, AssetMessageReceiver),
    Failed(anyhow::Error),
    Ready(A),
}

/// Holds all assets and exposes utilities to load them asynchronously
pub struct AssetStorage {
    inner: RwLock<AssetStorageInner>,
    bus: EventBus<DI>,
}

/// Can be used to wait on an asset, or check its status.
enum PollResult {
    Pending(AssetMessageReceiver),
    Failed,
    Ready,
}

/// Stores all assets of a given type.
struct AssetContainer<A: Send + 'static> {
    items: HopSlotMap<Handle<A>, AssetEntry<A>>,
}

#[derive(Default)]
struct AssetStorageInner {
    containers: ErasedStorage,
}

impl<A: Send + 'static> AssetEntry<A> {
    /// Obtain an AssetRef from this entry by stripping away access to the contained
    /// Promise object.
    pub fn as_ref(&self) -> AssetRef<A> {
        match self {
            AssetEntry::Pending(_, _) => AssetRef::Pending,
            AssetEntry::Failed(err) => AssetRef::Failed(err),
            AssetEntry::Ready(asset) => AssetRef::Ready(asset),
        }
    }

    pub fn success(&self) -> bool {
        matches!(self, AssetEntry::Ready(_))
    }
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

impl AssetStorage {
    // Simple logging for now, we can add an event for this later and let systems subscribe to it.
    fn report_failure(error: &anyhow::Error) {
        error!("Error loading asset: {error}");
    }

    /// Acquire a read lock to the asset container and call the given callback with this lock.
    /// Potentially expensive on the first call, since it must create a new container and spawn a new GC thread
    /// for this asset type.
    fn with_container<A, R, F>(&self, f: F) -> R
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
                lock.with_new_container(f)
            }
            // If the container already existed, we now have a read lock to it and we can call the provided callback.
            Some(container) => f(container),
        }
    }

    /// Acquire a write lock to the asset container and call the given callback with this lock.
    /// Potentially expensive on the first call, since it must create a new container and spawn a new GC thread
    /// for this asset type.
    fn with_mut_container<A, R, F>(&self, f: F) -> R
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
                lock.with_new_mut_container(f)
            }
            // If the container already existed, we now have a write lock to it and we can call the provided callback.
            Some(container) => f(container),
        }
    }

    fn resolve_asset_load<A: Asset + Send + 'static>(
        &self,
        key: Handle<A>,
        result: Result<A>,
        sender: AssetMessageSender,
    ) {
        self.with_mut_container(|mut container| {
            // We can unwrap because insert_with_key returns first.
            // We guarantee this, because this `with_mut_container` blocks until
            // the calling `load()` returns.
            let entry = container.items.get_mut(key).unwrap();
            *entry = match result {
                Ok(value) => {
                    // We can send this message before updating the stored asset, because we are in a lock.
                    // The task waiting for the sender will have to wait until this lock is released anyway,
                    // so it can't race with the insertion below.
                    sender.send(AssetLoadMessage::Success).unwrap();
                    AssetEntry::Ready(value)
                }
                Err(err) => {
                    Self::report_failure(&err);
                    sender.send(AssetLoadMessage::Fail).unwrap();
                    AssetEntry::Failed(err)
                }
            };
        });
    }

    fn asset_load_task<A: Asset + Send + 'static>(
        key: Handle<A>,
        info: A::LoadInfo,
        bus: EventBus<DI>,
        sender: AssetMessageSender,
    ) {
        // First load the asset so we don't hold the DI lock for long
        let result = A::load(info, bus.clone());
        let di = bus.data().read().unwrap();
        let assets = di.get::<AssetStorage>().unwrap();
        // Put the loaded asset in the storage and send a message to all threads waiting on it.
        assets.resolve_asset_load(key, result, sender);
    }

    fn insert_with_key<A: Asset + Send + 'static>(
        key: Handle<A>,
        info: A::LoadInfo,
        bus: EventBus<DI>,
    ) -> AssetEntry<A> {
        // This channel will be used by with_when_ready to wait for the asset to be loaded.
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        // Spawn a background task for loading the asset. We keep the JoinHandle so we can allow canceling the task instead of detaching it.
        let task = tokio::task::spawn_blocking(move || Self::asset_load_task(key, info, bus, tx));
        // The receiver is stored in the asset entry so we can wait on it.
        AssetEntry::Pending(task, rx)
    }

    /// Check the status of an asset and obtain an awaitable receiver that can be used to
    /// wait for the asset's status.
    fn poll_asset<A: Send + 'static>(&self, handle: Handle<A>) -> PollResult {
        self.with_container(|container| {
            let entry = container.items.get(handle);
            match entry {
                None => PollResult::Failed,
                Some(entry) => match entry {
                    // Resubscribe to the broadcast channel
                    // * If the message was already received, then acquiring our current lock will block
                    //   until the asset is inserted, and it will be in the Ready state
                    // * If the message was not yet received, this is guaranteed to receive it at some point in the
                    //   future
                    AssetEntry::Pending(_, rx) => PollResult::Pending(rx.resubscribe()),
                    AssetEntry::Failed(_) => PollResult::Failed,
                    AssetEntry::Ready(_) => PollResult::Ready,
                },
            }
        })
    }
}

impl AssetStorage {
    /// Create a new instance of the asset manager and register it inside the DI system
    pub fn new_in_inject(bus: EventBus<DI>) {
        let this = Self {
            inner: RwLock::with_name(AssetStorageInner::default(), "AssetStorage"),
            bus: bus.clone(),
        };
        // Synchronization is handled internally already, so we do not use
        // `put_sync`.
        bus.data().write().unwrap().put(this);
    }

    /// Calls the provided callback function with the asset corresponding to the given handle and returns its result.
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
    /// Does not call the function if the asset was not found, if it failed, or if it is not ready yet, and
    /// returns None instead.
    pub fn with_if_ready<A, R, F>(&self, handle: Handle<A>, f: F) -> Option<R>
    where
        A: Asset + Send + 'static,
        F: FnOnce(&A) -> R, {
        // We can easily implement this in terms of `Self::with()`
        self.with(handle, |asset| match asset {
            AssetRef::Pending => None,
            AssetRef::Failed(_) => None,
            AssetRef::Ready(asset) => Some(f(asset)),
        })
        // Flatten the Option<Option<R>> into an Option<R>
        .flatten()
    }

    /// Calls the provided callback with the given asset, blocking the calling thread until it is ready.
    /// * If the asset does not exist, this does not block and instead returns None
    /// * If the asset failed to load previously, this does not block and instead returns None.
    pub fn with_when_ready<A, R, F>(&self, handle: Handle<A>, f: F) -> Option<R>
    where
        A: Asset + Send + 'static,
        F: FnOnce(&A) -> R, {
        // Poll the asset system once for the current status of the asset.
        // * If the asset doesn't exist, don't call `f`
        // * If the asset load completed, call `f` if it was successful
        // * If the asset load is still pending, we get a broadcast channel that we can wait on
        let poll = self.poll_asset(handle);
        match poll {
            // Wait for the broadcast message and call `f` if we receive a `Success` message.
            PollResult::Pending(mut rx) => {
                let message = tokio::runtime::Handle::current().block_on(rx.recv()).ok()?;
                match message {
                    AssetLoadMessage::Success => self.with_if_ready(handle, f),
                    AssetLoadMessage::Fail => None,
                }
            }
            PollResult::Failed => None,
            PollResult::Ready => {
                // We know the asset is ready, so this will always call `f`.
                self.with_if_ready(handle, f)
            }
        }
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
        // Acquire a writer lock to the container, since we need to insert a new key
        self.with_mut_container(|mut container| {
            container
                .items
                .insert_with_key(|key| Self::insert_with_key(key, info, self.bus.clone()))
        })
    }

    /// Frees up memory used by asset entries that failed to load.
    pub fn clear_failed_assets<A: Send + 'static>(&self) {
        self.with_mut_container::<A, _, _>(|mut container| {
            // Remove all entries that failed from the container.
            container
                .items
                .retain(|_, entry| !matches!(entry, AssetEntry::Failed(_)));
        });
    }

    /// Immediately delete an asset.
    /// # Safety
    /// This is marked unsafe because the asset could still be in use on the GPU when this is called.
    pub unsafe fn delete_asset<A: Send + 'static>(&self, handle: Handle<A>) {
        self.with_mut_container(|mut container| {
            container.items.remove(handle);
        });
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
