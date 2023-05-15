use std::collections::HashMap;

use anyhow::Result;
use gfx::state::{RenderState, SceneResources};
pub use graph::*;
use inject::DI;
pub use pass::*;
use phobos::DefaultAllocator;
use scheduler::EventBus;
use util::SafeUnwrap;
use world::World;

pub mod graph;
pub mod pass;

type Alloc = DefaultAllocator;

struct EnabledPass {
    pub name: String,
    // Amount of times this pass is enabled
    pub num_frames: u32,
}

/// This struct stores additional GPU work that can be submitted to be
/// executed on the next frame. Note that each pass is dropped immediately after
/// recording it, so storing resources inside is not possible (right now).
pub struct GpuWork {
    passes: HashMap<String, Box<dyn Pass<Alloc>>>,
    enabled_passes: Vec<EnabledPass>,
}

impl GpuWork {
    fn new() -> Self {
        Self {
            passes: Default::default(),
            enabled_passes: vec![],
        }
    }

    pub fn add_pass<P: Pass<Alloc> + 'static>(&mut self, pass: P, name: impl Into<String>) {
        self.passes.insert(name.into(), Box::new(pass));
    }

    /// Enable a pass for a single frame
    pub fn enable_once(&mut self, pass: impl Into<String>) {
        self.enabled_passes.push(EnabledPass {
            name: pass.into(),
            num_frames: 1,
        });
    }

    /// Record all passes and disable them if needed.
    pub fn drain_record<'cb>(
        &'cb mut self,
        graph: &mut FrameGraph<'cb, Alloc>,
        resources: &SceneResources,
        state: &'cb RenderState,
        world: &'cb World,
    ) -> Result<()> {
        self.enabled_passes = self
            .enabled_passes
            .drain(0..)
            .filter_map(|mut pass| {
                let exec = self.passes.get_mut(&pass.name)?;
                exec.record(graph, resources, state, world).safe_unwrap();
                pass.num_frames -= 1;
                if pass.num_frames == 0 {
                    None
                } else {
                    Some(pass)
                }
            })
            .collect();
        Ok(())
    }
}

pub fn initialize(bus: &EventBus<DI>) {
    let work = GpuWork::new();
    let mut di = bus.data().write().unwrap();
    di.put_sync(work);
}
