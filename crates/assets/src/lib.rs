use anyhow::Result;
use gfx::SharedContext;
use inject::DI;
pub use resources::*;
use scheduler::EventBus;

pub mod resources;

pub fn initialize(mut bus: EventBus<DI>) -> Result<()> {
    let gfx = bus
        .data()
        .read()
        .unwrap()
        .get::<SharedContext>()
        .cloned()
        .unwrap();
    NormalMap::init_pipelines(gfx, &mut bus)?;
    Ok(())
}
