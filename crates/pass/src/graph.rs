use std::collections::HashMap;

use anyhow::{anyhow, Result};
use derivative::Derivative;
use phobos::graph::pass_graph::BuiltPassGraph;
use phobos::{domain, Allocator, DefaultAllocator, Pass, PassGraph, VirtualResource};
use statistics::RendererStatistics;

#[derive(Derivative, Default)]
#[derivative(Debug)]
pub struct FrameGraph<'cb, A: Allocator = DefaultAllocator> {
    #[derivative(Debug = "ignore")]
    passes: HashMap<String, Pass<'cb, domain::All, RendererStatistics, A>>,
    aliases: HashMap<String, VirtualResource>,
}

impl<'cb, A: Allocator> FrameGraph<'cb, A> {
    pub fn new() -> Self {
        Self {
            passes: Default::default(),
            aliases: Default::default(),
        }
    }

    /// Returns source version of swapchain resource
    pub fn swapchain_resource(&self) -> VirtualResource {
        VirtualResource::image("swapchain")
    }

    pub fn add_pass(&mut self, pass: Pass<'cb, domain::All, RendererStatistics, A>) {
        self.passes.insert(pass.name().to_owned(), pass);
    }

    pub fn alias(&mut self, str: impl Into<String>, resource: VirtualResource) {
        self.aliases.insert(str.into(), resource);
    }

    pub fn aliased_resource(&self, name: &str) -> Result<VirtualResource> {
        self.aliases
            .get(name)
            .ok_or_else(|| anyhow!("No such alias {name:?}"))
            .cloned()
    }

    pub fn latest_version(&self, resource: &VirtualResource) -> Result<VirtualResource> {
        self.passes
            .values()
            .flat_map(|pass| pass.output(resource).cloned())
            .max_by_key(|resource| resource.version())
            .ok_or_else(|| anyhow!("No such resource {resource:?}"))
    }

    #[allow(dead_code)]
    pub fn output(&self, pass: &str, resource: &VirtualResource) -> Result<&VirtualResource> {
        let pass = self
            .passes
            .get(pass)
            .ok_or_else(|| anyhow!("No such pass {pass:?}"))?;
        pass.output(resource)
            .ok_or_else(|| anyhow!("No such resource {resource:?}"))
    }

    pub fn build(self) -> Result<BuiltPassGraph<'cb, domain::All, RendererStatistics, A>> {
        let mut graph = PassGraph::new(Some(&self.swapchain_resource()));
        for (_, pass) in self.passes {
            graph = graph.add_pass(pass)?;
        }
        graph.build()
    }
}
