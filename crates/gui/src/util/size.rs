use egui::Vec2;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Size<T: Copy>(T, T);

pub type USize = Size<u32>;
pub type FSize = Size<f32>;

impl<T: Copy> Size<T> {
    pub fn new(x: T, y: T) -> Self {
        Self(x, y)
    }

    pub fn x(&self) -> T {
        self.0
    }

    pub fn y(&self) -> T {
        self.1
    }
}

impl From<Vec2> for USize {
    fn from(value: Vec2) -> Self {
        Self(value.x as u32, value.y as u32)
    }
}

impl From<USize> for Vec2 {
    fn from(value: USize) -> Self {
        Vec2 {
            x: value.x() as f32,
            y: value.y() as f32,
        }
    }
}

impl From<FSize> for Vec2 {
    fn from(value: FSize) -> Self {
        Vec2 {
            x: value.x(),
            y: value.y(),
        }
    }
}
