use anyhow::Result;
use brush::brushes::*;
use brush::height::WeightFunction;
use brush::{BeginStrokeEvent, Brush, BrushSettings, BrushType, EndStrokeEvent};
use egui::{Context, Frame, Margin, PointerButton, Response, Slider, Ui, Vec2, WidgetText};
use enum_dispatch::enum_dispatch;
use events::DragWorldView;
use inject::DI;
use input::{ButtonState, InputState, Key, MousePosition};
use scheduler::EventBus;

use crate::editor::{BrushDecalInfo, WorldOverlayInfo};
use crate::widgets::aligned_label::aligned_label_with;
use crate::widgets::toolbar::Toolbar;

#[derive(Debug)]
pub struct BrushWidget {
    pub bus: EventBus<DI>,
    pub settings: BrushSettings,
    pub active_brush: Option<BrushType>,
}

impl BrushWidget {
    fn begin_stroke(&self) -> Result<()> {
        match &self.active_brush {
            None => {}
            Some(brush) => {
                self.bus.publish(&BeginStrokeEvent {
                    settings: self.settings,
                    brush: *brush,
                })?;
            }
        }
        Ok(())
    }

    fn end_stroke(&self) -> Result<()> {
        {
            let di = self.bus.data().read().unwrap();
            let mut overlay = di.write_sync::<WorldOverlayInfo>().unwrap();
            overlay.brush_decal = None;
        }
        self.bus.publish(&EndStrokeEvent)?;
        Ok(())
    }
}

impl BrushWidget {
    pub fn show(&mut self, ctx: &Context) -> Result<()> {
        egui::Window::new("Brush toolbar")
            .movable(true)
            .resizable(true)
            .min_width(305.0)
            .show(ctx, |ui| {
                let toolbar_button_size = 24.0;
                let style = ui.style();
                let mut side_panel_frame = Frame::side_top_panel(style);
                side_panel_frame.inner_margin.left = 0.0;
                egui::SidePanel::left("Toolbar")
                    .frame(side_panel_frame)
                    .resizable(false)
                    .exact_width(toolbar_button_size)
                    .show_inside(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            Toolbar::new(&mut self.active_brush)
                                .size(toolbar_button_size)
                                .tool("↕", "Height brush", SmoothHeight::default().into())
                                .show(ui);
                        });
                    });
                ui.vertical(|ui| {
                    let heading_separator = |ui: &mut Ui, label: &str| {
                        let mut frame = Frame::central_panel(ui.style());
                        frame.inner_margin.bottom = 0.0;
                        frame.inner_margin.top = 0.0;
                        frame.show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading(label);
                            });
                        });
                        ui.separator();
                    };
                    heading_separator(ui, "Global settings");
                    Frame::central_panel(ui.style()).show(ui, |ui| {
                        aligned_label_with(ui, "Radius", |ui| {
                            ui.add(Slider::new(&mut self.settings.radius, 1.0..=128.0));
                        });
                        aligned_label_with(ui, "Strength", |ui| {
                            ui.add(Slider::new(&mut self.settings.weight, 0.01..=5.0));
                        });
                    });
                    ui.separator();
                    heading_separator(ui, "Brush settings");
                    Frame::central_panel(ui.style()).show(ui, |ui| {
                        if let Some(brush) = &mut self.active_brush {
                            match brush {
                                // This is correct after running the proc macro, but IntelliJ and rust-analyzer don't like it very much.
                                // For this reason, I've added an additional type hint in each case to make using this easier.
                                BrushType::SmoothHeight(brush) => {
                                    let brush: &mut SmoothHeight = brush;
                                    aligned_label_with(ui, "Weight function", |ui| {
                                        egui::ComboBox::from_id_source("brush_weight_fn")
                                            .selected_text(format!("{}", brush.weight_fn))
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut brush.weight_fn,
                                                    WeightFunction::Gaussian(0.3),
                                                    "Gaussian",
                                                );
                                            });
                                    });
                                    // Display options for each weight function separately
                                    match &mut brush.weight_fn {
                                        WeightFunction::Gaussian(stddev) => {
                                            aligned_label_with(ui, "Standard deviation", |ui| {
                                                ui.add(Slider::new(stddev, 0.0001f32..=0.40f32));
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    });
                });
            });
        // If we have an active brush, set the overlay decal to its radius
        let di = self.bus.data().read().unwrap();
        let mut overlay = di.write_sync::<WorldOverlayInfo>().unwrap();
        if self.active_brush.is_some() {
            overlay.brush_decal = Some(BrushDecalInfo {
                radius: self.settings.radius,
                data: self.active_brush.unwrap().decal_data(),
                shader: self.active_brush.unwrap().decal_shader().to_owned(),
            });
        } else {
            // Otherwise disable decal
            overlay.brush_decal = None;
        }
        Ok(())
    }

    pub fn control(&mut self, response: &Response) -> Result<()> {
        let di = self.bus.data().read().unwrap();
        let input = di.read_sync::<InputState>().unwrap();

        if input.get_key(Key::Escape) == ButtonState::Pressed {
            self.active_brush = None;
        }
        // If a drag was started, begin the brush stroke
        if response.drag_started_by(PointerButton::Primary) {
            self.settings.invert = false;
            self.begin_stroke()?;
        } else if response.drag_started_by(PointerButton::Secondary) {
            self.settings.invert = true;
            self.begin_stroke()?;
        }

        if response.drag_released_by(PointerButton::Primary)
            || response.drag_released_by(PointerButton::Secondary)
        {
            self.end_stroke()?;
        }

        if response.dragged_by(PointerButton::Primary)
            || response.dragged_by(PointerButton::Secondary)
        {
            let mouse = input.mouse();
            let left_top = response.rect.left_top();
            let window_space_pos = MousePosition {
                x: mouse.x - left_top.x as f64,
                y: mouse.y - left_top.y as f64,
            };
            self.bus.publish(&DragWorldView {
                position: window_space_pos,
            })?;
        }
        Ok(())
    }
}
