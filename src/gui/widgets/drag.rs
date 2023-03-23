use std::ops::{Div, Mul, RangeInclusive};

use egui::emath;
use glam::Vec3;

// TODO: Transform these methods into builders

pub fn drag<T: emath::Numeric>(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, value: &mut T, speed: impl Into<f64>) -> egui::InnerResponse<bool> {
    ui.horizontal(|ui| {
        ui.label(label);
        let speed = speed.into();
        ui.add(egui::DragValue::new(value).speed(speed)).changed()
    })
}

pub fn drag_fmt<T: emath::Numeric>(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
    fmt: impl Fn(f64) -> String,
    value: &mut T,
    speed: impl Into<f64>,
) -> egui::InnerResponse<bool> {
    ui.horizontal(|ui| {
        ui.label(label);
        let speed = speed.into();
        ui.add(egui::DragValue::new(value).speed(speed).custom_formatter(|val, _| fmt(val)))
            .changed()
    })
}

pub fn drag_fmt_scaled<T: emath::Numeric + Mul<T, Output = T> + Div<T, Output = T>>(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
    fmt: impl Fn(f64, RangeInclusive<usize>) -> String,
    parse: impl Fn(&str) -> Option<f64>,
    value: &mut T,
    speed: impl Into<f64>,
    scale: T,
) -> egui::InnerResponse<bool> {
    ui.horizontal(|ui| {
        ui.label(label);
        let speed = speed.into();
        let scale = scale.into();
        let mut val = *value * scale;
        let dirty = ui
            .add(
                egui::DragValue::new(&mut val)
                    .speed(speed)
                    .custom_formatter(fmt)
                    .custom_parser(parse),
            )
            .changed();
        *value = val / scale;
        dirty
    })
}

pub fn drag3(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, value: &mut Vec3, speed: impl Into<f64>) -> egui::InnerResponse<bool> {
    ui.horizontal(|ui| {
        ui.label(label);
        let speed = speed.into();
        let mut dirty = false;
        dirty |= ui.add(egui::DragValue::new(&mut value.x).speed(speed)).changed();
        dirty |= ui.add(egui::DragValue::new(&mut value.y).speed(speed)).changed();
        dirty |= ui.add(egui::DragValue::new(&mut value.z).speed(speed)).changed();
        dirty
    })
}

pub fn drag3_scaled(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
    value: &mut Vec3,
    speed: impl Into<f64>,
    scale: impl Into<f32>,
    decimals: usize,
) -> egui::InnerResponse<bool> {
    ui.horizontal(|ui| {
        ui.label(label);
        let speed = speed.into();
        let scale = scale.into();
        let mut dirty = false;
        let mut values = *value * scale;
        dirty |= ui
            .add(
                egui::DragValue::new(&mut values.x)
                    .speed(speed)
                    .min_decimals(decimals)
                    .max_decimals(decimals + 2),
            )
            .changed();
        dirty |= ui
            .add(
                egui::DragValue::new(&mut values.y)
                    .speed(speed)
                    .min_decimals(decimals)
                    .max_decimals(decimals + 2),
            )
            .changed();
        dirty |= ui
            .add(
                egui::DragValue::new(&mut values.z)
                    .speed(speed)
                    .min_decimals(decimals)
                    .max_decimals(decimals + 2),
            )
            .changed();
        *value = values / scale;
        dirty
    })
}

pub fn drag3_precise(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
    value: &mut Vec3,
    speed: impl Into<f64>,
    decimals: usize,
) -> egui::InnerResponse<bool> {
    ui.horizontal(|ui| {
        ui.label(label);
        let speed = speed.into();
        let mut dirty = false;
        dirty |= ui
            .add(
                egui::DragValue::new(&mut value.x)
                    .speed(speed)
                    .min_decimals(decimals)
                    .max_decimals(decimals + 3),
            )
            .changed();
        dirty |= ui
            .add(
                egui::DragValue::new(&mut value.y)
                    .speed(speed)
                    .min_decimals(decimals)
                    .max_decimals(decimals + 3),
            )
            .changed();
        dirty |= ui
            .add(
                egui::DragValue::new(&mut value.z)
                    .speed(speed)
                    .min_decimals(decimals)
                    .max_decimals(decimals + 3),
            )
            .changed();
        dirty
    })
}

pub fn drag3_angle(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, value: &mut Vec3) -> egui::InnerResponse<bool> {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut dirty = false;
        dirty |= ui.drag_angle(&mut value.x).changed();
        dirty |= ui.drag_angle(&mut value.y).changed();
        dirty |= ui.drag_angle(&mut value.z).changed();
        dirty
    })
}
