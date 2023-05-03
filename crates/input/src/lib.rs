use std::collections::HashMap;
use std::fmt::Debug;

use anyhow::Result;
use inject::DI;
use scheduler::{Event, EventBus};

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
pub struct Input {
    mouse: MousePosition,
    mouse_buttons: HashMap<MouseButton, ButtonState>,
    kb_buttons: HashMap<Key, ButtonState>,
    buffered_events: Vec<InputEvent>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            mouse: Default::default(),
            mouse_buttons: Default::default(),
            kb_buttons: Default::default(),
            buffered_events: vec![],
        }
    }

    pub fn process_event(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::MousePosition(pos) => {
                self.process_event(InputEvent::MouseMove(MouseDelta {
                    x: pos.x - self.mouse.x,
                    y: pos.y - self.mouse.y,
                }))?;
                self.mouse = pos;
            }
            InputEvent::MouseButton(state) => {
                self.mouse_buttons.insert(state.button, state.state);
            }
            InputEvent::Button(state) => {
                self.kb_buttons.insert(state.button, state.state);
            }
            InputEvent::Scroll(_) => {}
            InputEvent::MouseMove(_) => {}
        };
        self.buffered_events.push(event);
        Ok(())
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

    pub fn flush(&self, mut bus: EventBus<DI>) -> Result<()> {
        for event in &self.buffered_events {
            bus.publish(event)?;
        }
        Ok(())
    }

    pub fn clear_buffer(&mut self) {
        self.buffered_events.clear();
    }
}

/// Initialize the input system
pub fn initialize(bus: &EventBus<DI>) {
    let input = Input::new();
    let mut di = bus.data().write().unwrap();
    di.put(input);
}
