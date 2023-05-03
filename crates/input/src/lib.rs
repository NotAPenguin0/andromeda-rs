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

#[derive(Debug)]
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
            .cloned()
            .unwrap_or(ButtonState::Released)
    }

    pub fn get_mouse_key(&self, key: MouseButton) -> ButtonState {
        self.mouse_buttons
            .get(&key)
            .cloned()
            .unwrap_or(ButtonState::Released)
    }
}

struct Input;

impl Input {
    pub fn process_event(
        &mut self,
        input_state: &mut InputState,
        event: &InputEvent,
    ) -> Vec<InputEvent> {
        let mut additional_events = Vec::new();
        match event {
            InputEvent::MousePosition(pos) => {
                let evt = InputEvent::MouseMove(MouseDelta {
                    x: pos.x - input_state.mouse.x,
                    y: pos.y - input_state.mouse.y,
                });
                additional_events.push(evt);
                let other = self.process_event(input_state, additional_events.last().unwrap());
                additional_events.extend(other.into_iter());
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
        additional_events
    }
}

impl System<DI> for Input {
    fn initialize(event_bus: &mut EventBus<DI>, system: &StoredSystem<Self>)
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
    let events = {
        let inject = ctx.read().unwrap();
        let mut state = inject.write_sync::<InputState>().unwrap();
        system.process_event(&mut state, event)
    };

    for evt in events {
        ctx.publish(&evt)?;
    }

    Ok(())
}

/// Initialize the input system
pub fn initialize(bus: &EventBus<DI>) {
    let state = InputState::new();
    let mut di = bus.data().write().unwrap();
    di.put_sync(state);
}
