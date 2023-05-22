use anyhow::Result;

use crate::event::{Event, EventContext};

pub trait Caller<E: Event + 'static, T> {
    fn call(&self, event: &E, context: &mut EventContext<T>) -> Result<E::Result>;
}

pub trait SinkCaller<E: Event + 'static, T> {
    fn call(&self, event: E, context: &mut EventContext<T>) -> Result<E::Result>;
}
