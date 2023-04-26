use camera::Camera;
use inject::DI;
use scheduler::EventBus;

/// Enable the camera controls when this widget is hovered
pub fn enable_camera_over(response: &egui::Response, bus: &EventBus<DI>) {
    let hover = response.hovered();
    let di = bus.data().read().unwrap();
    let camera = di.get::<Camera>().unwrap();
    camera.write().unwrap().enable_controls(hover);
}
