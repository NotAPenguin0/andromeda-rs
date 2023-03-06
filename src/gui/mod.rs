pub mod integration;
pub mod image;
pub mod size;

mod drag3;
mod async_actor_widget;
mod camera_controller;

use egui::{Color32, PointerButton, Pos2, Rect, Sense, Vec2};
pub use integration::UIIntegration;
pub use size::*;
pub use image::Image;
pub use camera_controller::CameraController;
pub use camera_controller::CameraScrollListener;

use tiny_tokio_actor::{Actor, ActorContext, async_trait, Handler, Message, SystemEvent};
use tokio::runtime::Handle;
use crate::app::{repaint, RepaintAll, RootActorSystem};
use crate::gui::async_actor_widget::actor_edit;
use crate::gui::drag3::{drag3, drag3_angle};
use crate::{math, state};
use crate::gui::camera_controller::{DragWorld, MouseOverWorld};

#[derive(Debug, Copy, Clone)]
pub struct ResizeSceneTexture(USize);

impl Message for ResizeSceneTexture {
    type Response = Option<Image>;
}

#[derive(Debug, Copy, Clone)]
pub struct QuerySceneTextureSize;

impl Message for QuerySceneTextureSize {
    type Response = Option<USize>;
}

#[derive(Debug, Copy, Clone)]
pub struct QueryCurrentSceneTexture;

impl Message for QueryCurrentSceneTexture {
    type Response = Option<Image>;
}

#[derive(Debug, Copy, Clone)]
pub struct SetNewTexture(pub Image);

impl Message for SetNewTexture {
    type Response = ();
}

#[derive(Default)]
pub struct TargetResizeActor {
    current_image: Option<Image>,
    new_size: Option<USize>,
}

impl<E> Actor<E> for TargetResizeActor where E: SystemEvent {}

#[async_trait]
impl<E> Handler<E, ResizeSceneTexture> for TargetResizeActor where E: SystemEvent {
    async fn handle(&mut self, msg: ResizeSceneTexture, _ctx: &mut ActorContext<E>) -> Option<Image> {
        if let Some(cur) = &self.current_image {
            if cur.size != msg.0 {
                self.new_size = Some(msg.0);
            }
        } else {
            self.new_size = Some(msg.0);
        }

        return self.current_image;
    }
}

#[async_trait]
impl<E> Handler<E, QuerySceneTextureSize> for TargetResizeActor where E: SystemEvent {
    async fn handle(&mut self, _msg: QuerySceneTextureSize, _ctx: &mut ActorContext<E>) -> Option<USize> {
        self.new_size
    }
}

#[async_trait]
impl<E> Handler<E, QueryCurrentSceneTexture> for TargetResizeActor where E: SystemEvent {
    async fn handle(&mut self, _msg: QueryCurrentSceneTexture, _ctx: &mut ActorContext<E>) -> Option<Image> {
        self.current_image
    }
}

#[async_trait]
impl<E> Handler<E, SetNewTexture> for TargetResizeActor where E: SystemEvent {
    async fn handle(&mut self, msg: SetNewTexture, _ctx: &mut ActorContext<E>) -> () {
        self.current_image = Some(msg.0);
        self.new_size = None;
    }
}

pub fn build_ui(context: &egui::Context, actors: &RootActorSystem) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        let dirty = egui::Window::new("Camera settings")
            .interactable(true)
            .movable(true)
            .resizable(true)
            .show(&context, |ui| {
                Handle::current().block_on(async {
                    let mut dirty = actor_edit::<math::Position, state::QueryCameraPosition, state::SetCameraPosition, _, _>(ui, actors.camera.clone(), |ui, value| {
                        drag3(ui, "Position", &mut value.0, 0.1).inner
                    }).await;
                    dirty |= actor_edit::<math::Rotation, state::QueryCameraRotation, state::SetCameraRotation, _, _>(ui, actors.camera.clone(), |ui, value| {
                        drag3_angle(ui, "Rotation", &mut value.0).inner
                    }).await;

                    dirty
                })
            });

        match dirty {
            None => {}
            Some(response) => {
                match response.inner {
                    Some(true) => {
                        actors.repaint.tell(repaint::RepaintAll).unwrap();
                    },
                    _ => {}
                }
            }
        }

        egui::Window::new("World view")
            .resizable(true)
            .default_size((800.0, 600.0))
            .movable(true)
            .show(&context, |ui| {
                let cursor = ui.cursor();
                let remaining_size = ui.available_size();
                let (response, painter) = ui.allocate_painter(remaining_size, Sense::drag());
                // Send resize event to the scene texture actor, as a result we get the texture back
                let image = Handle::current().block_on(actors.scene_texture.ask(
                    ResizeSceneTexture(USize::new(
                        remaining_size.x as u32,
                        remaining_size.y as u32))))
                    .unwrap();
                if let Some(image) = image {
                    painter.image(
                        image.id,
                        Rect::from_min_size(cursor.min, remaining_size),
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE
                    );
                }

                // Handle drag events and send them to the camera controller
                if response.dragged() {
                    actors.camera_controller.tell(DragWorld {
                        x: response.drag_delta().x,
                        y: response.drag_delta().y,
                    }).unwrap();
                    actors.repaint.tell(RepaintAll).unwrap();
                }

                let hover = response.hovered();
                actors.camera_controller.tell(MouseOverWorld(hover)).unwrap();
        });
    });
}