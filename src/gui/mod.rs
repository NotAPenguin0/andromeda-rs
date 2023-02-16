pub mod integration;
pub use integration::UIIntegration;

pub fn build_ui(context: &egui::Context, scene_texture: egui::TextureId) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        egui::Window::new("Widget")
            .interactable(true)
            .movable(true)
            .resizable(true)
            .default_size((100.0, 100.0))
            .show(&context, |ui| {
                if ui.button("Do not press me").clicked() {
                    warn!("Told you");
                }
                ui.allocate_space(ui.available_size());
            });

        egui::Window::new("World view")
            .interactable(true)
            .movable(true)
            .default_size((1920.0, 1080.0))
            .show(&context, |ui| {
                ui.image(scene_texture, (1920.0, 1080.0)); // TODO: Custom image type that tracks size properly
            });
    });
}