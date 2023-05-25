use anyhow::Result;
use camera::CameraState;
use gfx::state::RenderState;
use gfx::SharedContext;
use glam::{Mat3, Mat4, Vec3};
use gui::util::image_provider::ImageProvider;
use hot_reload::IntoDynamic;
use inject::DI;
use pass::FrameGraph;
use phobos::fsr2::{FfxFloatCoords2D, Fsr2DispatchDescription};
use phobos::graph::pass::Fsr2DispatchVirtualResources;
use phobos::{image, vk, PassBuilder, PhysicalResourceBindings, PipelineBuilder, VirtualResource};
use scheduler::EventBus;
use time::Time;
use world::World;

use crate::passes::atmosphere::AtmosphereRenderer;
use crate::passes::terrain::TerrainRenderer;
use crate::passes::terrain_decal::TerrainDecal;
use crate::passes::world_position::WorldPositionReconstruct;
use crate::postprocess::tonemap::Tonemap;
use crate::ui_integration::UIIntegration;
use crate::util::targets::{RenderTargets, SizeGroup, TargetSize, UpscaleQuality};

/// The world renderer is responsible for all the rendering logic
/// of the scene.
#[derive(Debug)]
pub struct WorldRenderer {
    bus: EventBus<DI>,
    tonemap: Tonemap,
    atmosphere: AtmosphereRenderer,
    terrain: TerrainRenderer,
    world_pos_reconstruct: WorldPositionReconstruct,
    terrain_decal: TerrainDecal,
    state: RenderState,
    ctx: SharedContext,
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
            .into_dynamic()
            .attach_shader("shaders/src/simple_mesh.vs.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/solid_color.fs.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(&mut bus, ctx.pipelines.clone())?;

        let mut targets = RenderTargets::new(ctx.clone())?;
        targets.set_output_resolution(16, 16)?;
        targets.set_upscale_quality(UpscaleQuality::Quality)?;

        targets.register_color_target(
            "scene_output",
            SizeGroup::RenderResolution,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R32G32B32A32_SFLOAT,
        )?;

        targets.register_color_target(
            "motion",
            SizeGroup::RenderResolution,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R16G16_SFLOAT,
        )?;

        targets.register_depth_target(
            "depth",
            SizeGroup::RenderResolution,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::D32_SFLOAT,
        )?;

        targets.register_color_target(
            "upscaled_output",
            SizeGroup::OutputResolution,
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::STORAGE,
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
            terrain: TerrainRenderer::new(ctx.clone(), &mut bus)?,
            world_pos_reconstruct: WorldPositionReconstruct::new(ctx.clone(), &mut bus)?,
            terrain_decal: TerrainDecal::new(ctx.clone(), bus.clone())?,
            bus,
            state,
            ctx,
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
        targets.set_output_resolution(
            (provider.size.x() as f32 * 1.5) as u32,
            (provider.size.y() as f32 * 1.5) as u32,
        )?;
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

    /// # DI Access
    /// - Read [`RenderTargets`]
    pub fn render_resolution(&self) -> TargetSize {
        let inject = self.bus.data().read().unwrap();
        let targets = inject.read_sync::<RenderTargets>().unwrap();
        targets.size_group_resolution(SizeGroup::RenderResolution)
    }

    /// # DI Access
    /// - Read [`RenderTargets`]
    pub fn output_resolution(&self) -> TargetSize {
        let inject = self.bus.data().read().unwrap();
        let targets = inject.read_sync::<RenderTargets>().unwrap();
        targets.size_group_resolution(SizeGroup::OutputResolution)
    }

    /// Get the current render aspect ratio.
    pub fn aspect_ratio(&self) -> f32 {
        let resolution = self.render_resolution();
        resolution.width as f32 / resolution.height as f32
    }

    /// Updates the internal render state with data from the world.
    /// # DI Access
    /// - Read [`CameraState`]
    fn update_render_state(&mut self, world: &World) -> Result<(f32, f32)> {
        self.state.previous_pv = self.state.projection_view;
        let di = self.bus.data().read().unwrap();
        let camera = di.read_sync::<CameraState>().unwrap();
        self.state.near = 0.1;
        self.state.far = 10000000.0;
        self.state.view = camera.matrix();
        self.state.fov = camera.fov().to_radians();
        self.state.projection = Mat4::perspective_rh(
            self.state.fov,
            self.aspect_ratio(),
            self.state.near,
            self.state.far,
        );
        // Jitter projection matrix
        let mut fsr2 = self.ctx.device.fsr2_context();
        let resolution = self.render_resolution();
        let (jitter_x, jitter_y) = fsr2.jitter_offset(resolution.width)?;
        let proj_jitter_x = 2.0 * jitter_x / resolution.width as f32;
        let proj_jitter_y = -2.0 * jitter_y / resolution.height as f32;
        let jitter_translation_matrix =
            Mat4::from_translation(Vec3::new(proj_jitter_x, proj_jitter_y, 0.0));
        self.state.projection = jitter_translation_matrix * self.state.projection;
        // Flip y because Vulkan
        let v = self.state.projection.col_mut(1).y;
        self.state.projection.col_mut(1).y = v * -1.0;
        self.state.cam_position = camera.position().0;
        self.state.projection_view = self.state.projection * self.state.view;
        self.state.inverse_projection_view = self.state.projection_view.inverse();
        self.state.inverse_projection = self.state.projection.inverse();
        self.state.inverse_view = self.state.view.inverse();
        self.state.inverse_view_rotation =
            Mat4::from_mat3(Mat3::from_mat4(self.state.view)).inverse();
        self.state.sun_direction = -world.sun_direction.front_direction();
        self.state.render_size = resolution.into();
        Ok((jitter_x, jitter_y))
    }

    /// Redraw the world. Returns a frame graph and physical resource bindings that
    /// can be submitted to the GPU.
    /// # DI Access
    /// - Read [`RenderTargets`]
    /// - Read [`Time`]
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

        let (jitter_x, jitter_y) = self.update_render_state(world)?;
        let resolution = self.render_resolution();

        let scene_output = image!("scene_output");
        let depth = image!("depth");
        let motion = image!("motion");
        let upscaled_output = image!("upscaled_output");
        let tonemapped_output = VirtualResource::image(Tonemap::output_name());

        // Render terrain
        self.terrain
            .render(&mut graph, &scene_output, &depth, &motion, world, &self.state)?;
        // Render atmosphere
        self.atmosphere
            .render(&mut graph, &scene_output, &depth, world, &self.state)?;
        // Render decal
        self.terrain_decal
            .render(&mut graph, &scene_output, &depth, world, &self.state)?;
        // Reconstruct world position from depth
        self.world_pos_reconstruct
            .render(&world, &mut graph, &depth, &self.state)?;

        // Upscale
        {
            let in_color = graph.latest_version(&scene_output).unwrap();
            let in_depth = graph.latest_version(&depth).unwrap();
            let in_motion = graph.latest_version(&motion).unwrap();

            let di = self.bus.data().read().unwrap();
            let time = di.read_sync::<Time>().unwrap();

            let fsr2_dispatch = Fsr2DispatchDescription {
                jitter_offset: FfxFloatCoords2D {
                    x: jitter_x,
                    y: jitter_y,
                },
                motion_vector_scale: FfxFloatCoords2D {
                    x: resolution.width as f32 / 2.0,
                    y: resolution.height as f32 / 2.0,
                },
                enable_sharpening: false,
                sharpness: 0.0,
                frametime_delta: time.delta,
                pre_exposure: 1.0,
                reset: false,
                camera_near: self.state.near,
                camera_far: self.state.far,
                camera_fov_vertical: self.state.fov,
                viewspace_to_meters_factor: 1.0,
                auto_reactive: None,
            };

            let fsr2_resources = Fsr2DispatchVirtualResources {
                color: in_color,
                depth: in_depth,
                motion_vectors: in_motion,
                exposure: None,
                reactive: None,
                transparency_and_composition: None,
                output: upscaled_output.clone(),
            };

            let fsr2_pass =
                PassBuilder::fsr2(self.ctx.device.clone(), fsr2_dispatch, fsr2_resources);
            graph.add_pass(fsr2_pass);
        }

        // Apply tonemapping
        self.tonemap.render(&mut graph, &upscaled_output)?;
        // Alias our final result to the expected name
        graph.alias("renderer_output", tonemapped_output);

        Ok((graph, bindings))
    }
}
