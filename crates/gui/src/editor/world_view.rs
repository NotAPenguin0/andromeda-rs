use brush::{BeginStrokeEvent, Brush, EndStrokeEvent, SmoothHeight};
use egui::{PointerButton, Response};
use events::DragWorldView;
use inject::DI;
use input::{ButtonState, InputState, MouseButton, MousePosition};
use scheduler::EventBus;
use util::SafeUnwrap;

use crate::editor::camera_controller::enable_camera_over;
use crate::util::image_provider::ImageProvider;
use crate::util::mouse_position::update_screen_space_position_over;
use crate::widgets::resizable_image::resizable_image_window;

/// # DI Access
/// - Read [`InputState`]
fn behaviour(response: Response, bus: &EventBus<DI>) {
    enable_camera_over(&response, bus).safe_unwrap();
    update_screen_space_position_over(&response, bus);

    let di = bus.data().read().unwrap();
    let input = di.read_sync::<InputState>().unwrap();

    // If a drag was started, begin the brush stroke
    if response.drag_started_by(PointerButton::Primary) {
        bus.publish(&BeginStrokeEvent {
            settings: Default::default(),
            brush: Brush::new(SmoothHeight {}),
        })
        .safe_unwrap();
    }

    if response.drag_released_by(PointerButton::Primary) {
        bus.publish(&EndStrokeEvent).safe_unwrap();
    }

    // Note: is_dragged_by() would not return true if the mouse is not moving
    if response.hovered() && input.get_mouse_key(MouseButton::Left) == ButtonState::Pressed {
        let mouse = input.mouse();
        let left_top = response.rect.left_top();
        let window_space_pos = MousePosition {
            x: mouse.x - left_top.x as f64,
            y: mouse.y - left_top.y as f64,
        };
        bus.publish(&DragWorldView {
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
