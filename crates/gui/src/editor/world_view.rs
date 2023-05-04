use std::sync::{Arc, RwLock};

use inject::DI;
use scheduler::EventBus;
use util::SafeUnwrap;

use crate::editor::camera_controller::enable_camera_over;
use crate::util::image::Image;
use crate::util::image_provider::ImageProvider;
use crate::widgets::resizable_image::resizable_image_window;

/// Show the world view
/// # DI Access
/// - Write [`ImageProvider`]
pub fn show(context: &egui::Context, bus: &EventBus<DI>) {
    resizable_image_window(
        context,
        "World view",
        |size| {
            let inject = bus.data().read().unwrap();
            let mut provider = inject.write_sync::<ImageProvider>().unwrap();
            provider.size = size.into();
            provider.handle
        },
        |response| enable_camera_over(&response, bus).safe_unwrap(),
        (800.0, 600.0).into(),
    );
}
