use std::collections::HashMap;
use std::fmt::Debug;

use anyhow::Result;
use inject::DI;
use scheduler::{Event, EventBus, EventContext, StoredSystem, System};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Key {
    Shift,
    Escape,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct MousePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct MouseDelta {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct MouseButtonState {
    pub state: ButtonState,
    pub button: MouseButton,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyState {
    pub state: ButtonState,
    pub button: Key,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollInfo {
    pub delta_x: f32,
    pub delta_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    MousePosition(MousePosition),
    MouseMove(MouseDelta),
    MouseButton(MouseButtonState),
    Button(KeyState),
    Scroll(ScrollInfo),
}

impl Event for InputEvent {}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(value: winit::event::MouseButton) -> Self {
        match value {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Other(x) => MouseButton::Other(x),
        }
    }
}

impl From<winit::event::ElementState> for ButtonState {
    fn from(value: winit::event::ElementState) -> Self {
        match value {
            winit::event::ElementState::Pressed => ButtonState::Pressed,
            winit::event::ElementState::Released => ButtonState::Released,
        }
    }
}

#[derive(Debug, Default)]
pub struct InputState {
    mouse: MousePosition,
    mouse_buttons: HashMap<MouseButton, ButtonState>,
    kb_buttons: HashMap<Key, ButtonState>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            mouse: Default::default(),
            mouse_buttons: Default::default(),
            kb_buttons: Default::default(),
        }
    }

    pub fn get_key(&self, key: Key) -> ButtonState {
        self.kb_buttons
            .get(&key)
            .copied()
            .unwrap_or(ButtonState::Released)
    }

    pub fn get_mouse_key(&self, key: MouseButton) -> ButtonState {
        self.mouse_buttons
            .get(&key)
            .copied()
            .unwrap_or(ButtonState::Released)
    }

    pub fn mouse(&self) -> MousePosition {
        self.mouse
    }
}

struct Input;

impl Input {
    pub fn process_event(&self, input_state: &mut InputState, event: &InputEvent) {
        match event {
            InputEvent::MousePosition(pos) => {
                input_state.mouse = *pos;
            }
            InputEvent::MouseButton(state) => {
                input_state.mouse_buttons.insert(state.button, state.state);
            }
            InputEvent::Button(state) => {
                input_state.kb_buttons.insert(state.button, state.state);
            }
            InputEvent::Scroll(_) => {}
            InputEvent::MouseMove(_) => {}
        };
    }
}

impl System<DI> for Input {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>)
    where
        Self: Sized, {
        event_bus.subscribe(system, handle_input_event);
    }
}

fn handle_input_event(
    system: &mut Input,
    event: &InputEvent,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    let inject = ctx.read().unwrap();
    let mut state = inject.write_sync::<InputState>().unwrap();
    system.process_event(&mut state, event);
    Ok(())
}

/// Initialize the input system
pub fn initialize(bus: &mut EventBus<DI>) {
    bus.add_system(Input);
    let state = InputState::new();
    let mut di = bus.data().write().unwrap();
    di.put_sync(state);
}
