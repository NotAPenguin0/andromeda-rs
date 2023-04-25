use std::ops::{Deref, DerefMut};

use anyhow::Result;

use crate::bus::EventBus;

/// Trait to signal that this is an event type.
pub trait Event {}

pub struct EventContext<T> {
    bus: EventBus<T>,
}

impl<T: Clone + Send + Sync + 'static> EventContext<T> {
    pub(crate) fn new(bus: EventBus<T>) -> Self {
        Self {
            bus,
        }
    }

    pub fn publish<E: Event + 'static>(&mut self, event: &E) -> Result<()> {
        self.bus.publish(event)
    }
}

impl<T: Clone + Send + Sync + 'static> Deref for EventContext<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.bus.data()
    }
}

impl<T: Clone + Send + Sync + 'static> DerefMut for EventContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.bus.data_mut()
    }
}
