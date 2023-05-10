use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use inject::ErasedStorage;
use util::RwLock;

use crate::caller::Caller;
use crate::event::{Event, EventContext};
use crate::handler::Handler;
use crate::system::{StoredSystem, System};

struct TypedEventBus<E: Event, T> {
    systems: Vec<Box<dyn Caller<E, T>>>,
}

impl<E: Event + 'static, T: 'static> TypedEventBus<E, T> {
    pub fn new() -> Self {
        Self {
            systems: vec![],
        }
    }

    pub fn register_system<S: 'static>(
        &mut self,
        system: StoredSystem<S>,
        handler: impl Handler<S, E, T> + 'static,
    ) {
        system.subscribe(handler);
        self.systems.push(Box::new(system));
    }

    fn publish(&self, event: &E, context: &mut EventContext<T>) -> Result<Vec<E::Result>> {
        let mut results = Vec::with_capacity(self.systems.len());
        for system in &self.systems {
            results.push(system.call(event, context)?);
        }
        Ok(results)
    }
}

struct SyncEventBus<E: Event, T>(RwLock<TypedEventBus<E, T>>);

impl<E: Event + 'static, T: 'static> SyncEventBus<E, T> {
    fn new() -> Self {
        Self(RwLock::new(TypedEventBus::new()))
    }
}

impl<E: Event, T> Deref for SyncEventBus<E, T> {
    type Target = RwLock<TypedEventBus<E, T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
struct EventBusInner {
    buses: ErasedStorage,
}

/// The main event bus, stores systems and their handlers for each event.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub struct EventBus<T> {
    inner: Arc<RwLock<EventBusInner>>,
    data: T,
}

unsafe impl<T: Send> Send for EventBus<T> {}

unsafe impl<T: Sync> Sync for EventBus<T> {}

impl<T: Clone + Send + Sync + 'static> EventBus<T> {
    fn with_new_event_bus<E: Event + 'static, R, F: FnOnce(&SyncEventBus<E, T>) -> R>(
        &self,
        f: F,
    ) -> R {
        let mut lock = self.inner.write().unwrap();
        lock.buses.put(SyncEventBus::<E, T>::new());
        let bus = lock.buses.get().unwrap();
        f(bus)
    }

    fn with_event_bus<E: Event + 'static, R, F: FnOnce(&SyncEventBus<E, T>) -> R>(
        &self,
        f: F,
    ) -> R {
        let lock = self.inner.read().unwrap();
        let maybe_bus = lock.buses.get();
        match maybe_bus {
            None => {
                drop(lock);
                self.with_new_event_bus(f)
            }
            Some(bus) => f(bus),
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    /// Create a new event bus
    pub fn new(data: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(EventBusInner {
                buses: ErasedStorage::new(),
            })),
            data,
        }
    }

    /// Add a system to the event bus. Calls the system's initialize function to register
    /// handler callbacks
    pub fn add_system<S: System<T> + 'static>(&self, system: S) {
        let stored = StoredSystem::new(system);
        S::initialize(self, &stored);
    }

    /// Subscribe to an event on the bus
    pub fn subscribe<S: 'static, E: Event + 'static>(
        &self,
        system: &StoredSystem<S>,
        handler: impl Handler<S, E, T> + 'static,
    ) {
        self.with_event_bus(|bus| {
            bus.write()
                .unwrap()
                .register_system(system.clone(), handler);
        });
    }

    /// Publish an event to the bus
    pub fn publish<E: Event + 'static>(&self, event: &E) -> Result<Vec<E::Result>> {
        // Note: We only lock the entire bus for a short time to get access to the registry.
        // After that we only lock the individual event bus. This will cause the program to deadlock when recursively
        // triggering events, which is not something that is supported anyway.
        self.with_event_bus(|bus| {
            let mut context = EventContext::new(self.clone());
            let lock = bus.read().unwrap();
            lock.publish(event, &mut context)
        })
    }
}
