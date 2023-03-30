use std::sync::{Arc, RwLock};

use egui::{Response, Vec2};

use crate::gfx;
use crate::gui::editor::camera_controller::{enable_camera_over, CameraController};
use crate::gui::image_provider::ImageProvider;
use crate::gui::util::image::Image;
use crate::gui::util::integration::UIIntegration;
use crate::gui::widgets::resizable_image::resizable_image_window;

fn behaviour(response: &Response, camera_controller: &Arc<RwLock<CameraController>>) {
    enable_camera_over(response, camera_controller);
}

pub fn show(
    context: &egui::Context,
    mut provider: impl ImageProvider,
    camera_controller: &Arc<RwLock<CameraController>>,
) {
    resizable_image_window(
        context,
        "World view",
        |size| provider.get_image(size),
        |response| behaviour(&response, &camera_controller),
        (800.0, 600.0).into(),
    );
}
