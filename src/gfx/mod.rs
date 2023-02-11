use std::sync::{Arc, Mutex};
use anyhow::Result;

use phobos as ph;
use phobos::vk;

/// All shared graphics objects, these are safely refcounted using Arc and Arc<Mutex> where necessary, so cloning this struct is acceptable.
#[derive(Debug, Clone)]
pub struct SharedContext {
    pub allocator: Arc<Mutex<ph::Allocator>>,
    pub exec: Arc<ph::ExecutionManager>,
    pub pipelines: Arc<Mutex<ph::PipelineCache>>,
    pub descriptors: Arc<Mutex<ph::DescriptorCache>>,
    pub device: Arc<ph::Device>
}

pub fn redraw_world<'e, 'q>() -> Result<(ph::PassGraph<'e, 'q, ph::domain::All>, ph::PhysicalResourceBindings)> {
    let graph = ph::PassGraph::new();
    let bindings = ph::PhysicalResourceBindings::new();

    Ok((graph, bindings))
}