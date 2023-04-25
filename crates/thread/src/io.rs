use anyhow::Result;
use poll_promise::Promise;
use std::io::Read;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

/// Reads an entire file into a byte buffer and returns the promised result.
/// Runs asynchronously in a tokio task.
pub fn read_file_async<P: Into<PathBuf> + Send + 'static>(path: P) -> Promise<Result<Vec<u8>>> {
    Promise::spawn_async(async move {
        let path = path.into();
        let mut file = tokio::fs::File::open(&path).await?;
        let metadata = tokio::fs::metadata(&path).await?;
        let mut buffer = vec![0; metadata.len() as usize];
        file.read_exact(buffer.as_mut_slice()).await?;
        Ok(buffer)
    })
}

/// Reads an entire file into a byte buffer and returns the promised result. Runs in a blocking
/// tokio task.
#[allow(dead_code)]
pub fn read_file<P: Into<PathBuf> + Send + 'static>(path: P) -> Promise<Result<Vec<u8>>> {
    Promise::spawn_blocking(move || {
        let path = path.into();
        let mut file = std::fs::File::open(&path)?;
        let metadata = std::fs::metadata(&path)?;
        let mut buffer = vec![0; metadata.len() as usize];
        file.read_exact(buffer.as_mut_slice())?;
        Ok(buffer)
    })
}
