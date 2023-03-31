use anyhow::Result;
use phobos::domain::ExecutionDomain;
use phobos::{Allocator, CommandBuffer, DefaultAllocator, FrameManager, InFlightContext, Surface};
use winit::window::{Window, WindowId};

use crate::gfx::SharedContext;

#[derive(Debug)]
pub struct AppWindow<A: Allocator = DefaultAllocator> {
    pub frame: FrameManager<A>,
    pub window: Window,
    pub surface: Surface,
    pub gfx: SharedContext,
}

impl<A: Allocator> AppWindow<A> {
    pub async fn new_frame<
        D: ExecutionDomain + 'static,
        F: FnOnce(&Window, InFlightContext<A>) -> Result<CommandBuffer<D>>,
    >(
        &mut self,
        func: F,
    ) -> Result<()> {
        self.frame
            .new_frame(self.gfx.exec.clone(), &self.window, &self.surface, |ifc| {
                func(&self.window, ifc)
            })
            .await
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}
