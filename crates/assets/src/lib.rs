#![allow(dead_code)]
#![feature(associated_type_defaults)]

use anyhow::Result;
use gfx::SharedContext;
use inject::DI;
pub use resources::*;
use scheduler::EventBus;

use crate::storage::AssetStorage;

pub mod asset;
pub mod handle;
pub mod resources;
pub mod storage;
pub mod texture;

pub fn initialize(mut bus: EventBus<DI>) -> Result<()> {
    let gfx = bus
        .data()
        .read()
        .unwrap()
        .get::<SharedContext>()
        .cloned()
        .unwrap();
    NormalMap::init_pipelines(gfx, &mut bus)?;
    AssetStorage::new_in_inject(bus);
    Ok(())
}
