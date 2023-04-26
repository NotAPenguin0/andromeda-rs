use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use dyn_inject::Registry;

#[derive(Debug, Clone)]
pub struct DI(Arc<RwLock<Registry>>);

unsafe impl Send for DI {}

unsafe impl Sync for DI {}

impl DI {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(Registry::new())))
    }
}

impl Deref for DI {
    type Target = Arc<RwLock<Registry>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DI {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
