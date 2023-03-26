use crate::app::RootActorSystem;
use crate::gfx::world::World;

pub mod editor;
pub mod util;
pub mod widgets;

pub fn build_ui(context: &egui::Context, actors: &RootActorSystem, world: &mut World) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        editor::world_view::show(&context, &actors);
        editor::environment::show(&context, world);
        editor::render_options::show(&context, world);
    });
}
