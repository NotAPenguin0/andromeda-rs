use std::iter::Cycle;

use anyhow::Result;
use gfx::create_raw_sampler;
use gfx::state::RenderState;
use glam::Vec3;
use gui::util::mouse_position::WorldMousePosition;
use hot_reload::IntoDynamic;
use inject::DI;
use pass::FrameGraph;
use phobos::wsi::frame::FRAMES_IN_FLIGHT;
use phobos::{
    vk, Buffer, BufferView, ComputePipelineBuilder, MemoryType, PassBuilder, PipelineStage,
    Sampler, VirtualResource,
};
use scheduler::EventBus;
use util::RingBuffer;

#[derive(Debug)]
struct ReadbackData {
    valid: bool,
    view: BufferView,
}

/// Reconstructs the world position of a given coordinate by sampling the depth buffer
#[derive(Debug)]
pub struct WorldPositionReconstruct {
    ctx: gfx::SharedContext,
    sampler: Sampler,
    bus: EventBus<DI>,
    data_buffer: Buffer,
    views: RingBuffer<ReadbackData, FRAMES_IN_FLIGHT>,
}

impl WorldPositionReconstruct {
    pub fn new(mut ctx: gfx::SharedContext, bus: &mut EventBus<DI>) -> Result<Self> {
        ComputePipelineBuilder::new("world_pos_reconstruct")
            .into_dynamic()
            .set_shader("shaders/src/world_pos.cs.hlsl")
            .build(bus, ctx.pipelines.clone())?;

        let sampler = create_raw_sampler(&ctx)?;

        const SIZE_OF_ENTRY: u64 = std::mem::size_of::<Vec3>() as u64;
        let data_buffer = Buffer::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            SIZE_OF_ENTRY * FRAMES_IN_FLIGHT as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryType::GpuToCpu,
        )?;

        let views = (0u64..FRAMES_IN_FLIGHT as u64)
            .map(|i| {
                let offset = i * SIZE_OF_ENTRY;
                data_buffer.view(offset, SIZE_OF_ENTRY)
            })
            .map(|view| {
                Ok(ReadbackData {
                    valid: false,
                    view: view?,
                })
            })
            .collect::<Result<Vec<ReadbackData>>>()?;
        let mut it = views.into_iter();
        let views = [it.next().unwrap(), it.next().unwrap()];
        let views = RingBuffer::new(views);

        Ok(WorldPositionReconstruct {
            ctx,
            sampler,
            bus: bus.clone(),
            data_buffer,
            views,
        })
    }

    pub fn render<'cb>(
        &'cb mut self,
        graph: &mut FrameGraph<'cb>,
        depth: &VirtualResource,
        state: &'cb RenderState,
    ) -> Result<()> {
        let di = self.bus.data().read().unwrap();
        let mut mouse = di.write_sync::<WorldMousePosition>().unwrap();

        self.views.next();
        // This is now definitely safe to access, so we read it back and write it to the mouse position state
        let data = self.views.current_mut();
        // If this data is coming from a valid submission we can read it
        mouse.world_space = if data.valid {
            let data = data.view.mapped_slice::<Vec3>()?.first().unwrap();
            Some(*data)
        } else {
            None
        };

        if let Some(pos) = mouse.screen_space {
            // This data entry is coming from a valid submisison
            data.valid = true;
            let pass = PassBuilder::new("world_pos_reconstruct")
                .sample_image(depth, PipelineStage::COMPUTE_SHADER)
                .execute_fn(|cmd, ifc, bindings, stats| Ok(cmd))
                .build();
            graph.add_pass(pass);
        } else {
            // We didn't submit anything for this entry, so it is invalid.
            data.valid = false;
        }
        Ok(())
    }
}
