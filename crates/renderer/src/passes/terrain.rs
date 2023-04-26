use anyhow::Result;
use gfx::{create_linear_sampler, create_raw_sampler};
use glam::{Mat4, Vec3};
use hot_reload::IntoDynamic;
use inject::DI;
use ph::vk;
use phobos::prelude as ph;
use phobos::prelude::traits::*;
use scheduler::EventBus;
use statistics::{RendererStatistics, TimedCommandBuffer};
use world::World;

use crate::util::graph::FrameGraph;
use crate::world_renderer::RenderState;

/// The terrain renderer. Stores resources it needs for rendering.
/// This struct renders the main terrain mesh.
#[derive(Debug)]
pub struct TerrainRenderer {
    heightmap_sampler: ph::Sampler,
    linear_sampler: ph::Sampler,
}

impl TerrainRenderer {
    /// Create a new terrain renderer, this will initialize some resources and create
    /// necessary pipelines.
    pub fn new(ctx: gfx::SharedContext, bus: &mut EventBus<DI>) -> Result<Self> {
        ph::PipelineBuilder::new("terrain")
            .samples(vk::SampleCountFlags::TYPE_8)
            .depth(true, true, false, vk::CompareOp::LESS)
            .dynamic_states(&[
                vk::DynamicState::SCISSOR,
                vk::DynamicState::VIEWPORT,
                vk::DynamicState::POLYGON_MODE_EXT,
            ])
            .vertex_input(0, vk::VertexInputRate::VERTEX)
            .vertex_attribute(0, 0, vk::Format::R32G32_SFLOAT)?
            .vertex_attribute(0, 1, vk::Format::R32G32_SFLOAT)?
            .blend_attachment_none()
            .tessellation(4, vk::PipelineTessellationStateCreateFlags::empty())
            .into_dynamic()
            .attach_shader("shaders/src/terrain.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/terrain.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .attach_shader(
                "shaders/src/terrain.hull.hlsl",
                vk::ShaderStageFlags::TESSELLATION_CONTROL,
            )
            .attach_shader(
                "shaders/src/terrain.dom.hlsl",
                vk::ShaderStageFlags::TESSELLATION_EVALUATION,
            )
            .build(bus, ctx.pipelines.clone())?;
        Ok(Self {
            heightmap_sampler: create_raw_sampler(&ctx)?,
            linear_sampler: create_linear_sampler(&ctx)?,
        })
    }

    /// Render the terrain and add all relevant passes to the graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - The frame graph to add the passes to
    /// * `color` - The name of the color attachment to render to. The latest version will be queried from the graph.
    /// * `depth` - The name of the depth attachment to use. The latest version will be queried from the graph.
    /// * `world` - The world state with parameters for rendering.
    /// * `state` - The render state with camera settings and global rendering options.
    pub fn render<'cb, A: Allocator>(
        &'cb mut self,
        graph: &mut FrameGraph<'cb, A>,
        color: &ph::VirtualResource,
        depth: &ph::VirtualResource,
        world: &'cb World,
        state: &'cb RenderState,
    ) -> Result<()> {
        let pass = ph::PassBuilder::<_, _, A>::render("terrain")
            .color_attachment(
                color,
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                }),
            )?
            .depth_attachment(
                depth,
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                }),
            )?
            .execute_fn(|cmd, ifc, _bindings, stats: &mut RendererStatistics| {
                let cmd = cmd.begin_section(stats, "terrain")?;
                let cmd = if let Some(terrain) = world.terrain.value() {
                    let mut cam_ubo =
                        ifc.allocate_scratch_ubo(std::mem::size_of::<Mat4>() as vk::DeviceSize)?;
                    cam_ubo
                        .mapped_slice()?
                        .copy_from_slice(std::slice::from_ref(&state.projection_view));
                    let mut lighting_ubo =
                        ifc.allocate_scratch_ubo(std::mem::size_of::<Vec3>() as vk::DeviceSize)?;
                    lighting_ubo
                        .mapped_slice()?
                        .copy_from_slice(std::slice::from_ref(&state.sun_direction));
                    let tess_factor: u32 = world.options.tessellation_level;
                    cmd.bind_graphics_pipeline("terrain")?
                        .full_viewport_scissor()
                        .push_constant(vk::ShaderStageFlags::TESSELLATION_CONTROL, 0, &tess_factor)
                        .push_constant(
                            vk::ShaderStageFlags::TESSELLATION_EVALUATION,
                            4,
                            &world.terrain_options.vertical_scale,
                        )
                        .bind_uniform_buffer(0, 0, &cam_ubo)?
                        .bind_sampled_image(
                            0,
                            1,
                            &terrain.height_map.image.view,
                            &self.heightmap_sampler,
                        )?
                        .bind_uniform_buffer(0, 2, &lighting_ubo)?
                        .bind_sampled_image(
                            0,
                            3,
                            &terrain.normal_map.image.view,
                            &self.linear_sampler,
                        )?
                        .bind_sampled_image(
                            0,
                            4,
                            &terrain.diffuse_map.image.view,
                            &self.linear_sampler,
                        )?
                        .set_polygon_mode(if world.options.wireframe {
                            vk::PolygonMode::LINE
                        } else {
                            vk::PolygonMode::FILL
                        })?
                        .bind_vertex_buffer(0, &terrain.mesh.vertices_view)
                        .bind_index_buffer(&terrain.mesh.indices_view, vk::IndexType::UINT32)
                        .draw_indexed(terrain.mesh.index_count, 1, 0, 0, 0)?
                } else {
                    cmd
                };
                stats.end_section(cmd, "terrain")
            })
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}
