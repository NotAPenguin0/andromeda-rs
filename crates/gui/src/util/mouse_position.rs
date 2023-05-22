use egui::Response;
use glam::Vec2;
use inject::DI;
use input::InputState;
use scheduler::EventBus;
use util::mouse_position::WorldMousePosition;

/// Update the mouse screen space position over a widget
/// # DI Access
/// - Write [`WorldMousePosition`]
/// - Read [`InputState`]
pub fn update_screen_space_position_over(response: &Response, bus: &EventBus<DI>) {
    let di = bus.data().read().unwrap();
    let mut state = di.write_sync::<WorldMousePosition>().unwrap();
    // If we are over the widget, compute the position relative to the widget's top-left corner.
    if response.hovered() {
        let input = di.read_sync::<InputState>().unwrap();
        let mouse = input.mouse();
        let left_top = response.rect.left_top();
        let window_space_pos = Vec2 {
            x: mouse.x as f32 - left_top.x,
            y: mouse.y as f32 - left_top.y,
        };
        state.screen_space = Some(window_space_pos);
    } else {
        // We are not over the widget, so both world and screen space positions do not exist.
        state.screen_space = None;
        state.world_space = None;
        state.terrain_uv = None;
    }
}
