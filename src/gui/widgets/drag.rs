use std::ops::{Div, Mul, RangeInclusive};

use egui::{emath, Ui};
use glam::Vec3;

use crate::math::Rotation;

pub trait Draggable: Copy + Mul<f32, Output = Self> + Div<f32, Output = Self> {
    fn drag(&mut self, ui: &mut egui::Ui, speed: f64, digits: usize, suffix: &str);
}

impl Draggable for Vec3 {
    fn drag(&mut self, ui: &mut Ui, speed: f64, digits: usize, suffix: &str) {
        self.x.drag(ui, speed, digits, suffix);
        self.y.drag(ui, speed, digits, suffix);
        self.z.drag(ui, speed, digits, suffix);
    }
}

impl Draggable for Rotation {
    fn drag(&mut self, ui: &mut Ui, _speed: f64, _digits: usize, _suffix: &str) {
        // TODO: Maybe make speed and digits work with this too
        ui.drag_angle(&mut self.0.x);
        ui.drag_angle(&mut self.0.y);
        ui.drag_angle(&mut self.0.z);
    }
}

impl Draggable for f32 {
    fn drag(&mut self, ui: &mut Ui, speed: f64, digits: usize, suffix: &str) {
        ui.add(
            egui::DragValue::new(self)
                .speed(speed)
                .min_decimals(digits)
                .max_decimals(digits)
                .suffix(suffix),
        );
    }
}

pub struct Drag<'v, 's, T, L>
where
    T: Draggable,
    L: Into<egui::WidgetText>, {
    original_value: &'v mut T,
    scaled_value: T,
    scale: f32,
    speed: f64,
    label: L,
    digits: usize,
    suffix: Option<&'s str>,
}

impl<'v, 's, T, L> Drag<'v, 's, T, L>
where
    T: Draggable,
    L: Into<egui::WidgetText>,
{
    pub fn new(label: L, value: &'v mut T) -> Self {
        Self {
            scaled_value: *value,
            original_value: value,
            label,
            digits: 2,
            suffix: None,
            scale: 1.0,
            speed: 1.0,
        }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self.scaled_value = *self.original_value * scale;
        self
    }

    pub fn suffix(mut self, suffix: &'s str) -> Self {
        self.suffix = Some(suffix);
        self
    }

    pub fn digits(mut self, digits: usize) -> Self {
        self.digits = digits;
        self
    }

    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    pub fn show(mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(self.label.into());
            self.scaled_value = *self.original_value * self.scale;
            self.scaled_value.drag(ui, self.speed, self.digits, self.suffix.unwrap_or(""));
            *self.original_value = self.scaled_value / self.scale;
        });
    }
}
