use anyhow::Result;

use crate::core::bus::EventBus;

/// Trait to signal that this is an event type.
pub trait Event {}

pub struct EventContext {
    bus: EventBus,
}

impl EventContext {
    pub(crate) fn new(bus: EventBus) -> Self {
        Self {
            bus,
        }
    }

    pub fn publish<E: Event + 'static>(&mut self, event: &E) -> Result<()> {
        self.bus.publish(event)
    }
}
