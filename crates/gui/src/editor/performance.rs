use std::time::Duration;

use egui::Ui;
use inject::DI;
use scheduler::EventBus;
use statistics::RendererStatistics;

use crate::widgets::aligned_label::aligned_label_with;

fn show_duration(ui: &mut Ui, duration: &Duration) {
    let micros = duration.as_micros();
    let ms = micros as f64 / 1000.0;
    ui.label(format!("{:.2} ms", ms));
}

pub fn show(context: &egui::Context, bus: &EventBus<DI>) {
    let di = bus.data().read().unwrap();
    let stats = di.read_sync::<RendererStatistics>().unwrap();
    egui::Window::new("Performance")
        .resizable(true)
        .movable(true)
        .show(context, |ui| {
            ui.collapsing("Pass timings", |ui| {
                for (name, duration) in stats.section_timings() {
                    if name != "all_render" {
                        aligned_label_with(ui, name, |ui| {
                            show_duration(ui, duration);
                        });
                    }
                }
                ui.separator();
                let time = stats.section_timings().get("all_render").unwrap();
                aligned_label_with(ui, "gpu time", |ui| {
                    show_duration(ui, time);
                });
            });
            aligned_label_with(ui, "frame time", |ui| {
                show_duration(ui, &stats.average_frame_time());
            });
        });
}
