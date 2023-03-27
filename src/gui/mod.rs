use std::sync::{Arc, RwLock};

use crate::app::RootActorSystem;
use crate::gfx;
use crate::gfx::world::{FutureWorld, World};
use crate::gui::editor::camera_controller::CameraController;
use crate::gui::util::integration::UIIntegration;

pub mod editor;
pub mod util;
pub mod widgets;

pub fn build_ui(
    context: &egui::Context,
    integration: &mut UIIntegration,
    gfx: gfx::SharedContext,
    targets: &mut gfx::RenderTargets,
    camera_controller: &Arc<RwLock<CameraController>>,
    future: &mut FutureWorld,
    world: &mut World,
) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        editor::world_view::show(&context, targets, integration, &camera_controller);
        editor::environment::show(&context, world);
        editor::render_options::show(&context, world);
        editor::terrain_options::show(&context, gfx, future, world);
    });
}
