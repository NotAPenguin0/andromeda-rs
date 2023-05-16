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

pub type BatchEventReceiver = tokio::sync::broadcast::Receiver<()>;
pub type BatchEventSender = tokio::sync::broadcast::Sender<()>;

pub struct GpuWork {
    pub batch: Option<SubmitBatch<All>>,
    batch_ready_rx: BatchEventReceiver,
    batch_ready_tx: BatchEventSender,
}

impl GpuWork {
    fn new() -> Self {
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        Self {
            batch: None,
            batch_ready_rx: rx,
            batch_ready_tx: tx,
        }
    }

    pub fn put_batch(&mut self, batch: SubmitBatch<All>) {
        self.batch = Some(batch);
        self.batch_ready_tx.send(()).unwrap();
    }

    pub fn take_batch(&mut self) -> Option<SubmitBatch<All>> {
        self.batch.take()
    }

    pub fn receiver(bus: &EventBus<DI>) -> BatchEventReceiver {
        let di = bus.data().read().unwrap();
        let this = di.read_sync::<Self>().unwrap();
        this.batch_ready_rx.resubscribe()
    }

    pub fn with_batch<R, F: FnOnce(&mut SubmitBatch<All>) -> R>(
        bus: &EventBus<DI>,
        f: F,
    ) -> Result<R> {
        let mut recv = Self::receiver(bus);
        futures::executor::block_on(recv.recv())?;
        let di = bus.data().read().unwrap();
        let mut this = di.write_sync::<Self>().unwrap();
        match &mut this.batch {
            None => {
                bail!("Batch ready signal received but no batch found in GpuWork structure.")
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
