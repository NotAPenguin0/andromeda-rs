use egui::{Checkbox, Slider};
use world::World;

use crate::widgets::aligned_label::aligned_label_with;

pub fn show(context: &egui::Context, world: &mut World) {
    egui::Window::new("Render options")
        .resizable(true)
        .movable(true)
        .show(context, |ui| {
            aligned_label_with(ui, "Tessellation level", |ui| {
                ui.add(Slider::new(&mut world.options.tessellation_level, 1..=128));
            });
            aligned_label_with(ui, "Wireframe", |ui| {
                ui.add(Checkbox::without_text(&mut world.options.wireframe));
            });
        });
}
