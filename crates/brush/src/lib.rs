use anyhow::Result;
use assets::storage::AssetStorage;
use events::ClickWorldView;
use inject::DI;
use scheduler::{EventBus, EventContext, StoredSystem, System};
use world::World;

struct BrushSystem {}

impl BrushSystem {
    pub fn new() -> Self {
        Self {}
    }
}

impl System<DI> for BrushSystem {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>) {
        event_bus.subscribe(system, handle_click_world_view);
    }
}

fn handle_click_world_view(
    system: &mut BrushSystem,
    click: &ClickWorldView,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    Ok(())
}

pub fn initialize(bus: &EventBus<DI>) {
    let system = BrushSystem::new();
    bus.add_system(system);
}
