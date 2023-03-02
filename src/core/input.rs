use std::collections::HashMap;
use tiny_tokio_actor::*;

use crate::core::SafeUnwrap;
use anyhow::Result;
use std::fmt::Debug;

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
    pub y: f64
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

#[derive(Debug, Clone, Copy, Message)]
pub enum InputEvent {
    MouseMove(MousePosition),
    MouseButton(MouseButtonState),
    Button(KeyState)
}

#[derive(Debug, Copy, Clone, Message)]
#[response(ButtonState)]
pub struct QueryMouseButton(pub MouseButton);

#[derive(Debug, Copy, Clone, Message)]
#[response(ButtonState)]
pub struct QueryKeyState(pub Key);

#[async_trait]
pub trait InputListener: Send {
    async fn handle(&mut self, event: InputEvent) -> Result<()>;
}

pub struct AddInputListener<L>(pub L) where L: InputListener;

impl<L> Message for AddInputListener<L> where L: InputListener + 'static {
    type Response = ();
}

unsafe impl<L> Send for AddInputListener<L> where L: InputListener {}
unsafe impl<L> Sync for AddInputListener<L> where L: InputListener {}

#[derive(Default, Derivative, Actor)]
#[derivative(Debug)]
pub struct Input {
    mouse: MousePosition,
    mouse_buttons: HashMap<MouseButton, ButtonState>,
    kb_buttons: HashMap<Key, ButtonState>,
    #[derivative(Debug="ignore")]
    listeners: Vec<Box<dyn InputListener>>,
}

unsafe impl Send for Input {}
unsafe impl Sync for Input {}

impl Input {
    async fn process_event(&mut self, event: InputEvent) {
        for listener in &mut self.listeners {
            listener.handle(event).await.safe_unwrap();
        }
    }
}

#[async_trait]
impl<E> Handler<E, InputEvent> for Input where E: SystemEvent {
    async fn handle(&mut self, msg: InputEvent, _ctx: &mut ActorContext<E>) -> () {
        match msg {
            InputEvent::MouseMove(pos) => { self.mouse = pos; }
            InputEvent::MouseButton(state) => { self.mouse_buttons.insert(state.button, state.state); }
            InputEvent::Button(state) => { self.kb_buttons.insert(state.button, state.state); }
        }
        self.process_event(msg).await;
    }
}

#[async_trait]
impl<E> Handler<E, QueryMouseButton> for Input where E: SystemEvent {
    async fn handle(&mut self, msg: QueryMouseButton, _ctx: &mut ActorContext<E>) -> ButtonState {
        self.mouse_buttons.get(&msg.0).cloned().unwrap_or(ButtonState::Released)
    }
}

#[async_trait]
impl<E> Handler<E, QueryKeyState> for Input where E: SystemEvent {
    async fn handle(&mut self, msg: QueryKeyState, _ctx: &mut ActorContext<E>) -> ButtonState {
        self.kb_buttons.get(&msg.0).cloned().unwrap_or(ButtonState::Released)
    }
}

#[async_trait]
impl<E, L> Handler<E, AddInputListener<L>> for Input where E: SystemEvent, L: InputListener + Debug + 'static {
    async fn handle(&mut self, msg: AddInputListener<L>, _ctx: &mut ActorContext<E>) -> () {
        debug!("Added input listener '{:#?}'", msg.0);
        self.listeners.push(Box::new(msg.0));
    }
}
