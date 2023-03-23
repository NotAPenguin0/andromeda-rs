use egui::{Response, Vec2};
use tokio::runtime::Handle;

use crate::app::RootActorSystem;
use crate::gui::editor::camera_controller::control_camera;
use crate::gui::util::image::Image;
use crate::gui::util::size::USize;
use crate::gui::widgets::resizable_image::resizable_image_window;
use crate::gui::ResizeSceneTexture;

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
