use std::intrinsics::likely;

use anyhow::Result;
use assets::storage::AssetStorage;
use events::ClickWorldView;
use glam::Vec3;
use inject::DI;
use scheduler::{EventBus, EventContext, StoredSystem, System};
use util::mouse_position::WorldMousePosition;
use util::SafeUnwrap;
use world::World;

type BrushEventReceiver = tokio::sync::mpsc::Receiver<BrushEvent>;
type BrushEventSender = tokio::sync::mpsc::Sender<BrushEvent>;

struct BrushSystem {
    event_sender: BrushEventSender,
}

impl BrushSystem {
    pub fn new(tx: BrushEventSender) -> Self {
        Self {
            event_sender: tx,
        }
    }
}

impl System<DI> for BrushSystem {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>) {
        event_bus.subscribe(system, handle_click_world_view);
    }
}

#[derive(Debug)]
enum BrushEvent {
    ClickPos(Vec3),
}

fn use_brush_at_position(position: Vec3) -> Result<()> {
    Ok(())
}

fn brush_task(mut recv: BrushEventReceiver) {
    // While the sender is not dropped, we can keep waiting for events
    while let Some(event) = recv.blocking_recv() {
        match event {
            BrushEvent::ClickPos(position) => {
                use_brush_at_position(position).safe_unwrap();
            }
        }
    }
}

fn handle_click_world_view(
    system: &mut BrushSystem,
    _click: &ClickWorldView,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    let di = ctx.read().unwrap();
    let mouse = di.read_sync::<WorldMousePosition>().unwrap();
    match mouse.world_space {
        None => {}
        Some(pos) => {
            system
                .event_sender
                .blocking_send(BrushEvent::ClickPos(pos))?;
        }
    };
    Ok(())
}

pub fn initialize(bus: &EventBus<DI>) {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let system = BrushSystem::new(tx);
    bus.add_system(system);
    tokio::task::spawn_blocking(|| brush_task(rx));
}
