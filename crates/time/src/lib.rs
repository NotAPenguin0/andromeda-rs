use std::time::{Duration, Instant};

use anyhow::Result;
use events::Tick;
use inject::DI;
use scheduler::{EventBus, EventContext, StoredSystem, System};

struct TimeSystem;

#[derive(Debug, Clone)]
pub struct Time {
    last_time: Instant,
    pub delta: Duration,
}

impl System<DI> for TimeSystem {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>) {
        event_bus.subscribe(system, handle_tick_event);
    }
}

fn handle_tick_event(
    _system: &mut TimeSystem,
    _event: &Tick,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    let di = ctx.read().unwrap();
    let mut time = di.write_sync::<Time>().unwrap();
    let now = Instant::now();
    time.delta = now - time.last_time;
    time.last_time = now;
    Ok(())
}

pub fn initialize(bus: &EventBus<DI>) -> Result<()> {
    bus.add_system(TimeSystem);
    let mut di = bus.data().write().unwrap();
    di.put_sync(Time {
        last_time: Instant::now(),
        delta: Default::default(),
    });
    Ok(())
}
