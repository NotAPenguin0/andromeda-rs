use std::collections::HashMap;
use phobos as ph;

use anyhow::{anyhow, Result};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FrameGraph<'e, 'q> {
    #[derivative(Debug="ignore")]
    passes: HashMap<String, ph::Pass<'e, 'q, ph::domain::All>>
}

impl<'e, 'q> FrameGraph<'e, 'q> {
    pub fn new() -> Self {
        Self { passes: Default::default() }
    }

    pub fn add_pass(&mut self, pass: ph::Pass<'e, 'q, ph::domain::All>) {
        self.passes.insert(pass.name().to_owned(), pass);
    }

    pub fn latest_version(&self, resource: ph::VirtualResource) -> Result<ph::VirtualResource> {
        self.passes.values().flat_map(|pass| {
            pass.output(&resource)
        })
        .max_by_key(|resource| {
            resource.version()
        })
        .ok_or(anyhow!("No such resource {:?}", resource))
    }

    pub fn output(&self, pass: &str, resource: ph::VirtualResource) -> Result<ph::VirtualResource> {
        let pass = self.passes.get(pass)
            .ok_or(anyhow!("No such pass {:?}", pass))?;
        pass.output(&resource)
            .ok_or(anyhow!("No such resource {:?}", resource))
    }

    pub fn build(mut self) -> Result<ph::BuiltPassGraph<'e, 'q, ph::domain::All>> {
        let mut graph = ph::PassGraph::new();
        for (_, pass) in self.passes {
            graph = graph.add_pass(pass)?;
        }
        graph.build()
    }
}