use egui::Slider;

use crate::gfx;
use crate::gfx::resource::terrain::Terrain;
use crate::gfx::world::World;
use crate::gui::widgets::aligned_label::aligned_label_with;
use crate::gui::widgets::drag::Drag;

pub fn show(context: &egui::Context, gfx: gfx::SharedContext, world: &mut World) {
    egui::Window::new("Terrain options")
        .resizable(true)
        .movable(true)
        .show(&context, |ui| {
            let mut dirty =
                Drag::new("Terrain horizontal scale", &mut world.terrain_options.horizontal_scale)
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
                let options = world.terrain_options.clone();
                match world.terrain.take() {
                    None => {}
                    Some(old) => {
                        world.terrain.promise(Terrain::from_new_mesh(
                            old.height_map,
                            old.normal_map,
                            old.diffuse_map,
                            options,
                            gfx,
                        ));
                    }
                }
            }
        });
}
