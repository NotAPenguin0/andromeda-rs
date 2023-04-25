use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::Result;
use dyn_inject::Registry;

use crate::core::caller::Caller;
use crate::core::event::{Event, EventContext};
use crate::core::handler::Handler;
use crate::core::system::{StoredSystem, System};

struct TypedEventBus<E: Event> {
    systems: Vec<Box<dyn Caller<E>>>,
}

impl<E: Event + 'static> TypedEventBus<E> {
    pub(crate) fn new() -> Self {
        Self {
            systems: vec![],
        }
    }

    pub(crate) fn register_system<S: 'static>(
        &mut self,
        mut system: StoredSystem<S>,
        handler: impl Handler<S, E> + 'static,
    ) {
        system.subscribe(handler);
        self.systems.push(Box::new(system));
    }

    fn publish(&mut self, event: &E, context: &mut EventContext) -> Result<()> {
        for system in &mut self.systems {
            system.call(&event, context)?;
        }
        Ok(())
    }
}

struct SyncEventBus<E: Event>(Arc<Mutex<TypedEventBus<E>>>);

impl<E: Event> Clone for SyncEventBus<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<E: Event + 'static> SyncEventBus<E> {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(TypedEventBus::new())))
    }
}

impl<E: Event> Deref for SyncEventBus<E> {
    type Target = Arc<Mutex<TypedEventBus<E>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct EventBusInner {
    buses: Registry,
}

/// The main event bus, stores systems and their handlers for each event.
#[derive(Clone)]
pub struct EventBus(Arc<RwLock<EventBusInner>>);

impl EventBus {
    fn get_or_create_bus<'a, E: Event + 'static>(&mut self) -> SyncEventBus<E> {
        let lock = self.0.read().unwrap();
        if lock.buses.get::<SyncEventBus<E>>().is_some() {
            lock.buses.get::<SyncEventBus<E>>().cloned().unwrap()
        } else {
            drop(lock);
            let mut lock = self.0.write().unwrap();
            lock.buses.put(SyncEventBus::<E>::new());
            lock.buses.get::<SyncEventBus<E>>().cloned().unwrap()
        }
    }

    /// Create a new event bus
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(EventBusInner {
            buses: Registry::new(),
        })))
    }

    /// Add a system to the event bus. Calls the system's initialize function to register
    /// handler callbacks
    pub fn add_system<S: System + 'static>(&mut self, system: S) {
        let stored = StoredSystem::new(system);
        S::initialize(self, &stored);
    }

    /// Subscribe to an event on the bus
    pub fn subscribe<S: 'static, E: Event + 'static>(
        &mut self,
        system: &StoredSystem<S>,
        handler: impl Handler<S, E> + 'static,
    ) {
        let bus = self.get_or_create_bus::<E>();
        bus.lock().unwrap().register_system(system.clone(), handler);
    }

    /// Publish an event to the bus
    pub fn publish<E: Event + 'static>(&mut self, event: &E) -> Result<()> {
        // Note: We only lock the entire bus for a short time to get access to the registry.
        // After that we only lock the individual event bus. This will cause the program to deadlock when recursively
        // triggering events, which is not something that is supported anyway.
        let bus = self.get_or_create_bus::<E>();
        let mut context = EventContext::new(self.clone());
        let mut lock = bus.lock().unwrap();
        lock.publish(event, &mut context)
    }
}
