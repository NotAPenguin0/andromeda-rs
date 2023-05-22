use anyhow::Result;
use gfx::state::RenderState;
use glam::{Mat4, Vec3Swizzles, Vec4};
use hot_reload::IntoDynamic;
use inject::DI;
use pass::FrameGraph;
use ph::vk;
use phobos as ph;
use phobos::{Allocator, GraphicsCmdBuffer};
use scheduler::EventBus;
use statistics::{RendererStatistics, TimedCommandBuffer};
use world::World;

use crate::{ubo_struct, ubo_struct_assign};

/// The atmosphere renderer is responsible for rendering the
/// atmosphere into the frame graph.
#[allow(dead_code)]
#[derive(Debug)]
pub struct AtmosphereRenderer {
    ctx: gfx::SharedContext,
}

impl AtmosphereRenderer {
    /// Create a new atmosphere renderer. This will initialize pipelines and other resources it needs.
    pub fn new(ctx: gfx::SharedContext, bus: &mut EventBus<DI>) -> Result<Self> {
        ph::PipelineBuilder::new("atmosphere")
            .depth(true, false, false, vk::CompareOp::LESS_OR_EQUAL)
            .cull_mask(vk::CullModeFlags::NONE)
            .blend_additive_unmasked(
                vk::BlendFactor::ONE,
                vk::BlendFactor::ONE,
                vk::BlendFactor::ONE,
                vk::BlendFactor::ONE,
            )
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .samples(vk::SampleCountFlags::TYPE_8) // TODO: config, sample shading
            .into_dynamic()
            .attach_shader("shaders/src/fullscreen.vs.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/atmosphere.fs.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(bus, ctx.pipelines.clone())?;

        Ok(AtmosphereRenderer {
            ctx,
        })
    }

    /// Render the atmosphere and add all relevant passes to the graph.
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
        let pass = ph::PassBuilder::<_, _, A>::render("atmosphere")
            .color_attachment(&graph.latest_version(color)?, vk::AttachmentLoadOp::LOAD, None)?
            .depth_attachment(&graph.latest_version(depth)?, vk::AttachmentLoadOp::LOAD, None)?
            .execute_fn(|mut cmd, ifc, _bindings, stats: &mut RendererStatistics| {
                ubo_struct_assign!(
                    camera,
                    ifc,
                    struct Camera {
                        pv: Mat4 = state.projection_view,
                        inv_proj: Mat4 = state.inverse_projection,
                        inv_view_rotation: Mat4 = state.inverse_view_rotation,
                        cam_pos: Vec4 = state.cam_position.xyzx(),
                    }
                );

                ubo_struct_assign!(
                    atmosphere,
                    ifc,
                    struct Atmosphere {
                        radii_mie_albedo_g: Vec4 = Vec4::new(
                            world.atmosphere.planet_radius,
                            world.atmosphere.atmosphere_radius,
                            world.atmosphere.mie_albedo,
                            world.atmosphere.mie_g,
                        ),
                        rayleigh: Vec4 = Vec4::from((
                            world.atmosphere.rayleigh_coefficients,
                            world.atmosphere.rayleigh_scatter_height,
                        )),
                        mie: Vec4 = Vec4::from((
                            world.atmosphere.mie_coefficients,
                            world.atmosphere.mie_scatter_height,
                        )),
                        ozone_sun: Vec4 = Vec4::from((
                            world.atmosphere.ozone_coefficients,
                            world.atmosphere.sun_intensity,
                        )),
                    }
                );

                let pc = Vec4::from((state.sun_direction, 0.0));

                cmd = cmd
                    .begin_section(stats, "atmosphere")?
                    .bind_graphics_pipeline("atmosphere")?
                    .full_viewport_scissor()
                    .bind_uniform_buffer(0, 0, &camera_buffer)?
                    .bind_uniform_buffer(0, 1, &atmosphere_buffer)?
                    .push_constant(vk::ShaderStageFlags::FRAGMENT, 0, &pc)
                    .draw(6, 1, 0, 0)?
                    .end_section(stats, "atmosphere")?;
                Ok(cmd)
            })
            .build();

        graph.add_pass(pass);
        Ok(())
    }
}
