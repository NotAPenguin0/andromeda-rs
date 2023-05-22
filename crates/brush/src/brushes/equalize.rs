use anyhow::Result;
use glam::Vec3;
use inject::DI;
use scheduler::EventBus;

use crate::{Brush, BrushSettings};

#[derive(Copy, Clone, Debug)]
pub struct Equalize {}

impl Brush for Equalize {
    fn apply(&self, bus: &EventBus<DI>, position: Vec3, settings: &BrushSettings) -> Result<()> {
        todo!()
    }
}
