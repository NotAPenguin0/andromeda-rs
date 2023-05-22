use brush::{BeginStrokeEvent, BrushSettings, BrushType, EndStrokeEvent, SmoothHeight};
use egui::{PointerButton, Response};
use events::DragWorldView;
use inject::DI;
use input::{ButtonState, InputState, MouseButton, MousePosition};
use scheduler::EventBus;
use util::SafeUnwrap;

use crate::editor::brushes::BrushWidget;
use crate::editor::camera_controller::enable_camera_over;
use crate::util::image_provider::ImageProvider;
use crate::util::mouse_position::update_screen_space_position_over;
use crate::widgets::resizable_image::resizable_image_window;

/// # DI Access
/// - Read [`InputState`]
fn behaviour(response: Response, bus: &EventBus<DI>, brushes: &mut BrushWidget) {
    enable_camera_over(&response, bus).safe_unwrap();
    update_screen_space_position_over(&response, bus);
    brushes.control(&response).safe_unwrap();
}

/// Show the world view
/// # DI Access
/// - Write [`ImageProvider`]
pub fn show(context: &egui::Context, bus: &EventBus<DI>, brushes: &mut BrushWidget) {
    resizable_image_window(
        context,
        "World view",
        |size| {
            let inject = bus.data().read().unwrap();
            let mut provider = inject.write_sync::<ImageProvider>().unwrap();
            provider.size = size.into();
            provider.handle
        },
        |response| behaviour(response, bus, brushes),
        (1440.0, 1000.0).into(),
    );
}
