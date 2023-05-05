use anyhow::Result;

use crate::event::{Event, EventContext};

pub trait Caller<E: Event + 'static, T> {
    fn call(&mut self, event: &E, context: &mut EventContext<T>) -> Result<E::Result>;
}
