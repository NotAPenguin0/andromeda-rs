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
use crate::driver::{Driver, process_event};

extern crate pretty_env_logger;
#[macro_use] extern crate log;


fn main() -> Result<!> {
    pretty_env_logger::init_timed();
    // Initialize tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _guard = runtime.enter();

    // Create window
    let (event_loop, window) = Driver::create_window()?;
    // Create application driver
    let mut driver = Driver::init(window)?;

    // Run the app driver on the event loop
    event_loop.run(move |event, target, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow { return; }
        *control_flow = process_event(&mut driver, event).unwrap();
    });
}
