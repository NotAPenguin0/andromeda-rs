use anyhow::Result;
use camera::CameraState;
use gfx::state::{RenderState, SceneResources};
use gfx::SharedContext;
use glam::{Mat3, Mat4};
use gui::util::image_provider::ImageProvider;
use hot_reload::IntoDynamic;
use inject::DI;
use pass::{FrameGraph, GpuWork};
use phobos::{vk, PassBuilder, PhysicalResourceBindings, PipelineBuilder, VirtualResource};
use scheduler::EventBus;
use world::World;

use crate::passes::atmosphere::AtmosphereRenderer;
use crate::passes::terrain::TerrainRenderer;
use crate::postprocess::tonemap::Tonemap;
use crate::ui_integration::UIIntegration;
use crate::util::targets::{RenderTargets, SizeGroup};

/// The world renderer is responsible for all the rendering logic
/// of the scene.
#[derive(Debug)]
pub struct WorldRenderer {
    bus: EventBus<DI>,
    tonemap: Tonemap,
    atmosphere: AtmosphereRenderer,
    terrain: TerrainRenderer,
    state: RenderState,
}

impl WorldRenderer {
    /// Initialize the world renderer.
    /// This will create pipelines, initialize render targets and create
    /// other necessary objects.
    pub fn new(ctx: SharedContext, mut bus: EventBus<DI>) -> Result<Self> {
        PipelineBuilder::new("flat_draw")
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
            .build(&mut bus, ctx.pipelines.clone())?;

        let mut targets = RenderTargets::new(ctx.clone())?;
        targets.set_output_resolution(1, 1)?;

        targets.register_multisampled_color_target(
            "scene_output",
            SizeGroup::OutputResolution,
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::SampleCountFlags::TYPE_8,
        )?;

        targets.register_multisampled_depth_target(
            "depth",
            SizeGroup::OutputResolution,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::Format::D32_SFLOAT,
            vk::SampleCountFlags::TYPE_8,
        )?;

        targets.register_color_target(
            "resolved_output",
            SizeGroup::OutputResolution,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R32G32B32A32_SFLOAT,
        )?;

        let state = RenderState::default();
        let tonemap = Tonemap::new(ctx.clone(), &mut targets, &mut bus)?;

        {
            let mut inject = bus.data().write().unwrap();
            inject.put_sync(targets);
        }

        Ok(Self {
            tonemap,
            atmosphere: AtmosphereRenderer::new(ctx.clone(), &mut bus)?,
            terrain: TerrainRenderer::new(ctx, &mut bus)?,
            bus,
            state,
        })
    }

    /// Name of the rendertarget that is the final output of the
    /// scene rendering.
    pub fn output_name() -> &'static str {
        Tonemap::output_name()
    }

    /// Updates the output image used in the UI to have the correct size.
    /// # DI Access
    /// - Write [`RenderTargets`]
    /// - Write [`ImageProvider`]
    pub fn update_output_image(&mut self, ui: &mut UIIntegration) -> Result<()> {
        let inject = self.bus.data().read().unwrap();
        let mut targets = inject.write_sync::<RenderTargets>().unwrap();
        let mut provider = inject.write_sync::<ImageProvider>().unwrap();
        targets.set_output_resolution(provider.size.x(), provider.size.y())?;
        // Then grab our color output.
        let image = targets.get_target_view(Self::output_name()).unwrap();
        // We can re-register the same image, nothing will happen.
        let handle = ui.register_texture(&image);
        provider.handle = Some(handle);
        Ok(())
    }

    /// Update deferred deletion queues.
    /// # DI Access
    /// - Write [`RenderTargets`]
    pub fn new_frame(&mut self) {
        let inject = self.bus.data().read().unwrap();
        let mut targets = inject.write_sync::<RenderTargets>().unwrap();
        targets.next_frame();
    }

    /// Get the current render aspect ratio.
    /// # DI Access
    /// - Read [`RenderTargets`]
    pub fn aspect_ratio(&self) -> f32 {
        let inject = self.bus.data().read().unwrap();
        let targets = inject.read_sync::<RenderTargets>().unwrap();
        let resolution = targets.size_group_resolution(SizeGroup::OutputResolution);
        resolution.width as f32 / resolution.height as f32
    }

    /// Updates the internal render state with data from the world.
    /// # DI Access
    /// - Read [`CameraState`]
    fn update_render_state(&mut self, world: &World) -> Result<()> {
        let di = self.bus.data().read().unwrap();
        let camera = di.read_sync::<CameraState>().unwrap();
        self.state.view = camera.matrix();
        self.state.projection =
            Mat4::perspective_rh(camera.fov().to_radians(), self.aspect_ratio(), 0.1, 10000000.0);
        // Flip y because Vulkan
        let v = self.state.projection.col_mut(1).y;
        self.state.projection.col_mut(1).y = v * -1.0;
        self.state.cam_position = camera.position().0;
        self.state.projection_view = self.state.projection * self.state.view;
        self.state.inverse_projection_view = self.state.projection_view.inverse();
        self.state.inverse_projection = self.state.projection.inverse();
        self.state.inverse_view_rotation =
            Mat4::from_mat3(Mat3::from_mat4(self.state.view)).inverse();
        self.state.sun_direction = -world.sun_direction.front_direction();
        Ok(())
    }

    /// Redraw the world. Returns a frame graph and physical resource bindings that
    /// can be submitted to the GPU.
    /// # DI Access
    /// - Read [`RenderTargets`]
    pub fn redraw_world<'cb>(
        &'cb mut self,
        world: &'cb World,
    ) -> Result<(FrameGraph<'cb>, PhysicalResourceBindings)> {
        let mut bindings = PhysicalResourceBindings::new();
        let mut graph = FrameGraph::new();
        {
            let inject = self.bus.data().read().unwrap();
            let targets = inject.read_sync::<RenderTargets>().unwrap();
            targets.bind_targets(&mut bindings);
        }

        self.update_render_state(world)?;

        let scene_output = VirtualResource::image("scene_output");
        let depth = VirtualResource::image("depth");
        let resolved_output = VirtualResource::image("resolved_output");
        let tonemapped_output = VirtualResource::image(Tonemap::output_name());

        // Before all regular render passes we want to execute any other requested work.
        {
            let di = self.bus.data().read().unwrap();
            let mut work = di.write_sync::<GpuWork>().unwrap();
            let resources = SceneResources {
                color: scene_output.clone(),
                depth: depth.clone(),
            };
            work.drain_record(&mut graph, &resources, &self.state, world)?;
        }

        // 1. Render terrain
        self.terrain
            .render(&mut graph, &scene_output, &depth, world, &self.state)?;
        // 2. Render atmosphere
        self.atmosphere
            .render(&mut graph, &scene_output, &depth, world, &self.state)?;
        // 3. Resolve MSAA
        let resolve = PassBuilder::render("msaa_resolve")
            .color_attachment(
                &graph.latest_version(&scene_output)?,
                vk::AttachmentLoadOp::LOAD,
                None,
            )?
            // We dont currently need depth resolved
            // .depth_attachment(graph.latest_version(depth.clone())?, vk::AttachmentLoadOp::LOAD, None)?
            .resolve(&graph.latest_version(&scene_output)?, &resolved_output)
            .build();
        graph.add_pass(resolve);
        // 4. Apply tonemapping
        self.tonemap.render(&mut graph, &resolved_output)?;
        // 5. Alias our final result to the expected name
        graph.alias("renderer_output", tonemapped_output);

        Ok((graph, bindings))
    }
}
