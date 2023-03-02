use tiny_tokio_actor::*;

use crate::core::{Event, SafeUnwrap};
use anyhow::Result;

#[derive(Default, Debug, Clone, Copy)]
pub struct MousePosition(pub f64, pub f64);

#[derive(Debug, Clone, Copy, Message)]
pub enum InputEvent {
    MouseMove(MousePosition),
}

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
        }
        self.process_event(msg).await;
    }
}

#[async_trait]
impl<E, L> Handler<E, AddInputListener<L>> for Input where E: SystemEvent, L: InputListener + 'static {
    async fn handle(&mut self, msg: AddInputListener<L>, ctx: &mut ActorContext<E>) -> () {
        self.listeners.push(Box::new(msg.0));
    }
}