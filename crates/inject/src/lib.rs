#![feature(thin_box)]
#![feature(unsize)]

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub use storage::ErasedStorage;
use util::RwLock;

pub mod storage;

#[derive(Debug, Clone)]
pub struct DI(Arc<RwLock<ErasedStorage>>);

unsafe impl Send for DI {}

unsafe impl Sync for DI {}

impl Default for DI {
    fn default() -> Self {
        Self(Arc::new(RwLock::with_name(ErasedStorage::new(), "DI storage")))
    }
}

impl DI {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Deref for DI {
    type Target = Arc<RwLock<ErasedStorage>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DI {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
