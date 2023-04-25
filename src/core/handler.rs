use anyhow::Result;

use crate::core::event::{Event, EventContext};

/// Event handlers must implement this trait. It is implemented for
/// `Fn(&mut S, &E, &mut EventContext)` already.
pub trait Handler<S, E: Event + 'static> {
    fn handle(&self, system: &mut S, event: &E, context: &mut EventContext) -> Result<()>;
}

impl<S, E: Event + 'static, F: Fn(&mut S, &E, &mut EventContext) -> Result<()>> Handler<S, E>
    for F
{
    fn handle(&self, system: &mut S, event: &E, context: &mut EventContext) -> Result<()> {
        self(system, event, context)
    }
}
