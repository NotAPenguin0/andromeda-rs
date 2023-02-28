use phobos as ph;

use anyhow::Result;
use phobos::IncompleteCmdBuffer;
use winit::window::Window;

use crate::{gfx, gui};
use crate::app::{repaint, RootActorSystem};

#[derive(Debug)]
pub struct UpdateLoop {

}

impl UpdateLoop {
    pub fn new(gfx: &gfx::Context) -> Result<Self> {
        Ok(Self{

        })
    }

    // TODO: fix lifetime issues or move back to main update fn
    async fn redraw<'s: 'e + 'q, 'e, 'q>(&'s mut self, actors: &mut RootActorSystem, renderer: &'s mut gfx::WorldRenderer) -> Result<(ph::PassGraph<'e, 'q, ph::domain::All>, ph::PhysicalResourceBindings)> {
        // If we have a repaint, ask the graphics system for a redraw
        // In the future, we could even make this fully asynchronous and keep refreshing the UI while
        // we redraw, though this is only necessary if our frame time budget gets seriously
        // exceeded.
        let status = actors.update_repaint_status().await?;
        Ok(match status {
            repaint::RepaintStatus::None => { (ph::PassGraph::new(), ph::PhysicalResourceBindings::new()) }
            repaint::RepaintStatus::UIOnly => { (ph::PassGraph::new(), ph::PhysicalResourceBindings::new()) }
            repaint::RepaintStatus::All => {
                renderer.redraw_world()?
            }
        })
    }

    pub async fn update(
        &mut self,
        mut ifc: ph::InFlightContext<'_>,
        ui: &mut gui::UIIntegration,
        window: &Window,
        renderer: &mut gfx::WorldRenderer,
        actors: &mut RootActorSystem,
        gfx: gfx::SharedContext,
        debug: Option<&ph::DebugMessenger>)
        -> Result<ph::CommandBuffer<ph::domain::All>> {

        actors.update_rt_size(ui, renderer).await?;

        let scene_output = renderer.output_image().view.clone();

        let (graph, mut bindings) = self.redraw(actors, renderer).await?;
        let swapchain = ph::VirtualResource::image("swapchain");
        // Record UI commands
        let graph = ui.render(window, swapchain.clone(), graph)?;
        // Add a present pass to the graph.
        let present_pass = ph::PassBuilder::present("present", swapchain.upgrade());
        let mut graph = graph.add_pass(present_pass)?.build()?;

        // Bind the swapchain resource.
        bindings.bind_image("swapchain", ifc.swapchain_image.clone().unwrap());
        // Bind the output image resource

        bindings.bind_image("output", scene_output);
        // Record this frame.
        let cmd = gfx.exec.on_domain::<ph::domain::All>()?;
        let cmd = ph::record_graph(&mut graph, &bindings, &mut ifc, cmd, debug)?;
        cmd.finish()
    }
}
