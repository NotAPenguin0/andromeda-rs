use std::sync::{Arc, RwLock};

use inject::DI;
use scheduler::EventBus;

use crate::editor::camera_controller::enable_camera_over;
use crate::util::image_provider::ImageProvider;
use crate::widgets::resizable_image::resizable_image_window;

pub fn show(context: &egui::Context, bus: &EventBus<DI>) {
    resizable_image_window(
        context,
        "World view",
        |size| {
            let mut di = bus.data().write().unwrap();
            let provider = di.get_dyn_mut::<dyn ImageProvider>().unwrap();
            provider.get_image(size.into())
        },
        |response| enable_camera_over(&response, bus),
        (800.0, 600.0).into(),
    );
}
