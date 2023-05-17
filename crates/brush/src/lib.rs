use ::util::mouse_position::WorldMousePosition;
use ::util::SafeUnwrap;
use anyhow::{bail, Result};
use assets::storage::AssetStorage;
use assets::{NormalMap, Terrain, TerrainOptions};
use events::DragOnWorldView;
use gfx::{create_linear_sampler, SharedContext};
use glam::{Vec2, Vec3};
use hot_reload::IntoDynamic;
use inject::DI;
use log::{info, trace};
use pass::GpuWork;
use phobos::domain::All;
use phobos::{
    vk, CommandBuffer, ComputeCmdBuffer, ComputePipelineBuilder, IncompleteCmdBuffer,
    IncompleteCommandBuffer, PipelineStage, Sampler,
};
use scheduler::{EventBus, EventContext, StoredSystem, System};
use world::World;

use crate::brushes::height::record_update_commands;
use crate::util::terrain_uv_at;

pub mod brushes;
pub mod util;

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
        event_bus.subscribe(system, handle_drag_world_view);
    }
}

#[derive(Debug)]
enum BrushEvent {
    StrokeAt(Vec3),
}

fn update_heightmap(uv: Vec2, bus: &EventBus<DI>, sampler: &Sampler) -> Result<()> {
    let di = bus.data().read().unwrap();
    let terrain_handle = {
        let world = di.read_sync::<World>().unwrap();
        world.terrain
    };
    // If no terrain handle was set, we cannot reasonably use a brush on it
    let Some(terrain_handle) = terrain_handle else { bail!("Used brush but terrain handle is not set.") };
    // Get the asset system so we can wait until the terrain is loaded.
    // Note that this should usually complete quickly, since without a loaded
    // terrain we cannot use a brush.
    let assets = di.get::<AssetStorage>().unwrap();
    assets
        .with_when_ready(terrain_handle, |terrain| {
            terrain.with_when_ready(bus, |heights, normals, _, _| {
                // Get the graphics context and allocate a command buffer
                let ctx = di.get::<SharedContext>().cloned().unwrap();
                let cmd = ctx.exec.on_domain::<All, _>(
                    Some(ctx.pipelines.clone()),
                    Some(ctx.descriptors.clone()),
                )?;
                let cmd = record_update_commands(cmd, uv, sampler, heights, normals)?;
                // Submit our commands once a batch is ready
                GpuWork::with_batch(bus, move |batch| batch.submit(cmd))??;
                Ok::<_, anyhow::Error>(())
            })
        })
        .flatten()
        .unwrap_or(Ok(()))?;
    Ok(())
}

fn use_brush_at_position(
    bus: &EventBus<DI>,
    position: Vec3,
    height_sampler: &Sampler,
) -> Result<()> {
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
    let uv = terrain_uv_at(position, &world.terrain_options);
    update_heightmap(uv, bus, height_sampler)?;
    Ok(())
}

fn brush_task(bus: EventBus<DI>, mut recv: BrushEventReceiver) {
    let sampler = {
        let di = bus.data().read().unwrap();
        let gfx = di.get::<SharedContext>().cloned().unwrap();
        create_linear_sampler(&gfx).unwrap()
    };
    // While the sender is not dropped, we can keep waiting for events
    while let Some(event) = recv.blocking_recv() {
        match event {
            BrushEvent::StrokeAt(position) => {
                use_brush_at_position(&bus, position, &sampler).safe_unwrap();
            }
        }
    }
}

fn handle_drag_world_view(
    system: &mut BrushSystem,
    _drag: &DragOnWorldView,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    let di = ctx.read().unwrap();
    let mouse = di.read_sync::<WorldMousePosition>().unwrap();
    match mouse.world_space {
        None => {}
        Some(pos) => {
            system
                .event_sender
                .blocking_send(BrushEvent::StrokeAt(pos))?;
        }
    };
    Ok(())
}

fn create_brush_pipeline(bus: &EventBus<DI>) -> Result<()> {
    let di = bus.data().read().unwrap();
    let gfx = di.get::<SharedContext>().cloned().unwrap();
    ComputePipelineBuilder::new("height_brush")
        // Make sure this pipeline is persistent so we don't constantly recompile it
        .persistent()
        .into_dynamic()
        .set_shader("shaders/src/height_brush.cs.hlsl")
        .build(bus, gfx.pipelines.clone())?;
    ComputePipelineBuilder::new("normal_recompute")
        .persistent()
        .into_dynamic()
        .set_shader("shaders/src/normal_recompute.cs.hlsl")
        .build(bus, gfx.pipelines.clone())?;
    ComputePipelineBuilder::new("blur_rect")
        .persistent()
        .into_dynamic()
        .set_shader("shaders/src/blur_rect.cs.hlsl")
        .build(bus, gfx.pipelines)?;
    Ok(())
}

pub fn initialize(bus: &EventBus<DI>) -> Result<()> {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let system = BrushSystem::new(tx);
    bus.add_system(system);
    create_brush_pipeline(bus)?;
    let bus = bus.clone();
    tokio::task::spawn_blocking(|| brush_task(bus, rx));
    Ok(())
}
