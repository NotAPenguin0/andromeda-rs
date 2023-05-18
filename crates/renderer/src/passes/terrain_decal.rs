use std::collections::HashMap;

use anyhow::Result;
use assets::storage::AssetStorage;
use gfx::create_raw_sampler;
use gfx::state::RenderState;
use glam::{Mat4, Quat, Vec3};
use gui::editor::WorldOverlayInfo;
use hot_reload::IntoDynamic;
use inject::DI;
use log::info;
use pass::FrameGraph;
use phobos::{
    vk, Allocator, GraphicsCmdBuffer, PassBuilder, PipelineBuilder, PipelineStage, Sampler,
    VirtualResource,
};
use scheduler::EventBus;
use util::mouse_position::WorldMousePosition;
use world::World;

#[derive(Debug)]
pub struct TerrainDecal {
    bus: EventBus<DI>,
    ctx: gfx::SharedContext,
    depth_sampler: Sampler,
    // Hashmap from frag shader name to pipeline name
    decal_pipelines: HashMap<String, String>,
}

impl TerrainDecal {
    fn new_pipeline(&mut self, shader: &str) -> Result<&str> {
        let name = "terrain_decal_".to_owned() + shader;
        PipelineBuilder::new(name.clone())
            .depth(false, false, false, vk::CompareOp::ALWAYS)
            .blend_additive_unmasked(
                vk::BlendFactor::ONE,
                vk::BlendFactor::DST_ALPHA,
                vk::BlendFactor::ONE,
                vk::BlendFactor::ONE,
            )
            .cull_mask(vk::CullModeFlags::FRONT)
            .samples(vk::SampleCountFlags::TYPE_1)
            .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT])
            .into_dynamic()
            .attach_shader("shaders/src/decal.vs.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader(shader, vk::ShaderStageFlags::FRAGMENT)
            .build(&mut self.bus, self.ctx.pipelines.clone())?;
        self.decal_pipelines.insert(shader.to_owned(), name);
        Ok(self.decal_pipelines.get(shader).unwrap())
    }

    fn get_pipeline(&mut self, shader: &str) -> Result<&str> {
        if self.decal_pipelines.get(shader).is_some() {
            Ok(self.decal_pipelines.get(shader).unwrap())
        } else {
            self.new_pipeline(shader)
        }
    }
}

impl TerrainDecal {
    pub fn new(ctx: gfx::SharedContext, mut bus: EventBus<DI>) -> Result<Self> {
        Ok(Self {
            bus,
            depth_sampler: create_raw_sampler(&ctx)?,
            ctx,
            decal_pipelines: HashMap::default(),
        })
    }
    pub fn render<'cb, A: Allocator>(
        &'cb mut self,
        graph: &mut FrameGraph<'cb, A>,
        color: &VirtualResource,
        depth: &VirtualResource,
        world: &'cb World,
        state: &'cb RenderState,
    ) -> Result<()> {
        let depth = depth.clone();
        let pipeline = {
            let bus = self.bus.clone();
            let di = bus.data().read().unwrap();
            let overlay = di.read_sync::<WorldOverlayInfo>().unwrap();
            let Some(decal) = &overlay.brush_decal else { return Ok(()) };
            self.get_pipeline(&decal.shader)?.to_owned()
        };
        let bus = &self.bus;
        let sampler = &self.depth_sampler;
        let pass = PassBuilder::<_, _, A>::render("terrain_decal")
            .color_attachment(&graph.latest_version(color)?, vk::AttachmentLoadOp::LOAD, None)?
            .sample_image(&graph.latest_version(&depth)?, PipelineStage::FRAGMENT_SHADER)
            .execute_fn(move |cmd, ifc, bindings, stats| {
                let mut cmd = Some(cmd);
                if let Some(terrain) = world.terrain {
                    let di = bus.data().read().unwrap();
                    let assets = di.get::<AssetStorage>().unwrap();
                    match assets
                        .with_if_ready(terrain, |terrain| {
                            terrain.with_if_ready(assets, |heightmap, normal_map, color, mesh| {
                                let mut cmd = cmd.take().unwrap();
                                let mouse = di.read_sync::<WorldMousePosition>().unwrap();
                                let overlay = di.read_sync::<WorldOverlayInfo>().unwrap();
                                let Some(decal) = &overlay.brush_decal else { return Ok(cmd) };
                                let Some(pos) = mouse.world_space else { return Ok(cmd) };
                                let decal_radius_inverse = 1.0 / decal.radius;
                                let transform = Mat4::from_scale_rotation_translation(
                                    Vec3::splat(decal.radius),
                                    Quat::from_rotation_x(90.0f32.to_radians()),
                                    pos,
                                );
                                let to_decal_space =
                                    Mat4::orthographic_rh(-0.5, 0.5, -0.5, 0.5, 0.001, 100.0)
                                        * transform.inverse();
                                let transforms = [
                                    state.projection_view,
                                    state.inverse_projection,
                                    state.inverse_view,
                                    transform,
                                    to_decal_space,
                                ];
                                let mut transform_data = ifc.allocate_scratch_ubo(
                                    std::mem::size_of_val(&transforms) as u64,
                                )?;
                                transform_data.mapped_slice()?.copy_from_slice(&transforms);
                                let mut sizes = ifc.allocate_scratch_ubo(8)?;
                                sizes
                                    .mapped_slice()?
                                    .copy_from_slice(&[decal.radius, decal_radius_inverse]);
                                cmd = cmd
                                    .bind_graphics_pipeline(&pipeline)?
                                    .full_viewport_scissor()
                                    .bind_uniform_buffer(0, 0, &transform_data)?
                                    .resolve_and_bind_sampled_image(
                                        0, 1, &depth, sampler, bindings,
                                    )?
                                    .push_constant(
                                        vk::ShaderStageFlags::FRAGMENT,
                                        0,
                                        &state.render_size,
                                    );
                                match decal.data {
                                    None => {}
                                    Some(data) => {
                                        cmd = cmd.push_constant(
                                            vk::ShaderStageFlags::FRAGMENT,
                                            8,
                                            &data,
                                        );
                                    }
                                }
                                cmd = cmd.draw(36, 1, 0, 0)?;
                                Ok::<_, anyhow::Error>(cmd)
                            })
                        })
                        .flatten()
                    {
                        Some(new_cmd) => cmd = Some(new_cmd?),
                        None => {}
                    }
                }
                Ok(cmd.unwrap())
            })
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}
