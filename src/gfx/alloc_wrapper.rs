use std::ops::Deref;
use std::sync::{Arc, Mutex};

use phobos as ph;

#[derive(Debug, Clone)]
pub struct ThreadSafeAllocator(Arc<Mutex<ph::Allocator>>);

impl ThreadSafeAllocator {
    pub fn new(alloc: Arc<Mutex<ph::Allocator>>) -> Self {
        Self {
            0: alloc
        }
    }
}

impl Deref for ThreadSafeAllocator {
    type Target = Arc<Mutex<ph::Allocator>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}