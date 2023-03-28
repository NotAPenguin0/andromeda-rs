use std::rc::Rc;

use anyhow::Result;
use glam::{Mat3, Mat4, Vec3};
use phobos as ph;
use phobos::vk;

use crate::gfx;
use crate::gfx::passes::AtmosphereInfo;
use crate::gfx::resource::height_map::HeightMap;
use crate::gfx::resource::TerrainPlane;
use crate::gfx::targets::{RenderTargets, SizeGroup};
use crate::gfx::world::World;
use crate::gfx::{passes, postprocess};
use crate::hot_reload::{IntoDynamic, SyncShaderReload};

#[derive(Debug)]
pub struct RenderOptions {
    pub tessellation_level: u32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            tessellation_level: 8,
        }
    }
}

#[derive(Debug, Default)]
pub struct RenderState {
    pub view: Mat4,
    pub projection: Mat4,
    pub projection_view: Mat4,
    pub inverse_projection: Mat4,
    pub inverse_projection_view: Mat4,
    pub inverse_view_rotation: Mat4,
    pub position: Vec3,
    pub atmosphere: AtmosphereInfo,
    pub sun_dir: Vec3,
    pub terrain_mesh: Option<Rc<TerrainPlane>>,
    pub height_map: Option<Rc<HeightMap>>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct WorldRenderer {
    pub targets: RenderTargets,
    ctx: gfx::SharedContext,
    state: RenderState,
    tonemap: postprocess::Tonemap,
    atmosphere: passes::AtmosphereRenderer,
    terrain: passes::TerrainRenderer,
}

impl WorldRenderer {
    pub fn new(reload: SyncShaderReload, ctx: gfx::SharedContext) -> Result<Self> {
        ph::PipelineBuilder::new("flat_draw")
            .vertex_input(0, vk::VertexInputRate::VERTEX)
            .vertex_attribute(0, 0, vk::Format::R32G32B32_SFLOAT)?
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .blend_attachment_none()
            .depth(true, true, false, vk::CompareOp::LESS)
            .cull_mask(vk::CullModeFlags::NONE)
            .samples(vk::SampleCountFlags::TYPE_8) // TODO: Config, etc.
            .into_dynamic()
            .attach_shader("shaders/src/simple_mesh.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/solid_color.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(reload.clone(), ctx.pipelines.clone())?;

        let mut targets = RenderTargets::new()?;
        targets.set_output_resolution(1, 1)?;

        targets.register_multisampled_color_target(
            "scene_output",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::SampleCountFlags::TYPE_8,
        )?;

        targets.register_multisampled_depth_target(
            "depth",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::Format::D32_SFLOAT,
            vk::SampleCountFlags::TYPE_8,
        )?;

        targets.register_color_target(
            "resolved_output",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R32G32B32A32_SFLOAT,
        )?;

        Ok(Self {
            ctx: ctx.clone(),
            state: RenderState::default(),
            tonemap: postprocess::Tonemap::new(ctx.clone(), &reload, &mut targets)?,
            atmosphere: passes::AtmosphereRenderer::new(ctx.clone(), &reload)?,
            terrain: passes::TerrainRenderer::new(ctx.clone(), &reload)?,
            targets,
        })
    }

    pub fn output_name() -> &'static str {
        postprocess::Tonemap::output_name()
    }

    pub fn output_image(&self) -> ph::ImageView {
        self.targets.get_target_view(Self::output_name()).unwrap()
    }

    pub fn new_frame(&mut self) {
        self.targets.next_frame();
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.output_image().width() as f32 / self.output_image().height() as f32
    }

    fn update_render_state(&mut self, world: &World) -> Result<()> {
        let camera = world.camera.read().unwrap();
        self.state.view = camera.matrix();
        self.state.projection = Mat4::perspective_rh(camera.fov().to_radians(), self.aspect_ratio(), 0.1, 10000000.0);
        // Flip y because Vulkan
        let v = self.state.projection.col_mut(1).y;
        self.state.projection.col_mut(1).y = v * -1.0;
        self.state.position = camera.position().0;
        self.state.projection_view = self.state.projection * self.state.view;
        self.state.inverse_projection_view = self.state.projection_view.inverse();
        self.state.inverse_projection = self.state.projection.inverse();
        self.state.inverse_view_rotation = Mat4::from_mat3(Mat3::from_mat4(self.state.view)).inverse();
        self.state.atmosphere = world.atmosphere;
        self.state.sun_dir = -world.sun_direction.front_direction();
        self.state.terrain_mesh = world.terrain_mesh.clone();
        self.state.height_map = world.height_map.clone();
        Ok(())
    }

    pub async fn redraw_world<'s: 'e + 'q, 'world: 'e + 'q, 'q, 'e>(
        &'s mut self,
        world: &'world World,
    ) -> Result<(gfx::FrameGraph<'e, 'q>, ph::PhysicalResourceBindings)> {
        let mut bindings = ph::PhysicalResourceBindings::new();
        let mut graph = gfx::FrameGraph::new();
        self.targets.bind_targets(&mut bindings);

        self.update_render_state(world)?;

        let scene_output = ph::VirtualResource::image("scene_output");
        let depth = ph::VirtualResource::image("depth");
        let resolved_output = ph::VirtualResource::image("resolved_output");
        let tonemapped_output = ph::VirtualResource::image(postprocess::Tonemap::output_name());

        // 1. Render terrain
        self.terrain
            .render(&world.options, &mut graph, &mut bindings, &scene_output, &depth, &self.state)
            .await?;
        // 2. Render atmosphere
        self.atmosphere
            .render(&mut graph, &mut bindings, &scene_output, &depth, &self.state)
            .await?;
        // 3. Resolve MSAA
        let resolve = ph::PassBuilder::render("msaa_resolve")
            .color_attachment(&graph.latest_version(&scene_output)?, vk::AttachmentLoadOp::LOAD, None)?
            // We dont currently need depth resolved
            // .depth_attachment(graph.latest_version(depth.clone())?,vk::AttachmentLoadOp::LOAD, None)?
            .resolve(&graph.latest_version(&scene_output)?, &resolved_output)
            .build();
        graph.add_pass(resolve);
        // 4. Apply tonemapping
        self.tonemap.render(&resolved_output, &mut graph)?;
        // 5. Alias our final result to the expected name
        graph.alias("renderer_output", tonemapped_output);

        Ok((graph, bindings))
    }
}
