use glam::Vec3;
use tiny_tokio_actor::{Message, Actor, ActorContext, ActorRef, async_trait, Handler, SystemEvent};
use crate::app::RootActorSystem;
use crate::core::{ButtonState, Event, Input, input, InputEvent, InputListener, Key, MouseButton, QueryKeyState, QueryMouseButton};
use crate::math::{Position, Rotation};
use crate::state::{Camera, QueryCameraVectors, UpdateCameraPosition, UpdateCameraRotation};

#[derive(Message)]
pub struct DragWorld {
    pub x: f32,
    pub y: f32
}

#[derive(Actor)]
pub struct CameraController {
    input: ActorRef<Event, Input>,
    camera: ActorRef<Event, Camera>,
}

impl CameraController {
    pub fn new(input: ActorRef<Event, Input>, camera: ActorRef<Event, Camera>) -> Self {
        Self {
            input,
            camera
        }
    }

    async fn handle_move(&self, drag: DragWorld) {
        let vectors = self.camera.ask(QueryCameraVectors).await.unwrap();
        let delta = vectors.up * drag.y + vectors.right * (-drag.x);
        const SPEED: f32 = 0.02;
        self.camera.tell(UpdateCameraPosition(Position(delta * SPEED))).unwrap();
    }

    async fn handle_rotate(&self, drag: DragWorld) {
        let delta = Vec3::new(drag.y, drag.x, 0.0);
        const SPEED: f32 = 0.02;
        self.camera.tell(UpdateCameraRotation(Rotation(delta * SPEED))).unwrap();
    }
}

#[async_trait]
impl<E> Handler<E, DragWorld> for CameraController where E: SystemEvent {
    async fn handle(&mut self, msg: DragWorld, _ctx: &mut ActorContext<E>) -> () {
        if self.input.ask(QueryMouseButton(MouseButton::Middle)).await.unwrap() == ButtonState::Pressed {
            if self.input.ask(QueryKeyState(Key::Shift)).await.unwrap() == ButtonState::Pressed {
                self.handle_move(msg).await;
            } else {
                self.handle_rotate(msg).await;
            }
        }
    }
}