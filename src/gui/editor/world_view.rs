use egui::{Response, Vec2};
use tiny_tokio_actor::*;
use tokio::runtime::Handle;

use crate::app::RootActorSystem;
use crate::gui::editor::camera_controller::control_camera;
use crate::gui::util::image::Image;
use crate::gui::util::size::USize;
use crate::gui::widgets::resizable_image::resizable_image_window;

#[derive(Debug, Copy, Clone, Message)]
#[response(Option < Image >)]
pub struct ResizeSceneTexture(USize);

#[derive(Debug, Copy, Clone, Message)]
#[response(Option < USize >)]
pub struct QuerySceneTextureSize;

#[derive(Debug, Copy, Clone, Message)]
#[response(Option < Image >)]
pub struct QueryCurrentSceneTexture;

#[derive(Debug, Copy, Clone, Message)]
pub struct SetNewTexture(pub Image);

#[derive(Default, Actor)]
pub struct TargetResizeActor {
    current_image: Option<Image>,
    new_size: Option<USize>,
}

#[async_trait]
impl<E> Handler<E, ResizeSceneTexture> for TargetResizeActor
where
    E: SystemEvent,
{
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
impl<E> Handler<E, QuerySceneTextureSize> for TargetResizeActor
where
    E: SystemEvent,
{
    async fn handle(&mut self, _msg: QuerySceneTextureSize, _ctx: &mut ActorContext<E>) -> Option<USize> {
        self.new_size
    }
}

#[async_trait]
impl<E> Handler<E, QueryCurrentSceneTexture> for TargetResizeActor
where
    E: SystemEvent,
{
    async fn handle(&mut self, _msg: QueryCurrentSceneTexture, _ctx: &mut ActorContext<E>) -> Option<Image> {
        self.current_image
    }
}

#[async_trait]
impl<E> Handler<E, SetNewTexture> for TargetResizeActor
where
    E: SystemEvent,
{
    async fn handle(&mut self, msg: SetNewTexture, _ctx: &mut ActorContext<E>) -> () {
        self.current_image = Some(msg.0);
        self.new_size = None;
    }
}

fn get_image(size: Vec2, actors: &RootActorSystem) -> Option<Image> {
    Handle::current()
        .block_on(
            actors
                .scene_texture
                .ask(ResizeSceneTexture(USize::new(size.x as u32, size.y as u32))),
        )
        .unwrap()
}

fn behaviour(response: &Response, actors: &RootActorSystem) {
    control_camera(response, actors);
}

pub fn show(context: &egui::Context, actors: &RootActorSystem) {
    resizable_image_window(
        context,
        "World view",
        |size| get_image(size, &actors),
        |response| behaviour(&response, &actors),
        (800.0, 600.0).into(),
    );
}
