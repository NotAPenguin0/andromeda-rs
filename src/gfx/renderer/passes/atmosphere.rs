use anyhow::Result;
use glam::{Mat4, Vec3Swizzles, Vec4};
use ph::vk;
use phobos as ph;
use phobos::{Allocator, GraphicsCmdBuffer};

use crate::gfx;
use crate::gfx::renderer::world_renderer::RenderState;
use crate::gfx::util::graph::FrameGraph;
use crate::hot_reload::IntoDynamic;
use crate::state::world::World;

#[allow(dead_code)]
#[derive(Debug)]
pub struct AtmosphereRenderer {
    ctx: gfx::SharedContext,
}

impl AtmosphereRenderer {
    pub fn new(ctx: gfx::SharedContext) -> Result<Self> {
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
            .attach_shader("shaders/src/fullscreen.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/atmosphere.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(ctx.shader_reload.clone(), ctx.pipelines.clone())?;

        Ok(AtmosphereRenderer {
            ctx,
        })
    }

    pub fn render<'s: 'e + 'q, 'state: 'e + 'q, 'world: 'e + 'q, 'e, 'q, A: Allocator>(
        &'s mut self,
        graph: &mut FrameGraph<'e, 'q, A>,
        _bindings: &mut ph::PhysicalResourceBindings,
        color: &ph::VirtualResource,
        depth: &ph::VirtualResource,
        world: &'world World,
        state: &'state RenderState,
    ) -> Result<()> {
        let pass = ph::PassBuilder::render("atmosphere")
            .color_attachment(&graph.latest_version(color)?, vk::AttachmentLoadOp::LOAD, None)?
            .depth_attachment(&graph.latest_version(depth)?, vk::AttachmentLoadOp::LOAD, None)?
            .execute(|mut cmd, ifc, _bindings| {
                #[repr(C)]
                struct Camera {
                    pv: Mat4,
                    inv_proj: Mat4,
                    inv_view_rotation: Mat4,
                    cam_pos: Vec4,
                }

                let mut camera =
                    ifc.allocate_scratch_ubo(std::mem::size_of::<Camera>() as vk::DeviceSize)?;
                let camera_data = camera.mapped_slice::<Camera>()?;
                let mut camera_data = camera_data.get_mut(0).unwrap();
                camera_data.pv = state.projection_view;
                camera_data.inv_proj = state.inverse_projection;
                camera_data.inv_view_rotation = state.inverse_view_rotation;
                camera_data.cam_pos = state.cam_position.xyzx(); // last component does not matter

                #[repr(C)]
                struct Atmosphere {
                    radii_mie_albedo_g: Vec4,
                    rayleigh: Vec4,
                    mie: Vec4,
                    ozone_sun: Vec4,
                }
                // TODO: Macro magic to make this more convenient?
                let mut atmosphere =
                    ifc.allocate_scratch_ubo(std::mem::size_of::<Atmosphere>() as vk::DeviceSize)?;
                let atmosphere_data = atmosphere.mapped_slice::<Atmosphere>()?;
                let mut atmosphere_data = atmosphere_data.get_mut(0).unwrap();
                atmosphere_data.radii_mie_albedo_g = Vec4::new(
                    world.atmosphere.planet_radius,
                    world.atmosphere.atmosphere_radius,
                    world.atmosphere.mie_albedo,
                    world.atmosphere.mie_g,
                );
                atmosphere_data.rayleigh = Vec4::from((
                    world.atmosphere.rayleigh_coefficients,
                    world.atmosphere.rayleigh_scatter_height,
                ));
                atmosphere_data.mie = Vec4::from((
                    world.atmosphere.mie_coefficients,
                    world.atmosphere.mie_scatter_height,
                ));
                atmosphere_data.ozone_sun = Vec4::from((
                    world.atmosphere.ozone_coefficients,
                    world.atmosphere.sun_intensity,
                ));

                let pc = Vec4::from((state.sun_direction, 0.0));

                cmd = cmd
                    .bind_graphics_pipeline("atmosphere")?
                    .full_viewport_scissor()
                    .bind_uniform_buffer(0, 0, &camera)?
                    .bind_uniform_buffer(0, 1, &atmosphere)?
                    .push_constants(vk::ShaderStageFlags::FRAGMENT, 0, std::slice::from_ref(&pc))
                    .draw(6, 1, 0, 0)?;

                Ok(cmd)
            })
            .build();

        graph.add_pass(pass);
        Ok(())
    }
}
