use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::Result;
use dyn_inject::Registry;

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
        mut system: StoredSystem<S>,
        handler: impl Handler<S, E, T> + 'static,
    ) {
        system.subscribe(handler);
        self.systems.push(Box::new(system));
    }

    fn publish(&mut self, event: &E, context: &mut EventContext<T>) -> Result<()> {
        for system in &mut self.systems {
            system.call(&event, context)?;
        }
        Ok(())
    }
}

struct SyncEventBus<E: Event, T>(Arc<Mutex<TypedEventBus<E, T>>>);

impl<E: Event, T> Clone for SyncEventBus<E, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<E: Event + 'static, T: 'static> SyncEventBus<E, T> {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(TypedEventBus::new())))
    }
}

impl<E: Event, T> Deref for SyncEventBus<E, T> {
    type Target = Arc<Mutex<TypedEventBus<E, T>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
struct EventBusInner {
    buses: Registry,
}

/// The main event bus, stores systems and their handlers for each event.
#[derive(Debug, Clone)]
pub struct EventBus<T> {
    inner: Arc<RwLock<EventBusInner>>,
    data: T,
}

unsafe impl<T: Send> Send for EventBus<T> {}

unsafe impl<T: Sync> Sync for EventBus<T> {}

impl<T: Clone + Send + Sync + 'static> EventBus<T> {
    fn get_or_create_bus<'a, E: Event + 'static>(&self) -> SyncEventBus<E, T> {
        let lock = self.inner.read().unwrap();
        if lock.buses.get::<SyncEventBus<E, T>>().is_some() {
            lock.buses.get::<SyncEventBus<E, T>>().cloned().unwrap()
        } else {
            drop(lock);
            let mut lock = self.inner.write().unwrap();
            lock.buses.put(SyncEventBus::<E, T>::new());
            lock.buses.get::<SyncEventBus<E, T>>().cloned().unwrap()
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
                buses: Registry::new(),
            })),
            data,
        }
    }

    /// Add a system to the event bus. Calls the system's initialize function to register
    /// handler callbacks
    pub fn add_system<S: System<T> + 'static>(&mut self, system: S) {
        let stored = StoredSystem::new(system);
        S::initialize(self, &stored);
    }

    /// Subscribe to an event on the bus
    pub fn subscribe<S: 'static, E: Event + 'static>(
        &mut self,
        system: &StoredSystem<S>,
        handler: impl Handler<S, E, T> + 'static,
    ) {
        let bus = self.get_or_create_bus::<E>();
        bus.lock().unwrap().register_system(system.clone(), handler);
    }

    /// Publish an event to the bus
    pub fn publish<E: Event + 'static>(&self, event: &E) -> Result<()> {
        // Note: We only lock the entire bus for a short time to get access to the registry.
        // After that we only lock the individual event bus. This will cause the program to deadlock when recursively
        // triggering events, which is not something that is supported anyway.
        let bus = self.get_or_create_bus::<E>();
        let mut context = EventContext::new(self.clone());
        let mut lock = bus.lock().unwrap();
        lock.publish(event, &mut context)
    }
}
