use anyhow::Result;
use brush::{BeginStrokeEvent, Brush, BrushSettings, BrushType, EndStrokeEvent, SmoothHeight};
use egui::{Context, PointerButton, Response, Slider};
use events::DragWorldView;
use inject::DI;
use input::{ButtonState, InputState, Key, MouseButton, MousePosition};
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
            .show(ctx, |ui| {
                Toolbar::new(&mut self.active_brush)
                    .tool("↕", SmoothHeight::default().into())
                    .show(ui);
                // "↕"
            });
        // If we have an active brush, set the overlay decal to its radius
        let di = self.bus.data().read().unwrap();
        let mut overlay = di.write_sync::<WorldOverlayInfo>().unwrap();
        if self.active_brush.is_some() {
            overlay.brush_decal = Some(BrushDecalInfo {
                radius: self.settings.radius,
                data: None,
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
