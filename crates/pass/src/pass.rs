use anyhow::Result;
use gfx::state::{RenderState, SceneResources};
use phobos::Allocator;
use world::World;

use crate::FrameGraph;

pub trait Pass<A: Allocator> {
    fn record<'cb>(
        &'cb mut self,
        graph: &mut FrameGraph<'cb, A>,
        resources: &SceneResources,
        state: &'cb RenderState,
        world: &'cb World,
    ) -> Result<()>;
}
