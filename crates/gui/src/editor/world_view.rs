use std::sync::{Arc, RwLock};

use inject::DI;
use scheduler::EventBus;
use util::SafeUnwrap;

use crate::editor::camera_controller::enable_camera_over;
use crate::util::image::Image;
use crate::util::image_provider::ImageProvider;
use crate::widgets::resizable_image::resizable_image_window;

pub fn show(context: &egui::Context, bus: &EventBus<DI>, target: Option<Image>) {
    resizable_image_window(
        context,
        "World view",
        |size| target,
        |response| enable_camera_over(&response, bus).safe_unwrap(),
        (800.0, 600.0).into(),
    );
}
