extern crate core;

use anyhow::{bail, Result};
use assets::handle::Handle;
use assets::storage::AssetStorage;
use assets::{Heightmap, NormalMap, Terrain, TerrainOptions};
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
        event_bus.subscribe(system, handle_drag_world_view);
    }
}

#[derive(Debug)]
enum BrushEvent {
    StrokeAt(Vec3),
}

const SIZE: u32 = 256;

fn record_update_normals<'q>(
    cmd: IncompleteCommandBuffer<'q, All>,
    uv: Vec2,
    sampler: &Sampler,
    heights: &Heightmap,
    normals: &NormalMap,
) -> Result<IncompleteCommandBuffer<'q, All>> {
    // Transition the normal map for writing
    let cmd = cmd.transition_image(
        &normals.image.image.view,
        PipelineStage::TOP_OF_PIPE,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::ImageLayout::GENERAL,
        vk::AccessFlags2::NONE,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
    );
    const NORMAL_SIZE: u32 = SIZE + 4;
    let cmd = cmd.bind_compute_pipeline("normal_recompute")?;
    let cmd = cmd
        .bind_storage_image(0, 0, &normals.image.image.view)?
        .bind_sampled_image(0, 1, &heights.image.image.view, sampler)?
        .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &NORMAL_SIZE)
        .dispatch(
            (NORMAL_SIZE as f32 / 16.0f32).ceil() as u32,
            (NORMAL_SIZE as f32 / 16.0f32).ceil() as u32,
            1,
        )?;
    // Transition the normal map back to ShaderReadOnlyOptimal for drawing
    let cmd = cmd.transition_image(
        &normals.image.image.view,
        PipelineStage::COMPUTE_SHADER,
        PipelineStage::BOTTOM_OF_PIPE,
        vk::ImageLayout::GENERAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        vk::AccessFlags2::NONE,
    );
    Ok(cmd)
}

fn record_update_commands(
    cmd: IncompleteCommandBuffer<All>,
    uv: Vec2,
    sampler: &Sampler,
    heights: &Heightmap,
    normals: &NormalMap,
) -> Result<CommandBuffer<All>> {
    // We are going to write to this image in a compute shader, so submit a barrier for this first.
    let cmd = cmd.transition_image(
        &heights.image.image.view,
        PipelineStage::TOP_OF_PIPE,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::ImageLayout::GENERAL,
        vk::AccessFlags2::NONE,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
    );
    // Bind the pipeline we will use to update the heightmap
    let cmd = cmd.bind_compute_pipeline("height_brush")?;
    // Bind the image to the descriptor, push our uvs to the shader and dispatch our compute shader
    let cmd = cmd
        .bind_storage_image(0, 0, &heights.image.image.view)?
        .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &SIZE)
        .dispatch(SIZE / 16, SIZE / 16, 1)?;
    // Transition back to ShaderReadOnlyOptimal for drawing. This also synchronizes access to the heightmap
    // with the normal map calculation shader
    let cmd = cmd.transition_image(
        &heights.image.image.view,
        PipelineStage::COMPUTE_SHADER,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::GENERAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        vk::AccessFlags2::SHADER_SAMPLED_READ,
    );
    let cmd = record_update_normals(cmd, uv, sampler, heights, normals)?;
    cmd.finish()
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

fn height_uv_at(world_pos: Vec3, options: &TerrainOptions) -> Vec2 {
    // First compute outer bounds of the terrain mesh
    let min_x = options.min_x();
    let min_y = options.min_y();
    let max_x = options.max_x();
    let max_y = options.max_y();
    // Then we get the length of the terrain in each dimension
    let dx = (max_x - min_x).abs();
    let dy = (max_y - min_y).abs();
    // Now we can simple calculate the ratio between world_pos and the length in each dimension
    // to get the uvs.
    // Note that we use the z coordinate since y is up, and our terrain is in the flat plane.
    // We will assume our terrain is properly centered. In this case, the uvs we get are
    // in the [-0.5, 0.5] range, so we need to remap them to [0, 1]. Since this range is
    // of the same size, we can just add 0.5
    let uv = Vec2::new(world_pos.x / dx, world_pos.z / dy);
    uv + 0.5
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
    let uv = height_uv_at(position, &world.terrain_options);
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
