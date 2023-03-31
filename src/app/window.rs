use anyhow::Result;
use phobos::domain::ExecutionDomain;
use phobos::{Allocator, CommandBuffer, DefaultAllocator, FrameManager, InFlightContext, Surface};
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::window::{Window, WindowBuilder, WindowId};

use crate::gfx::SharedContext;

/// Create the winit window and event loop.
pub fn create_window() -> Result<(EventLoop<()>, Window)> {
    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_title("Andromeda")
        .with_inner_size(winit::dpi::LogicalSize::new(1920.0, 1080.0))
        .build(&event_loop)?;
    Ok((event_loop, window))
}

/// The main application window. Holds the phobos frame manager and surface, as well as the
/// winit window.
#[derive(Debug)]
pub struct AppWindow<A: Allocator = DefaultAllocator> {
    frame: FrameManager<A>,
    window: Window,
    surface: Surface,
    gfx: SharedContext,
}

impl<A: Allocator> AppWindow<A> {
    /// Create a new application window.
    pub fn new(
        frame: FrameManager<A>,
        window: Window,
        surface: Surface,
        gfx: SharedContext,
    ) -> Self {
        Self {
            frame,
            window,
            surface,
            gfx,
        }
    }

    /// Start a new frame and run the given function when it is ready.
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

    /// Get the window id of this application window.
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Request a redraw from winit.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}
