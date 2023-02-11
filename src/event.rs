use tiny_tokio_actor::*;

#[derive(Clone, Debug)]
pub struct Event {}

impl SystemEvent for Event {}