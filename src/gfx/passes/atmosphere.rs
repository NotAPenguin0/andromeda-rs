use anyhow::Result;
use glam::{Mat4, Vec3, Vec3Swizzles, Vec4};
use ph::vk;
use phobos as ph;
use phobos::GraphicsCmdBuffer;

use crate::app::RootActorSystem;
use crate::gfx;
use crate::hot_reload::IntoDynamic;

#[derive(Debug, Default, Copy, Clone)]
pub struct AtmosphereInfo {
    pub planet_radius: f32,
    pub atmosphere_radius: f32,
    pub rayleigh_coefficients: Vec3,
    pub rayleigh_scatter_height: f32,
    pub mie_coefficients: Vec3,
    pub mie_albedo: f32,
    pub mie_scatter_height: f32,
    pub mie_g: f32,
    pub ozone_coefficients: Vec3,
    pub sun_intensity: f32,
}

impl AtmosphereInfo {
    /// Returns earth-like atmosphere parameters
    pub fn earth() -> Self {
        Self {
            planet_radius: 6371000.0,
            atmosphere_radius: 6471000.0,
            rayleigh_coefficients: Vec3::new(0.0000058, 0.0000133, 0.00003331),
            rayleigh_scatter_height: 8000.0,
            mie_coefficients: Vec3::new(0.000021, 0.000021, 0.000021),
            mie_albedo: 0.9,
            mie_scatter_height: 1200.0,
            mie_g: 0.8,
            ozone_coefficients: Vec3::new(0.00000077295962, 0.000000667717648, 0.0000000704931588),
            sun_intensity: 22.0,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AtmosphereRenderer {
    ctx: gfx::SharedContext,
}

impl AtmosphereRenderer {
    pub fn new(ctx: gfx::SharedContext, actors: &RootActorSystem) -> Result<Self> {
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
            .build(actors.shader_reload.clone(), ctx.pipelines.clone())?;

        Ok(AtmosphereRenderer {
            ctx,
        })
    }

    pub async fn render<'s: 'e + 'q, 'state: 'e + 'q, 'e, 'q>(
        &'s mut self,
        graph: &mut gfx::FrameGraph<'e, 'q>,
        _bindings: &mut ph::PhysicalResourceBindings,
        color: ph::VirtualResource,
        depth: ph::VirtualResource,
        state: &'state gfx::RenderState,
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

                let mut camera = ifc.allocate_scratch_ubo(std::mem::size_of::<Camera>() as vk::DeviceSize)?;
                let camera_data = camera.mapped_slice::<Camera>()?;
                let mut camera_data = camera_data.get_mut(0).unwrap();
                camera_data.pv = state.projection_view;
                camera_data.inv_proj = state.inverse_projection;
                camera_data.inv_view_rotation = state.inverse_view_rotation;
                camera_data.cam_pos = state.position.xyzx(); // last component does not matter

                #[repr(C)]
                struct Atmosphere {
                    radii_mie_albedo_g: Vec4,
                    rayleigh: Vec4,
                    mie: Vec4,
                    ozone_sun: Vec4,
                }
                // TODO: Macro magic to make this more convenient?
                let mut atmosphere = ifc.allocate_scratch_ubo(std::mem::size_of::<Atmosphere>() as vk::DeviceSize)?;
                let atmosphere_data = atmosphere.mapped_slice::<Atmosphere>()?;
                let mut atmosphere_data = atmosphere_data.get_mut(0).unwrap();
                atmosphere_data.radii_mie_albedo_g = Vec4::new(
                    state.atmosphere.planet_radius,
                    state.atmosphere.atmosphere_radius,
                    state.atmosphere.mie_albedo,
                    state.atmosphere.mie_g,
                );
                atmosphere_data.rayleigh = Vec4::from((state.atmosphere.rayleigh_coefficients, state.atmosphere.rayleigh_scatter_height));
                atmosphere_data.mie = Vec4::from((state.atmosphere.mie_coefficients, state.atmosphere.mie_scatter_height));
                atmosphere_data.ozone_sun = Vec4::from((state.atmosphere.ozone_coefficients, state.atmosphere.sun_intensity));

                let pc = Vec4::from((state.sun_dir, 0.0));

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
