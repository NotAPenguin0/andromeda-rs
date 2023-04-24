use anyhow::Result;

use crate::core::event::Event;

pub(crate) trait Caller<E: Event + 'static> {
    fn call(&mut self, event: &E) -> Result<()>;
}
