use anyhow::Result;
use assets::storage::AssetStorage;
use events::ClickWorldView;
use glam::{Vec2, Vec3};
use inject::DI;
use log::{log, trace};
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

fn height_uv_at(world_pos: Vec3, scale: f32) -> Vec2 {
    todo!()
}

fn use_brush_at_position(bus: &EventBus<DI>, position: Vec3) -> Result<()> {
    // If any of the values inside the position are NaN or infinite, the position is outside
    // of the rendered terrain mesh and we do not want to actually use the brush.
    if position.is_nan() || !position.is_finite() {
        return Ok(());
    }

    let di = bus.data().read().unwrap();
    let world = di.read_sync::<World>().unwrap();

    // We will apply our brush mainly to the heightmap texture for now. To know how
    // to do this, we need to find the UV coordinates of the heightmap texture
    // at the position we clicked at.
    let scale = world.terrain_options.horizontal_scale;
    let uv = height_uv_at(position, scale);
    Ok(())
}

fn brush_task(bus: EventBus<DI>, mut recv: BrushEventReceiver) {
    // While the sender is not dropped, we can keep waiting for events
    while let Some(event) = recv.blocking_recv() {
        match event {
            BrushEvent::ClickPos(position) => {
                use_brush_at_position(&bus, position).safe_unwrap();
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

pub fn initialize(bus: &EventBus<DI>) -> Result<()> {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let system = BrushSystem::new(tx);
    bus.add_system(system);
    let bus = bus.clone();
    std::thread::Builder::new()
        .name("brush-thread".into())
        .spawn(|| brush_task(bus, rx))?;
    Ok(())
}
