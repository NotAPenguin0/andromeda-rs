use std::path::Path;
use phobos as ph;

use anyhow::Result;
use layout::backends::svg::SVGWriter;
use layout::gv;
use layout::gv::GraphBuilder;
use phobos::{GraphViz, IncompleteCmdBuffer};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use winit::window::Window;

use crate::{gfx, gui};
use crate::app::{repaint};

#[derive(Debug)]
pub struct UpdateLoop {

}

async fn save_dotfile<G>(graph: &G, path: &str) where G: GraphViz {
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
            let mut f = File::create(Path::new(path)).await.unwrap();
            f.write(&svg.as_bytes()).await.unwrap();
        },
        Err(e) => {
            parser.print_error();
            println!("dot render error: {}", e);
        }
    }
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
        let graph = ui.render(window, swapchain.clone(), graph).await?;
        // Add a present pass to the graph.
        let present_pass = ph::PassBuilder::present("present", swapchain.upgrade());
        let mut graph = graph.add_pass(present_pass)?.build()?;

        if status == repaint::RepaintStatus::All {
            save_dotfile(graph.task_graph(), "graph.svg").await;
        }

        // Bind the swapchain resource.
        bindings.bind_image("swapchain", ifc.swapchain_image.clone().unwrap());
        // Record this frame.
        let cmd = gfx.exec.on_domain::<ph::domain::All>()?;
        let cmd = ph::record_graph(&mut graph, &bindings, &mut ifc, cmd, debug)?;
        cmd.finish()
    }
}
