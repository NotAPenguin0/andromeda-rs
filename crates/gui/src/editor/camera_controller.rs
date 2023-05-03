use anyhow::Result;
use camera::{Camera, EnableCameraEvent};
use inject::DI;
use scheduler::EventBus;

/// Enable the camera controls when this widget is hovered
pub fn enable_camera_over(response: &egui::Response, bus: &EventBus<DI>) -> Result<()> {
    let hover = response.hovered();
    bus.publish(&EnableCameraEvent {
        enabled: hover,
    })
}
