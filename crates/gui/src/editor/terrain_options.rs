use assets::storage::AssetStorage;
use assets::TerrainLoadInfo;
use egui::Slider;
use inject::DI;
use scheduler::EventBus;
use world::World;

use crate::widgets::aligned_label::aligned_label_with;
use crate::widgets::drag::Drag;

pub fn show(context: &egui::Context, bus: &EventBus<DI>, world: &mut World) {
    egui::Window::new("Terrain options")
        .resizable(true)
        .movable(true)
        .show(context, |ui| {
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
                ui.add(Slider::new(&mut world.terrain_options.patch_resolution, 1..=64))
                    .changed()
            })
            .inner;

            // If changed, generate new terrain
            if dirty {
                let di = bus.data().read().unwrap();
                let assets = di.get::<AssetStorage>().unwrap();
                match world.terrain.take() {
                    None => {}
                    Some(old) => {
                        world.terrain = Some(assets.load(TerrainLoadInfo::FromNewMesh {
                            old,
                            options: world.terrain_options,
                        }));
                    }
                }
            }
        });
}
