pub use graph::*;
use inject::DI;
pub use pass::*;
use phobos::domain::All;
use phobos::sync::submit_batch::SubmitBatch;
use scheduler::EventBus;

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
}

pub fn initialize(bus: &EventBus<DI>) {
    let work = GpuWork::new();
    let mut di = bus.data().write().unwrap();
    di.put_sync(work);
}
