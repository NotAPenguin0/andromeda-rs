use std::path::Path;
use std::rc::Rc;

use anyhow::Result;
use layout::backends::svg::SVGWriter;
use layout::gv;
use layout::gv::GraphBuilder;
use phobos::prelude as ph;
use phobos::prelude::traits::*;
use poll_promise::Promise;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use winit::window::Window;

use crate::gfx;
use crate::gfx::resource::TerrainPlane;
use crate::gfx::world::{FutureWorld, World};
use crate::gui::util::integration::UIIntegration;

#[derive(Debug)]
pub struct UpdateLoop {}

#[allow(dead_code)]
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

    pub fn poll_future(&self, world: &mut World, future: &mut FutureWorld) {
        // If we have a promise we are polling. Note that we are not creating any bindings
        // to make sure we can move out of the promise by swapping it with None
        if let Some(_) = &future.terrain_mesh {
            if let Some(_) = future.terrain_mesh.as_ref().unwrap().ready() {
                // Unwrap safety: We just verified that this Option contains a value, and that
                // it is ready.
                let promise = future.terrain_mesh.take().unwrap();
                world.terrain_mesh = match promise.try_take() {
                    Ok(result) => match result {
                        Ok(terrain) => Some(Rc::new(terrain)),
                        Err(err) => {
                            error!("Error generating terrain mesh: {}", err);
                            None
                        }
                    },
                    Err(_) => None,
                }
            }
        }
    }

    pub async fn update(
        &mut self,
        mut ifc: ph::InFlightContext<'_>,
        ui: &mut UIIntegration,
        window: &Window,
        world: &mut World,
        future: &mut FutureWorld,
        renderer: &mut gfx::WorldRenderer,
        gfx: gfx::SharedContext,
        debug: Option<&ph::DebugMessenger>,
    ) -> Result<ph::CommandBuffer<ph::domain::All>> {
        self.poll_future(world, future);
        let (mut graph, mut bindings) = renderer.redraw_world(world).await?;

        let swapchain = graph.swapchain_resource();
        // Record UI commands
        ui.render(window, swapchain.clone(), &mut graph).await?;
        // Add a present pass to the graph.
        let present_pass = ph::PassBuilder::present("present", &graph.latest_version(&swapchain)?);
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
