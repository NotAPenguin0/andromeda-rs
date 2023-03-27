use std::collections::HashMap;
use std::fmt::Debug;

use anyhow::Result;

use crate::core::SafeUnwrap;

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

pub trait InputListener {
    fn handle(&self, event: InputEvent, input: &Input) -> Result<()>;
}

#[derive(Default, Derivative)]
#[derivative(Debug)]
pub struct Input {
    mouse: MousePosition,
    mouse_buttons: HashMap<MouseButton, ButtonState>,
    kb_buttons: HashMap<Key, ButtonState>,
    #[derivative(Debug = "ignore")]
    listeners: Vec<Box<dyn InputListener>>,
}

impl Input {
    fn fire_event_listeners(&self, event: InputEvent) {
        for listener in &self.listeners {
            listener.handle(event, &self).safe_unwrap();
        }
    }

    pub fn process_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::MousePosition(pos) => {
                self.process_event(InputEvent::MouseMove(MouseDelta {
                    x: pos.x - self.mouse.x,
                    y: pos.y - self.mouse.y,
                }));
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
        }
        self.fire_event_listeners(event);
    }

    pub fn add_listener<L: InputListener + Debug + 'static>(&mut self, listener: L) {
        debug!("Added input listener '{:#?}'", listener);
        self.listeners.push(Box::new(listener));
    }

    pub fn get_key(&self, key: Key) -> ButtonState {
        self.kb_buttons.get(&key).cloned().unwrap_or(ButtonState::Released)
    }

    pub fn get_mouse_key(&self, key: MouseButton) -> ButtonState {
        self.mouse_buttons.get(&key).cloned().unwrap_or(ButtonState::Released)
    }
}
