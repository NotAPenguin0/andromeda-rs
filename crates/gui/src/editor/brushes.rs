use anyhow::Result;
use brush::{BeginStrokeEvent, Brush, BrushSettings, EndStrokeEvent, SmoothHeight};
use egui::{Context, Slider};
use inject::DI;
use scheduler::EventBus;

use crate::widgets::aligned_label::aligned_label_with;
use crate::widgets::drag::Drag;

#[derive(Debug)]
pub struct BrushWidget {
    pub settings: BrushSettings,
    pub active_brush: Option<Brush>,
}

impl BrushWidget {
    pub fn show(&mut self, ctx: &Context) -> Result<()> {
        egui::Window::new("Brushes")
            .movable(true)
            .resizable(true)
            .show(ctx, |ui| {
                ui.collapsing("General settings", |ui| {
                    aligned_label_with(ui, "Radius", |ui| {
                        ui.add(Slider::new(&mut self.settings.radius, 1..=512));
                    });
                    aligned_label_with(ui, "Strength", |ui| {
                        ui.add(Slider::new(&mut self.settings.weight, 0.01..=5.0));
                    });
                })
            });
        Ok(())
    }

    pub fn begin_stroke(&self, bus: &EventBus<DI>) -> Result<()> {
        match &self.active_brush {
            None => {}
            Some(brush) => {
                bus.publish(&BeginStrokeEvent {
                    settings: self.settings,
                    brush: Brush::new(SmoothHeight {}),
                })?;
            }
        }
        Ok(())
    }

    pub fn end_stroke(&self, bus: &EventBus<DI>) -> Result<()> {
        bus.publish(&EndStrokeEvent)?;
        Ok(())
    }
}
