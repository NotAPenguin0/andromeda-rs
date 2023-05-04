use std::sync::{Arc, RwLock};

use inject::DI;
use scheduler::EventBus;
use world::World;

use crate::util::image::Image;

pub mod camera_controller;
pub mod environment;
pub mod performance;
pub mod render_options;
pub mod terrain_options;
pub mod world_view;

#[derive(Debug)]
pub struct Editor {
    context: egui::Context,
    bus: EventBus<DI>,
}

impl Editor {
    pub fn new(context: egui::Context, bus: EventBus<DI>) -> Self {
        Self {
            context,
            bus,
        }
    }

    pub fn show(&self, world: &mut World) {
        egui::CentralPanel::default().show(&self.context, |ui| {
            ui.heading("Editor");

            world_view::show(&self.context, &self.bus);
            environment::show(&self.context, world);
            render_options::show(&self.context, world);
            terrain_options::show(&self.context, &self.bus, world);
            performance::show(&self.context, &self.bus);
        });
    }
}
