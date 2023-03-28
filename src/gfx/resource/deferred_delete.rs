use phobos::{Buffer, DeletionQueue};

use crate::gfx::PairedImageView;

pub trait DeleteDeferred {}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DeferredDelete {
    #[derivative(Debug = "ignore")]
    resources: DeletionQueue<Box<dyn DeleteDeferred>>,
}

impl DeferredDelete {
    pub fn new() -> Self {
        Self {
            resources: DeletionQueue::new(4),
        }
    }

    pub fn defer_deletion<T: DeleteDeferred + 'static>(&mut self, resource: T) {
        self.resources.push(Box::new(resource));
    }

    pub fn next_frame(&mut self) {
        self.resources.next_frame();
    }
}
