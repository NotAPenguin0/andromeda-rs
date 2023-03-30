use std::sync::{Arc, RwLock};

use crate::gfx;
use crate::gfx::world::World;
use crate::gui::editor::camera_controller::CameraController;
use crate::gui::image_provider::ImageProvider;
use crate::gui::util::integration::UIIntegration;

pub mod editor;
pub mod image_provider;
pub mod util;
pub mod widgets;

pub fn build_ui(
    context: egui::Context,
    gfx: gfx::SharedContext,
    image_provider: impl ImageProvider,
    camera_controller: &Arc<RwLock<CameraController>>,
    world: &mut World,
) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        editor::world_view::show(&context, image_provider, &camera_controller);
        editor::environment::show(&context, world);
        editor::render_options::show(&context, world);
        editor::terrain_options::show(&context, gfx, world);
    });
}
