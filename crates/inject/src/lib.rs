use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use dyn_inject::ErasedStorage;

#[derive(Debug, Clone)]
pub struct DI(Arc<RwLock<ErasedStorage>>);

unsafe impl Send for DI {}

unsafe impl Sync for DI {}

impl Default for DI {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(ErasedStorage::new())))
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
