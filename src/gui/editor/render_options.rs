use egui::Slider;

use crate::gfx::world::World;

pub fn show(context: &egui::Context, world: &mut World) {
    egui::Window::new("Render options")
        .resizable(true)
        .movable(true)
        .show(&context, |ui| {
            ui.horizontal(|ui| {
                ui.label("Tessellation level");
                ui.add(Slider::new(&mut world.options.tessellation_level, 1..=32));
            });
        });
}
