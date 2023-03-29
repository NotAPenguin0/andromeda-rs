use std::path::Path;

use anyhow::Result;
use poll_promise::Promise;

use crate::gfx::PairedImageView;

pub struct Texture {
    pub image: PairedImageView,
}

impl Texture {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Promise<Result<Self>> {
        trace!("Loading texture {path}");
    }
}
