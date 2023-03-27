use std::sync::{Arc, RwLock};

use egui::{Response, Vec2};
use tiny_tokio_actor::*;
use tokio::runtime::Handle;

use crate::gfx;
use crate::gui::editor::camera_controller::{enable_camera_over, CameraController};
use crate::gui::util::image::Image;
use crate::gui::util::integration::UIIntegration;
use crate::gui::util::size::USize;
use crate::gui::widgets::resizable_image::resizable_image_window;

fn get_image(size: Vec2, targets: &mut gfx::RenderTargets, integration: &mut UIIntegration) -> Option<Image> {
    // Make output resolution match our window size.
    targets.set_output_resolution(size.x as u32, size.y as u32).ok()?;
    // Then grab our color output.
    let image = targets.get_target_view("resolved_output").unwrap();
    // We can re-register the same image, nothing will happen.
    let handle = integration.register_texture(&image);
    Some(handle)
}

fn behaviour(response: &Response, camera_controller: &Arc<RwLock<CameraController>>) {
    enable_camera_over(response, camera_controller);
}

pub fn show(context: &egui::Context, targets: &mut gfx::RenderTargets, integration: &mut UIIntegration, camera_controller: &Arc<RwLock<CameraController>>) {
    resizable_image_window(
        context,
        "World view",
        |size| get_image(size, targets, integration),
        |response| behaviour(&response, &camera_controller),
        (800.0, 600.0).into(),
    );
}
