use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, Result};
use gfx::SharedContext;
use inject::DI;
use layout::backends::svg::SVGWriter;
use layout::gv;
use layout::gv::GraphBuilder;
use pass::GpuWork;
use phobos::domain::All;
use phobos::sync::submit_batch::SubmitBatch;
use phobos::{
    CommandBuffer, GraphViz, InFlightContext, IncompleteCmdBuffer, PassBuilder,
    RecordGraphToCommandBuffer,
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
    bus: EventBus<DI>,
}

#[allow(dead_code)]
fn save_dotfile<G>(graph: &G, path: &str)
where
    G: GraphViz, {
    let dot = graph.dot().unwrap();
    let dot = format!("{}", dot);
    let mut parser = gv::DotParser::new(&dot);
    match parser.process() {
        Ok(g) => {
            let mut svg = SVGWriter::new();
            let mut builder = GraphBuilder::new();
            builder.visit_graph(&g);
            let mut vg = builder.get();
            vg.do_it(false, false, false, &mut svg);
            let svg = svg.finalize();
            let mut f = File::create(Path::new(path)).unwrap();
            f.write(&svg.as_bytes()).unwrap();
        }
        Err(e) => {
            parser.print_error();
            println!("dot render error: {}", e);
        }
    }
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
            renderer: WorldRenderer::new(gfx.clone(), bus.clone())?,
            ui: UIIntegration::new(event_loop, window, gfx.clone())?,
            gfx,
            bus,
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

    // Create a new submit batch and take out the old one to submit it
    pub fn new_submit_batch(&self) -> Result<SubmitBatch<All>> {
        let new_batch = self.gfx.exec.start_submit_batch()?;
        let di = self.bus.data().read().unwrap();
        let mut work = di.write_sync::<GpuWork>().unwrap();
        let old_batch = work.take_batch();
        work.put_batch(new_batch);
        old_batch.ok_or_else(|| anyhow!("No previous submit batch set"))
    }

    /// Render a single frame to the window. This will render both the UI and the scene.
    /// Returns a command buffer that must be passed to phobos as this frame's command buffer.
    pub fn render(
        &mut self,
        window: &Window,
        world: &World,
        bus: &EventBus<DI>,
        ifc: &mut InFlightContext,
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

        // save_dotfile(graph.task_graph(), "graph.svg");

        let inject = bus.data().read().unwrap();
        let mut statistics = inject.write_sync::<RendererStatistics>().unwrap();
        let cmd = cmd.begin_section(&mut statistics, "all_render")?;
        let cmd =
            graph.record(cmd, &bindings, ifc, self.gfx.debug_messenger.clone(), &mut statistics)?;
        cmd.end_section(&mut statistics, "all_render")?.finish()
    }
}
