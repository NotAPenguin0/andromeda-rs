use anyhow::Result;
use egui::Vec2;
use gfx::create_raw_sampler;
use gfx::state::RenderState;
use glam::{Mat4, Vec4, Vec4Swizzles};
use hot_reload::IntoDynamic;
use inject::DI;
use pass::FrameGraph;
use phobos::wsi::frame::FRAMES_IN_FLIGHT;
use phobos::{
    image, vk, Buffer, BufferView, ComputeCmdBuffer, ComputePipelineBuilder, MemoryType,
    PassBuilder, PipelineStage, Sampler, VirtualResource,
};
use scheduler::EventBus;
use util::mouse_position::WorldMousePosition;
use util::RingBuffer;
use world::World;

use crate::ubo_struct_assign;

#[derive(Debug)]
struct ReadbackData {
    valid: bool,
}

/// Reconstructs the world position of a given coordinate by sampling the depth buffer
#[derive(Debug)]
pub struct WorldPositionReconstruct {
    ctx: gfx::SharedContext,
    sampler: Sampler,
    bus: EventBus<DI>,
    data_buffer: Buffer,
    full_view: BufferView,
    views: RingBuffer<ReadbackData, FRAMES_IN_FLIGHT>,
}

impl WorldPositionReconstruct {
    pub fn new(mut ctx: gfx::SharedContext, bus: &mut EventBus<DI>) -> Result<Self> {
        ComputePipelineBuilder::new("world_pos_reconstruct")
            .into_dynamic()
            .set_shader("shaders/src/world_pos.cs.hlsl")
            .build(bus, ctx.pipelines.clone())?;

        let sampler = create_raw_sampler(&ctx)?;

        const SIZE_OF_ENTRY: u64 = std::mem::size_of::<Vec4>() as u64;
        let data_buffer = Buffer::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            SIZE_OF_ENTRY * FRAMES_IN_FLIGHT as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryType::GpuToCpu,
        )?;

        let views = (0u64..FRAMES_IN_FLIGHT as u64)
            .map(|_| ReadbackData {
                valid: false,
            })
            .collect::<Vec<ReadbackData>>();
        let mut it = views.into_iter();
        let views = [it.next().unwrap(), it.next().unwrap()];
        let views = RingBuffer::new(views);
        let full_view = data_buffer.view_full();

        Ok(WorldPositionReconstruct {
            ctx,
            sampler,
            bus: bus.clone(),
            data_buffer,
            full_view,
            views,
        })
    }

    pub fn render<'cb>(
        &'cb mut self,
        world: &World,
        graph: &mut FrameGraph<'cb>,
        depth: &VirtualResource,
        state: &'cb RenderState,
    ) -> Result<()> {
        let di = self.bus.data().read().unwrap();
        let mut mouse = di.write_sync::<WorldMousePosition>().unwrap();

        self.views.next();
        let cur_idx = self.views.current_index() as u32;
        // This is now definitely safe to access, so we read it back and write it to the mouse position state
        let data = self.views.current_mut();
        // If this data is coming from a valid submission we can read it
        if data.valid {
            let data = self.full_view.mapped_slice::<Vec4>()?;
            let pos = data[cur_idx as usize];
            mouse.world_space = Some(pos.xyz());
            mouse.terrain_uv = Some(world.terrain_options.uv_at(pos.xyz()));
        }

        let mut pass = PassBuilder::new("world_pos_reconstruct")
            .sample_image(&graph.latest_version(depth)?, PipelineStage::COMPUTE_SHADER);

        if let Some(pos) = mouse.screen_space {
            // This data entry is coming from a valid submission
            data.valid = true;
            let sampler = &self.sampler;
            let view = &self.full_view;
            pass = pass.execute_fn(move |cmd, ifc, bindings, _stats| {
                ubo_struct_assign!(camera, ifc,
                struct Camera {
                    inv_projection: Mat4 = state.inverse_projection,
                    inv_view: Mat4 = state.inverse_view,
                });

                cmd.bind_compute_pipeline("world_pos_reconstruct")?
                    .resolve_and_bind_sampled_image(
                        0,
                        0,
                        &image!("resolved_depth"),
                        &sampler,
                        &bindings,
                    )?
                    .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &pos)
                    .push_constant(
                        vk::ShaderStageFlags::COMPUTE,
                        std::mem::size_of::<Vec2>() as u32,
                        &cur_idx,
                    )
                    .bind_storage_buffer(0, 1, view)?
                    .bind_uniform_buffer(0, 2, &camera_buffer)?
                    .dispatch(1, 1, 1)
            })
        } else {
            // We didn't submit anything for this entry, so it is invalid.
            data.valid = false;
        }
        graph.add_pass(pass.build());
        Ok(())
    }
}
