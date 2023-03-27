use std::ops::{Add, Div, Mul, Sub};

use egui::{Align, Ui};
use glam::Vec3;

use crate::gui::widgets::aligned_label::aligned_label_with;
use crate::math::Rotation;

pub trait Draggable: Copy + Default + Sub<Self, Output = Self> + Add<Self, Output = Self> + Mul<f32, Output = Self> + Div<f32, Output = Self> {
    fn drag(&mut self, ui: &mut egui::Ui, speed: f64, digits: usize, suffix: &str) -> bool;
}

impl Draggable for Vec3 {
    fn drag(&mut self, ui: &mut Ui, speed: f64, digits: usize, suffix: &str) -> bool {
        // The reason this is inverted is because of the right_to_left layout used when showing this.
        let mut dirty = false;
        dirty |= self.z.drag(ui, speed, digits, suffix);
        dirty |= self.y.drag(ui, speed, digits, suffix);
        dirty |= self.x.drag(ui, speed, digits, suffix);
        dirty
    }
}

impl Draggable for Rotation {
    fn drag(&mut self, ui: &mut Ui, _speed: f64, _digits: usize, _suffix: &str) -> bool {
        // TODO: Maybe make speed and digits work with this too
        // The reason this is inverted is because of the right_to_left layout used when showing this.
        let mut dirty = false;
        dirty |= ui.drag_angle(&mut self.0.z).changed();
        dirty |= ui.drag_angle(&mut self.0.y).changed();
        dirty |= ui.drag_angle(&mut self.0.x).changed();
        dirty
    }
}

impl Draggable for f32 {
    fn drag(&mut self, ui: &mut Ui, speed: f64, digits: usize, suffix: &str) -> bool {
        ui.add(
            egui::DragValue::new(self)
                .speed(speed)
                .min_decimals(digits)
                .max_decimals(digits)
                .suffix(suffix),
        )
        .changed()
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
    base: T,
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
            base: T::default(),
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

    /// Set a base value. The value shown to the user will be shown with this base value subtracted (before scaling).
    pub fn relative_to(mut self, base: T) -> Self {
        self.base = base;
        self
    }

    /// Returns true if the value changed
    pub fn show(mut self, ui: &mut egui::Ui) -> bool {
        aligned_label_with(ui, self.label, |ui| {
            self.scaled_value = (*self.original_value - self.base) * self.scale;
            let response = self.scaled_value.drag(ui, self.speed, self.digits, self.suffix.unwrap_or(""));
            *self.original_value = (self.scaled_value / self.scale) + self.base;
            response
        })
        .inner
    }
}
