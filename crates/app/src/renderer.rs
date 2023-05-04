use anyhow::Result;
use gfx::SharedContext;
use inject::DI;
use phobos::domain::All;
use phobos::{
    CommandBuffer, InFlightContext, IncompleteCmdBuffer, PassBuilder, RecordGraphToCommandBuffer,
};
use renderer::ui_integration::UIIntegration;
use renderer::world_renderer::WorldRenderer;
use scheduler::EventBus;
use statistics::{RendererStatistics, TimedCommandBuffer};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;
use world::World;

/// Stores the graphics and context, as well as the world and GUI renderers.
#[derive(Debug)]
pub struct AppRenderer {
    gfx: SharedContext,
    renderer: WorldRenderer,
    ui: UIIntegration,
}

impl AppRenderer {
    /// Initialize the application rendering system with an existing graphics context.
    pub fn new(
        gfx: SharedContext,
        window: &Window,
        event_loop: &EventLoop<()>,
        bus: EventBus<DI>,
    ) -> Result<Self> {
        Ok(Self {
            renderer: WorldRenderer::new(gfx.clone(), bus)?,
            ui: UIIntegration::new(event_loop, window, gfx.clone())?,
            gfx,
        })
    }

    /// Get the UI context.
    pub fn ui(&self) -> egui::Context {
        self.ui.context()
    }

    /// Get the graphics context.
    pub fn gfx(&self) -> SharedContext {
        self.gfx.clone()
    }

    /// Forward a winit event to the UI integration.
    pub fn process_event(&mut self, event: &WindowEvent) {
        self.ui.process_event(event);
    }

    /// Call each frame to update per-frame resources and state.
    pub fn new_frame(&mut self, window: &Window) {
        self.ui.new_frame(window);
        self.renderer.new_frame();
        self.gfx.pipelines.next_frame();
        self.gfx.descriptors.next_frame();
    }

    /// Render a single frame to the window. This will render both the UI and the scene.
    /// Returns a command buffer that must be passed to phobos as this frame's command buffer.
    pub fn render(
        &mut self,
        window: &Window,
        world: &World,
        bus: EventBus<DI>,
        mut ifc: InFlightContext,
    ) -> Result<CommandBuffer<All>> {
        self.renderer.update_output_image(&mut self.ui)?;
        let (mut graph, mut bindings) = self.renderer.redraw_world(world)?;
        let swapchain = graph.swapchain_resource();
        // Record UI commands
        self.ui.render(window, swapchain.clone(), &mut graph)?;
        // Add a present pass to the graph.
        let present_pass = PassBuilder::present("present", &graph.latest_version(&swapchain)?);
        graph.add_pass(present_pass);
        let mut graph = graph.build()?;

        // Bind the swapchain resource.
        bindings.bind_image("swapchain", ifc.swapchain_image.as_ref().unwrap());
        // Record this frame.
        let cmd = self.gfx.exec.on_domain::<All, _>(
            Some(self.gfx.pipelines.clone()),
            Some(self.gfx.descriptors.clone()),
        )?;

        let inject = bus.data().read().unwrap();
        let mut statistics = inject.write_sync::<RendererStatistics>().unwrap();
        let cmd = cmd.begin_section(&mut statistics, "all_render")?;
        let cmd = graph.record(
            cmd,
            &bindings,
            &mut ifc,
            self.gfx.debug_messenger.clone(),
            &mut statistics,
        )?;
        cmd.end_section(&mut statistics, "all_render")?.finish()
    }
}
