use std::sync::{Arc, RwLock};

use crate::app::RootActorSystem;
use crate::gfx;
use crate::gfx::world::{FutureWorld, World};
use crate::gui::editor::camera_controller::CameraController;

pub mod editor;
pub mod util;
pub mod widgets;

pub fn build_ui(
    context: &egui::Context,
    gfx: gfx::SharedContext,
    camera_controller: &Arc<RwLock<CameraController>>,
    actors: &RootActorSystem,
    future: &mut FutureWorld,
    world: &mut World,
) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        editor::world_view::show(&context, &actors, &camera_controller);
        editor::environment::show(&context, world);
        editor::render_options::show(&context, world);
        editor::terrain_options::show(&context, gfx, future, world);
    });
}
