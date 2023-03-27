use egui::{Color32, Pos2, Rect, Response, Sense, Vec2};

use crate::gui::util::image::Image;

pub fn resizable_image_window(
    context: &egui::Context,
    title: impl Into<egui::WidgetText>,
    get_image: impl FnOnce(Vec2) -> Option<Image>,
    behaviour: impl FnOnce(Response) -> (),
    default_size: Vec2,
) {
    egui::Window::new(title)
        .resizable(true)
        .default_size(default_size)
        .movable(true)
        .show(&context, |ui| {
            let cursor = ui.cursor();
            let remaining_size = ui.available_size();
            let (response, painter) = ui.allocate_painter(remaining_size, Sense::drag());
            // Get the image of the correct size
            let image = get_image(remaining_size);
            if let Some(image) = image {
                painter.image(
                    image.id,
                    Rect::from_min_size(cursor.min, remaining_size),
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            behaviour(response);
        });
}
