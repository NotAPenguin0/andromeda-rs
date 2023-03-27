use egui::{Align, InnerResponse, Ui};

/// Draws a horizontal section with the label on the left, aligned to the left, and the widget contents on the right, aligned to the right.
pub fn aligned_label_with<R>(ui: &mut Ui, label: impl Into<egui::WidgetText>, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.horizontal(|ui| {
        ui.label(label.into());
        ui.with_layout(egui::Layout::right_to_left(Align::Center), add_contents)
    })
    .inner
}
