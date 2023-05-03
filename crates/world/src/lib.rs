use anyhow::Result;
pub use atmosphere::*;
use inject::DI;
pub use render_options::*;
use scheduler::EventBus;
pub use world::*;

pub mod atmosphere;
pub mod render_options;
pub mod world;

pub fn initialize(bus: &EventBus<DI>) -> Result<()> {
    let world = World::new();
    let mut di = bus.data().write().unwrap();
    di.put_sync(world);
    Ok(())
}
