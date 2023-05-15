use egui::Response;
use events::ClickWorldView;
use inject::DI;
use input::{InputState, MousePosition};
use log::trace;
use scheduler::EventBus;
use util::SafeUnwrap;

use crate::editor::camera_controller::enable_camera_over;
use crate::util::image_provider::ImageProvider;
use crate::widgets::resizable_image::resizable_image_window;

fn behaviour(response: Response, bus: &EventBus<DI>) {
    enable_camera_over(&response, bus).safe_unwrap();

    if response.clicked() {
        let di = bus.data().read().unwrap();
        let input = di.read_sync::<InputState>().unwrap();
        let mouse = input.mouse();
        let left_top = response.rect.left_top();
        let window_space_pos = MousePosition {
            x: mouse.x - left_top.x as f64,
            y: mouse.y - left_top.y as f64,
        };
        bus.publish(&ClickWorldView {
            position: window_space_pos,
        })
        .safe_unwrap();
    }
}

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
        |response| behaviour(response, bus),
        (800.0, 600.0).into(),
    );
}
