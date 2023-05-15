use inject::DI;
use scheduler::{EventBus, StoredSystem, System};

struct BrushSystem {}

impl BrushSystem {
    pub fn new() -> Self {
        Self {}
    }
}

impl System<DI> for BrushSystem {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>) {}
}

pub fn initialize(bus: &EventBus<DI>) {
    let system = BrushSystem::new();
    bus.add_system(system);
}
