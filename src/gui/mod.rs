pub mod integration;
pub use integration::UIIntegration;

pub fn build_ui(context: &egui::Context) {
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
    });
}