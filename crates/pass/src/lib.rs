pub use graph::*;
use inject::DI;
pub use pass::*;
use phobos::domain::All;
use phobos::sync::submit_batch::SubmitBatch;
use scheduler::EventBus;

pub mod graph;
pub mod pass;

pub struct GpuWork {
    pub batch: Option<SubmitBatch<All>>,
}

impl GpuWork {
    fn new() -> Self {
        Self {
            batch: None,
        }
    }

    pub fn take_batch(&mut self) -> Option<SubmitBatch<All>> {
        self.batch.take()
    }
}

pub fn initialize(bus: &EventBus<DI>) {
    let work = GpuWork::new();
    let mut di = bus.data().write().unwrap();
    di.put_sync(work);
}
