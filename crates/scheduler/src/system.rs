use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use dyn_inject::Registry;

use crate::bus::EventBus;
use crate::caller::Caller;
use crate::event::{Event, EventContext};
use crate::handler::Handler;

/// A system must implement this to subscribe to events on the bus
pub trait System<T> {
    fn initialize(event_bus: &mut EventBus<T>, system: &StoredSystem<Self>)
    where
        Self: Sized;
}

struct StoredSystemInner<S> {
    state: S,
    handlers: Registry,
}

impl<S: 'static> StoredSystemInner<S> {
    fn handle<E: Event + 'static, T: 'static>(
        &mut self,
        event: &E,
        context: &mut EventContext<T>,
    ) -> Result<()> {
        let handler = self
            .handlers
            .get_dyn::<dyn Handler<S, E, T>>()
            .ok_or(anyhow!("No handler for this event"))?;
        handler.handle(&mut self.state, event, context)
    }
}

/// A system stored in the event bus. It is created for you when adding a system.
pub struct StoredSystem<S>(Arc<Mutex<StoredSystemInner<S>>>);

impl<S> Clone for StoredSystem<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S: 'static> StoredSystem<S> {
    pub(crate) fn new(state: S) -> Self {
        Self(Arc::new(Mutex::new(StoredSystemInner {
            state,
            handlers: Registry::new(),
        })))
    }

    pub(crate) fn subscribe<E: Event + 'static, T: 'static>(
        &mut self,
        handler: impl Handler<S, E, T> + 'static,
    ) {
        self.0
            .lock()
            .unwrap()
            .handlers
            .put_dyn::<dyn Handler<S, E, T>>(handler);
    }
}

impl<S: 'static, E: Event + 'static, T: 'static> Caller<E, T> for StoredSystem<S> {
    fn call(&mut self, event: &E, context: &mut EventContext<T>) -> Result<()> {
        self.0.lock().unwrap().handle(event, context)
    }
}
