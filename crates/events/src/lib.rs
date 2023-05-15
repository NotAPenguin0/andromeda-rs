use input::MousePosition;
use scheduler::Event;

pub struct Tick;

impl Event for Tick {}

/// Primary button click on the world view
#[derive(Debug, Copy, Clone)]
pub struct ClickWorldView {
    /// Screen space position of the mouse click
    pub position: MousePosition,
}

impl Event for ClickWorldView {}
