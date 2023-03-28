use egui::Slider;

use crate::gfx;
use crate::gfx::resource::TerrainPlane;
use crate::gfx::world::{FutureWorld, World};
use crate::gui::widgets::aligned_label::aligned_label_with;
use crate::gui::widgets::drag::Drag;

pub fn show(context: &egui::Context, gfx: gfx::SharedContext, future: &mut FutureWorld, world: &mut World) {
    egui::Window::new("Terrain options")
        .resizable(true)
        .movable(true)
        .show(&context, |ui| {
            let mut dirty = Drag::new("Terrain horizontal scale", &mut world.terrain_options.horizontal_scale)
                .speed(1.0)
                .suffix(" m")
                .show(ui);
            Drag::new("Terrain vertical scale", &mut world.terrain_options.vertical_scale)
                .speed(1.0)
                .suffix(" m")
                .show(ui);
            dirty |= aligned_label_with(ui, "Patch resolution", |ui| {
                ui.add(Slider::new(&mut world.terrain_options.patch_resolution, 1..=32))
                    .changed()
            })
            .inner;

            // If changed, generate new terrain
            if dirty {
                future.terrain_mesh = Some(TerrainPlane::generate(gfx, world.terrain_options));
            }
        });
}
