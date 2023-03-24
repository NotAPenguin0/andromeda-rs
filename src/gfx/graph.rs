use std::collections::HashMap;

use anyhow::{anyhow, Result};
use phobos::graph::pass_graph::BuiltPassGraph;
use phobos::prelude as ph;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FrameGraph<'e, 'q> {
    #[derivative(Debug = "ignore")]
    passes: HashMap<String, ph::Pass<'e, 'q, ph::domain::All>>,
    aliases: HashMap<String, ph::VirtualResource>,
}

impl<'e, 'q> FrameGraph<'e, 'q> {
    pub fn new() -> Self {
        Self {
            passes: Default::default(),
            aliases: Default::default(),
        }
    }

    /// Returns source version of swapchain resource
    pub fn swapchain_resource(&self) -> ph::VirtualResource {
        ph::VirtualResource::image("swapchain")
    }

    pub fn add_pass(&mut self, pass: ph::Pass<'e, 'q, ph::domain::All>) {
        self.passes.insert(pass.name().to_owned(), pass);
    }

    pub fn alias(&mut self, str: impl Into<String>, resource: ph::VirtualResource) {
        self.aliases.insert(str.into(), resource);
    }

    pub fn aliased_resource(&self, name: &str) -> Result<ph::VirtualResource> {
        self.aliases.get(name).ok_or(anyhow!("No such alias {:?}", name)).cloned()
    }

    pub fn latest_version(&self, resource: &ph::VirtualResource) -> Result<ph::VirtualResource> {
        self.passes
            .values()
            .flat_map(|pass| pass.output(&resource).cloned())
            .max_by_key(|resource| resource.version())
            .ok_or(anyhow!("No such resource {:?}", resource))
    }

    #[allow(dead_code)]
    pub fn output(&self, pass: &str, resource: &ph::VirtualResource) -> Result<&ph::VirtualResource> {
        let pass = self.passes.get(pass).ok_or(anyhow!("No such pass {:?}", pass))?;
        pass.output(resource).ok_or(anyhow!("No such resource {:?}", resource))
    }

    pub fn build(self) -> Result<BuiltPassGraph<'e, 'q, ph::domain::All>> {
        let mut graph = ph::PassGraph::new(Some(&self.swapchain_resource()));
        for (_, pass) in self.passes {
            graph = graph.add_pass(pass)?;
        }
        graph.build()
    }
}
