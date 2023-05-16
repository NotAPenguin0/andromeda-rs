use anyhow::{bail, Result};
pub use graph::*;
use inject::DI;
pub use pass::*;
use phobos::domain::All;
use phobos::sync::submit_batch::SubmitBatch;
use scheduler::EventBus;
use tokio::task::block_in_place;

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

    pub fn put_batch(&mut self, batch: SubmitBatch<All>) {
        self.batch = Some(batch);
    }

    pub fn take_batch(&mut self) -> Option<SubmitBatch<All>> {
        self.batch.take()
    }

    pub fn with_batch<R, F: FnOnce(&mut SubmitBatch<All>) -> R>(
        bus: &EventBus<DI>,
        f: F,
    ) -> Result<R> {
        let di = bus.data().read().unwrap();
        let mut this = di.write_sync::<Self>().unwrap();
        match &mut this.batch {
            None => {
                bail!("No submit batch registered. This is a bug the application.")
            }
            Some(batch) => Ok(f(batch)),
        }
    }
}

pub fn initialize(bus: &EventBus<DI>) {
    let work = GpuWork::new();
    let mut di = bus.data().write().unwrap();
    di.put_sync(work);
}
