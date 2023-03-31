use anyhow::Result;
use phobos::domain::ExecutionDomain;
use phobos::{Allocator, CommandBuffer, DefaultAllocator, FrameManager, InFlightContext, Surface};
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::window::{Window, WindowBuilder, WindowId};

use crate::gfx::SharedContext;

pub fn create_window() -> Result<(EventLoop<()>, Window)> {
    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_title("Andromeda")
        .with_inner_size(winit::dpi::LogicalSize::new(1920.0, 1080.0))
        .build(&event_loop)?;
    Ok((event_loop, window))
}

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
