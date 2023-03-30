use std::path::Path;

use anyhow::Result;
use layout::backends::svg::SVGWriter;
use layout::gv;
use layout::gv::GraphBuilder;
use phobos::prelude::traits::*;
use phobos::{prelude as ph, DebugMessenger};
use poll_promise::Promise;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use winit::window::Window;

use crate::gfx;
use crate::gfx::resource::deferred_delete::{DeferredDelete, DeleteDeferred};
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

    fn try_take_promise<T: Send + DeleteDeferred + 'static, U: Send + Into<T>>(
        promise: &mut Option<Promise<Result<U>>>,
        dst: &mut Option<T>,
        deferred_delete: &mut DeferredDelete,
    ) {
        if let Some(_) = &promise {
            if let Some(_) = promise.as_ref().unwrap().ready() {
                // Unwrap safety: We just verified that this Option contains a value, and that
                // it is ready.
                let promise = promise.take().unwrap();
                // Defer deletion of old value
                let old_value = dst.take();
                match old_value {
                    None => {}
                    Some(value) => {
                        deferred_delete.defer_deletion(value);
                    }
                }
                *dst = match promise.try_take() {
                    Ok(result) => match result {
                        Ok(value) => Some(value.into()),
                        Err(err) => {
                            error!("Error inside promise: {}", err);
                            None
                        }
                    },
                    Err(_) => None,
                }
            }
        }
    }

    pub fn poll_future(
        &self,
        world: &mut World,
        future: &mut FutureWorld,
        deferred_delete: &mut DeferredDelete,
    ) {
        Self::try_take_promise(&mut future.terrain, &mut world.terrain, deferred_delete);
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
        deferred_delete: &mut DeferredDelete,
        debug_messenger: Option<&DebugMessenger>,
    ) -> Result<ph::CommandBuffer<ph::domain::All>> {
        self.poll_future(world, future, deferred_delete);
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
        let cmd = gfx.exec.on_domain::<ph::domain::All>(
            Some(gfx.pipelines.clone()),
            Some(gfx.descriptors.clone()),
        )?;
        let cmd = graph.record(cmd, &bindings, &mut ifc, debug_messenger)?;
        cmd.finish()
    }
}
