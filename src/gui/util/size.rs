#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Size<T: Copy>(T, T);

pub type USize = Size<u32>;
pub type FSize = Size<f32>;

impl<T: Copy> Size<T> {
    pub fn new(x: T, y: T) -> Self {
        Self {
            0: x,
            1: y,
        }
    }

    pub fn x(&self) -> T {
        self.0
    }

    pub fn y(&self) -> T {
        self.1
    }
}

impl Into<egui::Vec2> for USize {
    fn into(self) -> egui::Vec2 {
        egui::Vec2 {
            x: self.0 as f32,
            y: self.1 as f32,
        }
    }
}

impl Into<egui::Vec2> for FSize {
    fn into(self) -> egui::Vec2 {
        egui::Vec2 {
            x: self.0,
            y: self.1,
        }
    }
}
