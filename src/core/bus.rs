use anyhow::Result;
use dyn_inject::Registry;

use crate::core::caller::Caller;
use crate::core::event::Event;
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

    fn publish(&mut self, event: &E) -> Result<()> {
        for system in &mut self.systems {
            system.call(&event)?;
        }
        Ok(())
    }
}

/// The main event bus, stores systems and their handlers for each event.
pub struct EventBus {
    buses: Registry,
}

impl EventBus {
    fn get_or_create_bus<E: Event + 'static>(&mut self) -> &mut TypedEventBus<E> {
        if self.buses.get_mut::<TypedEventBus<E>>().is_some() {
            self.buses.get_mut::<TypedEventBus<E>>().unwrap()
        } else {
            self.buses.put(TypedEventBus::<E>::new());
            self.buses.get_mut::<TypedEventBus<E>>().unwrap()
        }
    }

    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            buses: Registry::new(),
        }
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
        bus.register_system(system.clone(), handler);
    }

    /// Publish an event to the bus
    pub fn publish<E: Event + 'static>(&mut self, event: &E) -> Result<()> {
        let bus = self.get_or_create_bus::<E>();
        bus.publish(event)
    }
}
