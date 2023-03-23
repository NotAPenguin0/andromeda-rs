#![feature(async_closure)]
#![feature(never_type)]
#![feature(fn_traits)]

#[macro_use]
extern crate derivative;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use anyhow::Result;
use tokio;
use winit::event_loop::ControlFlow;

use crate::app::*;
use crate::core::*;

mod app;
mod core;
mod gfx;
mod gui;
mod hot_reload;
mod math;
mod state;

fn main() -> Result<!> {
    std::env::set_var("RUST_LOG", "trace");
    pretty_env_logger::init_timed();

    // Initialize tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let _guard = runtime.enter();

    // Create window
    let (event_loop, window) = Driver::create_window()?;
    // Create application driver
    let mut driver = Driver::init(&event_loop, window)?;

    // Run the app driver on the event loop
    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow {
            return;
        }
        let result = process_event(&mut driver, event);
        match result {
            Ok(flow) => *control_flow = flow,
            Err(e) => Err(e).safe_unwrap(),
        }
    })
}
