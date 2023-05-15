use std::io::Read;
use std::path::PathBuf;

use anyhow::Result;
use tokio::io::AsyncReadExt;

/// Reads an entire file into a byte buffer and returns the result
pub async fn read_file_async<P: Into<PathBuf> + Send + 'static>(path: P) -> Result<Vec<u8>> {
    let path = path.into();
    let mut file = tokio::fs::File::open(&path).await?;
    let metadata = tokio::fs::metadata(&path).await?;
    let mut buffer = vec![0; metadata.len() as usize];
    file.read_exact(buffer.as_mut_slice()).await?;
    Ok(buffer)
}

/// Reads an entire file into a byte buffer and returns the result
pub fn read_file<P: Into<PathBuf> + Send + 'static>(path: P) -> Result<Vec<u8>> {
    let path = path.into();
    let mut file = std::fs::File::open(&path)?;
    let metadata = std::fs::metadata(&path)?;
    let mut buffer = vec![0; metadata.len() as usize];
    file.read_exact(buffer.as_mut_slice())?;
    Ok(buffer)
}
