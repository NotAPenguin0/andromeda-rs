pub mod integration;
pub mod image;
pub mod size;

pub use integration::UIIntegration;
pub use size::*;
pub use image::Image;

use tiny_tokio_actor::{Actor, ActorContext, ActorRef, async_trait, Handler, Message, SystemEvent};
use tokio::runtime::Handle;

use crate::event;

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
    async fn handle(&mut self, msg: ResizeSceneTexture, ctx: &mut ActorContext<E>) -> Option<Image> {
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
    async fn handle(&mut self, msg: QuerySceneTextureSize, ctx: &mut ActorContext<E>) -> Option<USize> {
        self.new_size
    }
}

#[async_trait]
impl<E> Handler<E, QueryCurrentSceneTexture> for TargetResizeActor where E: SystemEvent {
    async fn handle(&mut self, msg: QueryCurrentSceneTexture, ctx: &mut ActorContext<E>) -> Option<Image> {
        self.current_image
    }
}

#[async_trait]
impl<E> Handler<E, SetNewTexture> for TargetResizeActor where E: SystemEvent {
    async fn handle(&mut self, msg: SetNewTexture, ctx: &mut ActorContext<E>) -> () {
        self.current_image = Some(msg.0);
        self.new_size = None;
    }
}

pub fn build_ui(context: &egui::Context, scene_texture: ActorRef<event::Event, TargetResizeActor>) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        egui::Window::new("Widget")
            .interactable(true)
            .movable(true)
            .resizable(true)
            .default_size((100.0, 100.0))
            .show(&context, |ui| {
                if ui.button("Do not press me").clicked() {
                    warn!("Told you");
                }
                ui.allocate_space(ui.available_size());
            });

        egui::Window::new("World view")
            .interactable(true)
            .movable(true)
            .resizable(true)
            .default_size((800.0, 600.0))
            .show(&context, |ui| {
                let remaining_size = ui.available_size();
                // Send resize event to the scene texture actor, as a result we get the texture back
                let image = Handle::current().block_on(scene_texture.ask(ResizeSceneTexture(USize::new(remaining_size.x as u32, remaining_size.y as u32)))).unwrap();
                if let Some(image) = image {
                    ui.image(image.id, image.size);
                }
        });
    });
}