use phobos as ph;

use anyhow::Result;
use phobos::IncompleteCmdBuffer;
use winit::window::Window;

use crate::{gfx, gui};
use crate::app::{repaint};

#[derive(Debug)]
pub struct UpdateLoop {

}

impl UpdateLoop {
    pub fn new(_gfx: &gfx::Context) -> Result<Self> {
        Ok(Self{

        })
    }

    pub async fn update(
        &mut self,
        mut ifc: ph::InFlightContext<'_>,
        ui: &mut gui::UIIntegration,
        window: &Window,
        scene_output: ph::ImageView,
        renderer: &mut gfx::WorldRenderer,
        status: repaint::RepaintStatus,
        gfx: gfx::SharedContext,
        debug: Option<&ph::DebugMessenger>)
        -> Result<ph::CommandBuffer<ph::domain::All>> {

        let (graph, mut bindings) = match status {
            repaint::RepaintStatus::None => { (ph::PassGraph::new(), ph::PhysicalResourceBindings::new()) }
            repaint::RepaintStatus::UIOnly => { (ph::PassGraph::new(), ph::PhysicalResourceBindings::new()) }
            repaint::RepaintStatus::All => {
                renderer.redraw_world().await?
            }
        };

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
