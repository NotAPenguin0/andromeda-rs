use anyhow::Result;

use crate::event::{Event, EventContext};

/// Event handlers must implement this trait. It is implemented for
/// `Fn(&mut S, &E, &mut EventContext)` already.
pub trait Handler<S, E: Event, T: 'static> {
    fn handle(&self, system: &mut S, event: &E, context: &mut EventContext<T>)
        -> Result<E::Result>;
}

impl<
        S,
        E: Event + 'static,
        T: 'static,
        F: Fn(&mut S, &E, &mut EventContext<T>) -> Result<E::Result>,
    > Handler<S, E, T> for F
{
    fn handle(
        &self,
        system: &mut S,
        event: &E,
        context: &mut EventContext<T>,
    ) -> Result<E::Result> {
        self(system, event, context)
    }
}
