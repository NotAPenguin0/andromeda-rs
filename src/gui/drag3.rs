use glam::Vec3;

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