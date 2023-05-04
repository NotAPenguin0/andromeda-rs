use inject::DI;
use scheduler::EventBus;

use crate::util::image_provider::ImageProvider;
use crate::util::size::USize;

pub mod editor;
pub mod util;
pub mod widgets;

pub fn initialize(bus: &EventBus<DI>) {
    let mut inject = bus.data().write().unwrap();
    inject.put_sync(ImageProvider {
        handle: None,
        size: USize::new(800, 600),
    })
}
