#![feature(async_closure)]
#![feature(never_type)]

mod driver;
mod gfx;
mod repaint;
mod event;

use tokio;
use tokio::sync::mpsc;

use anyhow::Result;
use futures::executor::block_on;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use crate::driver::Driver;

extern crate pretty_env_logger;
#[macro_use] extern crate log;


fn main() -> Result<!> {
    pretty_env_logger::init_timed();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _guard = runtime.enter();

    let (event_loop, window) = Driver::create_window()?;
    let mut driver = Driver::init(window)?;

    // TODO: move this somewhere
    event_loop.run(move |event, target, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow { return; }
        control_flow.set_wait();

        match event {
            winit::event::Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id
            } if window_id == driver.window.id() => {
                // Control flow is already set in main event handler
                control_flow.set_exit();
                driver.gfx.device.wait_idle().unwrap();
            },
            winit::event::Event::MainEventsCleared => {
                driver.window.request_redraw();
            }
            winit::event::Event::RedrawRequested(_) => { // TODO: Multi-window
                // TODO: Handle error gracefully instead of unwrapping
                block_on(driver.process_frame()).unwrap()
            }
            _ => (),
        };
    });
}
