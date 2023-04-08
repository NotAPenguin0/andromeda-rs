use std::time::Duration;

use egui::Ui;

use crate::gfx::renderer::statistics::StatisticsProvider;
use crate::gui::widgets::aligned_label::aligned_label_with;

fn show_duration(ui: &mut Ui, duration: &Duration) {
    let micros = duration.as_micros();
    let ms = micros as f64 / 1000.0;
    ui.label(format!("{:.2} ms", ms));
}

pub fn show(context: &egui::Context, stats: impl StatisticsProvider) {
    egui::Window::new("Performance")
        .resizable(true)
        .movable(true)
        .show(context, |ui| {
            ui.collapsing("Pass timings", |ui| {
                for (name, duration) in stats.section_timings() {
                    aligned_label_with(ui, name, |ui| {
                        show_duration(ui, duration);
                    });
                }
            });
        });
}
