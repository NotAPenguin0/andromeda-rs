use anyhow::Result;

use crate::core::event::Event;

/// Event handlers must implement this trait. It is implemented for
/// `Fn(&mut S, &E)` already.
pub trait Handler<S, E: Event + 'static> {
    fn handle(&self, system: &mut S, event: &E) -> Result<()>;
}

impl<S, E: Event + 'static, F: Fn(&mut S, &E) -> Result<()>> Handler<S, E> for F {
    fn handle(&self, system: &mut S, event: &E) -> Result<()> {
        self(system, event)
    }
}
