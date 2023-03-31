use std::sync::{Arc, RwLock};

use crate::gfx;
use crate::gfx::world::World;
use crate::gui::editor::camera_controller::CameraController;
use crate::gui::util::image_provider::ImageProvider;

pub mod camera_controller;
pub mod environment;
pub mod render_options;
pub mod terrain_options;
pub mod world_view;

#[derive(Debug)]
pub struct Editor {
    context: egui::Context,
    gfx: gfx::SharedContext,
    camera_controller: Arc<RwLock<CameraController>>,
}

impl Editor {
    pub fn new(
        context: egui::Context,
        gfx: gfx::SharedContext,
        camera_controller: Arc<RwLock<CameraController>>,
    ) -> Self {
        Self {
            context,
            gfx,
            camera_controller,
        }
    }

    pub fn show(&self, world: &mut World, image_provider: impl ImageProvider) {
        egui::CentralPanel::default().show(&self.context, |ui| {
            ui.heading("Editor");

            world_view::show(&self.context, image_provider, &self.camera_controller);
            environment::show(&self.context, world);
            render_options::show(&self.context, world);
            terrain_options::show(&self.context, self.gfx.clone(), world);
        });
    }
}
