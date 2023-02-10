use actix::prelude::*;
use anyhow::Result;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

/// Main application driver. Hosts the event loop.
pub struct Driver {}

impl Driver {
    pub fn init() -> Result<Self> {

        Ok(Driver {})
    }

    pub fn main_loop(&mut self) -> Result<()> {
        let event_loop = winit::event_loop::EventLoopBuilder::new().build();
        let window = winit::window::WindowBuilder::new()
            .with_title("Andromeda")
            .with_inner_size(LogicalSize::new(1920.0, 1080.0))
            .build(&event_loop)?;

        event_loop.run(move |event, _, control_flow| {
            if let ControlFlow::ExitWithCode(_) = *control_flow { return }
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id
                } if window_id == window.id() => {
                    *control_flow = ControlFlow::Exit;
                },
                _ => (),
            }
        })
    }
}