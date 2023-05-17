use input::MousePosition;
use scheduler::Event;

pub struct Tick;

impl Event for Tick {}

/// Primary button click on the world view
#[derive(Debug, Copy, Clone)]
pub struct DragWorldView {
    /// Current screen space position of the mouse
    pub position: MousePosition,
}

impl Event for DragWorldView {}
