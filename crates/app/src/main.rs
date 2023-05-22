#![feature(async_closure)]
#![feature(never_type)]
#![feature(fn_traits)]
#![allow(dead_code)]

extern crate pretty_env_logger;

use anyhow::Result;
use error::publish_error;
use log::error;
use winit::event_loop::ControlFlow;

use crate::driver::Driver;

mod driver;
mod renderer;
mod window;

fn main() -> Result<!> {
    std::env::set_var("RUST_LOG", "trace");
    pretty_env_logger::init_timed();

    #[cfg(feature = "tokio-tracing")]
    console_subscriber::init();

    // Initialize tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _guard = runtime.enter();

    // Create window
    let (event_loop, window) = window::create_window()?;
    // Create application driver
    let mut driver = Some(Driver::init(&event_loop, window)?);

    // Run the app driver on the event loop
    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow {
            return;
        }

        let result = match driver.as_mut() {
            None => Ok(ControlFlow::Exit),
            Some(driver) => driver.process_event(event),
        };

        match result {
            Ok(ControlFlow::Exit) => {
                match driver.take() {
                    None => {}
                    Some(driver) => {
                        drop(driver);
                    }
                };
                *control_flow = ControlFlow::Exit;
            }
            Ok(flow) => *control_flow = flow,
            Err(e) => {
                let bus = &driver.as_ref().unwrap().bus;
                publish_error!(bus, "{e}");
                error!("{e}");
            }
        };
    })
}
