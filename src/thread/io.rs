use std::path::{Path, PathBuf};

use anyhow::Result;
use poll_promise::Promise;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Reads an entire file into a byte buffer and returns the promised result.
/// Runs asynchronously in a tokio task.
pub fn read_file<P: Into<PathBuf> + Send + 'static>(path: P) -> Promise<Result<Vec<u8>>> {
    Promise::spawn_async(async move {
        let path = path.into();
        let mut file = File::open(&path).await?;
        let metadata = fs::metadata(&path).await?;
        let mut buffer = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut buffer).await?;
        Ok(buffer)
    })
}
