use std::time::Duration;

use egui::plot::{Legend, Line, Plot, PlotPoints};
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
            ui.collapsing("Plot", |ui| {
                context.request_repaint();
                let points = stats
                    .last_frame_times()
                    .zip(0..stats.frame_time_samples())
                    .map(|(duration, index)| [index as f64, duration.as_millis() as f64])
                    .collect::<PlotPoints>();
                let line = Line::new(points).name("ms");
                Plot::new("Frame time")
                    .auto_bounds_x()
                    .auto_bounds_y()
                    .include_y(0.0)
                    .legend(Legend::default())
                    .show_x(false)
                    .show_axes([false, true])
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });
            });
        });
}
