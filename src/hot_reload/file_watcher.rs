use std::fmt::Debug;
use std::fs;
use std::path::Path;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use futures::channel::mpsc::{channel, Receiver};
use notify::Watcher;

pub fn create_async_watcher() -> Result<(notify::RecommendedWatcher, Receiver<Result<notify::Event>>)> {
    let (mut tx, rx) = channel(1);

    let watcher = notify::RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            futures::executor::block_on(async {
                tx.send(res.map_err(|e| -> anyhow::Error { e.into() }))
                    .await
                    .unwrap();
            })
        },
        notify::Config::default(),
    )?;

    Ok((watcher, rx))
}

pub async fn async_watch<P, F>(path: P, recursive: bool, callback: F) -> Result<()>
    where
        P: AsRef<Path> + Debug,
        F: Fn(notify::Event),
{
    let (mut watcher, mut rx) = create_async_watcher()?;

    info!(
        "Starting file watcher for path {:?}, recursive = {:?}",
        path, recursive
    );
    let path = match fs::canonicalize(path.as_ref()) {
        Ok(path) => path,
        Err(_) => {
            error!("Invalid path {:?}", path);
            panic!()
        }
    };

    watcher.watch(
        path.as_ref(),
        if recursive {
            notify::RecursiveMode::Recursive
        } else {
            notify::RecursiveMode::NonRecursive
        },
    )?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => (callback)(event),
            Err(err) => {
                error!("File watcher error: {:?}", err);
            }
        }
    }

    Ok(())
}
