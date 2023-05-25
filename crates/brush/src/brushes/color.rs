use anyhow::Result;
use glam::{Vec3, Vec4};
use inject::DI;
use scheduler::EventBus;

use crate::{Brush, BrushSettings};

#[derive(Copy, Clone, Debug, Default)]
pub struct Color {
    pub color: Vec4,
}

impl Brush for Color {
    fn apply(&self, bus: &EventBus<DI>, position: Vec3, settings: &BrushSettings) -> Result<()> {
        todo!()
    }
}
