#![feature(async_closure)]
#![feature(never_type)]
#![feature(fn_traits)]

mod driver;
mod gfx;
mod gui;
mod repaint;
mod event;
mod safe_error;
mod hot_reload;

use tokio;

use anyhow::Result;
use winit::event_loop::ControlFlow;
use crate::driver::{Driver, process_event};
use safe_error::SafeUnwrap;

extern crate pretty_env_logger;
#[macro_use] extern crate log;

#[macro_use]
extern crate derivative;

fn main() -> Result<!> {
    std::env::set_var("RUST_LOG", "debug");
    pretty_env_logger::init_timed();

    // Initialize tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _guard = runtime.enter();

    // Create window
    let (event_loop, window) = Driver::create_window()?;
    // Create application driver
    let mut driver = Driver::init(&event_loop, window)?;

    // Run the app driver on the event loop
    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow { return; }
        let result = process_event(&mut driver, event);
        match result {
            Ok(flow) => { *control_flow = flow }
            Err(e) => { Err(e).safe_unwrap() }
        }
    });
}
