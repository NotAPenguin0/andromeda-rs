use anyhow::Result;

use crate::core::event::{Event, EventContext};

pub(crate) trait Caller<E: Event + 'static> {
    fn call(&mut self, event: &E, context: &mut EventContext) -> Result<()>;
}
