use anyhow::Result;
use assets::storage::AssetStorage;
use gfx::state::RenderState;
use gfx::{create_linear_sampler, create_raw_sampler};
use glam::{Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};
use gui::editor::WorldOverlayInfo;
use hot_reload::IntoDynamic;
use inject::DI;
use pass::FrameGraph;
use ph::vk;
use phobos::prelude as ph;
use phobos::prelude::traits::*;
use scheduler::EventBus;
use statistics::{RendererStatistics, TimedCommandBuffer};
use util::mouse_position::WorldMousePosition;
use world::World;

/// The terrain renderer. Stores resources it needs for rendering.
/// This struct renders the main terrain mesh.
#[derive(Debug)]
pub struct TerrainRenderer {
    heightmap_sampler: ph::Sampler,
    linear_sampler: ph::Sampler,
    bus: EventBus<DI>,
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
            .attach_shader("shaders/src/terrain.vs.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/terrain.fs.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .attach_shader(
                "shaders/src/terrain.hs.hlsl",
                vk::ShaderStageFlags::TESSELLATION_CONTROL,
            )
            .attach_shader(
                "shaders/src/terrain.ds.hlsl",
                vk::ShaderStageFlags::TESSELLATION_EVALUATION,
            )
            .build(bus, ctx.pipelines.clone())?;
        Ok(Self {
            heightmap_sampler: create_raw_sampler(&ctx)?,
            linear_sampler: create_linear_sampler(&ctx)?,
            bus: bus.clone(),
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
                let di = self.bus.data().read().unwrap();
                let assets = di.get::<AssetStorage>().unwrap();
                let mut cmd = Some(cmd.begin_section(stats, "terrain")?);
                if let Some(terrain) = world.terrain {
                    match assets
                        .with_if_ready(terrain, |terrain| {
                            terrain.with_if_ready(assets, |heightmap, normal_map, color, mesh| {
                                let mut cam_ubo = ifc.allocate_scratch_ubo(
                                    std::mem::size_of::<Mat4>() as vk::DeviceSize,
                                )?;
                                cam_ubo
                                    .mapped_slice()?
                                    .copy_from_slice(std::slice::from_ref(&state.projection_view));
                                let mut lighting_ubo = ifc.allocate_scratch_ubo(
                                    std::mem::size_of::<Vec4>() as vk::DeviceSize,
                                )?;
                                lighting_ubo
                                    .mapped_slice()?
                                    .copy_from_slice(std::slice::from_ref(&state.sun_direction));
                                let tess_factor: u32 = world.options.tessellation_level;
                                let cmd = cmd
                                    .take()
                                    .unwrap()
                                    .bind_graphics_pipeline("terrain")?
                                    .full_viewport_scissor()
                                    .push_constant(
                                        vk::ShaderStageFlags::TESSELLATION_CONTROL,
                                        0,
                                        &tess_factor,
                                    )
                                    .push_constant(
                                        vk::ShaderStageFlags::TESSELLATION_EVALUATION,
                                        4,
                                        &world.terrain_options.vertical_scale,
                                    )
                                    .bind_uniform_buffer(0, 0, &cam_ubo)?
                                    .bind_sampled_image(
                                        0,
                                        1,
                                        &heightmap.image.image.view,
                                        &self.heightmap_sampler,
                                    )?
                                    .bind_uniform_buffer(0, 2, &lighting_ubo)?
                                    .bind_sampled_image(
                                        0,
                                        3,
                                        &normal_map.image.image.view,
                                        &self.linear_sampler,
                                    )?
                                    .bind_sampled_image(
                                        0,
                                        4,
                                        &color.image.view,
                                        &self.linear_sampler,
                                    )?
                                    .set_polygon_mode(if world.options.wireframe {
                                        vk::PolygonMode::LINE
                                    } else {
                                        vk::PolygonMode::FILL
                                    })?
                                    .bind_vertex_buffer(0, &mesh.vertices_view)
                                    .bind_index_buffer(&mesh.indices_view, vk::IndexType::UINT32)
                                    .draw_indexed(mesh.index_count, 1, 0, 0, 0)?;
                                Ok::<_, anyhow::Error>(cmd)
                            })
                        })
                        .flatten()
                    {
                        None => {}
                        Some(new_cmd) => cmd = Some(new_cmd?),
                    }
                }
                let cmd = cmd.unwrap();
                stats.end_section(cmd, "terrain")
            })
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}
