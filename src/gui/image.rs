use crate::gui::USize;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Image {
    pub id: egui::TextureId,
    pub size: USize,
}