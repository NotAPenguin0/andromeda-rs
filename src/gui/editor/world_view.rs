use std::sync::{Arc, RwLock};

use crate::gui::editor::camera_controller::{enable_camera_over, CameraController};
use crate::gui::util::image_provider::ImageProvider;
use crate::gui::widgets::resizable_image::resizable_image_window;

pub fn show(
    context: &egui::Context,
    mut provider: impl ImageProvider,
    camera_controller: &Arc<RwLock<CameraController>>,
) {
    resizable_image_window(
        context,
        "World view",
        |size| provider.get_image(size),
        |response| enable_camera_over(&response, camera_controller),
        (800.0, 600.0).into(),
    );
}
