use std::path::Path;

use anyhow::Result;
use layout::backends::svg::SVGWriter;
use layout::gv;
use layout::gv::GraphBuilder;
use phobos::prelude as ph;
use phobos::prelude::traits::*;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use winit::window::Window;

use crate::gfx::world::World;
use crate::gui::util::integration::UIIntegration;
use crate::{gfx, gui};

#[derive(Debug)]
pub struct UpdateLoop {}

async fn save_dotfile<G>(graph: &G, path: &str)
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
            let mut f = File::create(Path::new(path)).await.unwrap();
            f.write(&svg.as_bytes()).await.unwrap();
        }
        Err(e) => {
            parser.print_error();
            println!("dot render error: {}", e);
        }
    }
}

impl UpdateLoop {
    pub fn new(_gfx: &gfx::Context) -> Result<Self> {
        Ok(Self {})
    }

    pub async fn update(
        &mut self,
        mut ifc: ph::InFlightContext<'_>,
        ui: &mut UIIntegration,
        window: &Window,
        world: &World,
        renderer: &mut gfx::WorldRenderer,
        gfx: gfx::SharedContext,
        debug: Option<&ph::DebugMessenger>,
    ) -> Result<ph::CommandBuffer<ph::domain::All>> {
        // Always repaint every frame from now on
        let (mut graph, mut bindings) = renderer.redraw_world(world).await?;

        let swapchain = graph.swapchain_resource();
        // Record UI commands
        ui.render(window, swapchain.clone(), &mut graph).await?;
        // Add a present pass to the graph.
        let present_pass = ph::PassBuilder::present("present", &graph.latest_version(swapchain)?);
        graph.add_pass(present_pass);
        let mut graph = graph.build()?;

        // save_dotfile(graph.task_graph(), "graph.svg").await;

        // Bind the swapchain resource.
        bindings.bind_image("swapchain", ifc.swapchain_image.as_ref().unwrap());
        // Record this frame.
        let cmd = gfx
            .exec
            .on_domain::<ph::domain::All>(Some(gfx.pipelines.clone()), Some(gfx.descriptors.clone()))?;
        let cmd = graph.record(cmd, &bindings, &mut ifc, debug)?;
        cmd.finish()
    }
}
