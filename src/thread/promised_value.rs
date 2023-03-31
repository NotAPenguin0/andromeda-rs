use std::fmt::{Debug, Formatter};

use anyhow::Result;
use phobos::DeletionQueue;
use poll_promise::Promise;

/// Wrapper struct that stores both a current value and a promise value that will eventually replace the current
/// value.
pub struct PromisedValue<T: Send + 'static> {
    present: Option<T>,
    future: Option<Promise<Result<T>>>,
    deletion_queue: DeletionQueue<T>,
}

impl<T: Send + 'static> PromisedValue<T> {
    pub fn new() -> Self {
        Self {
            present: None,
            future: None,
            deletion_queue: DeletionQueue::new(4),
        }
    }

    #[allow(dead_code)]
    pub fn new_promise(promise: Promise<Result<T>>) -> Self {
        Self {
            present: None,
            future: Some(promise),
            deletion_queue: DeletionQueue::new(4),
        }
    }

    /// Poll the eventual future value and replace the old value if there was one.
    pub fn poll(&mut self) {
        if self
            .future
            .as_ref()
            .and_then(|promise| promise.ready())
            .is_some()
        {
            // We just verified that
            // - self.future is Some(promise)
            // - This promise is ready
            let promise = self.future.take().unwrap();
            let result = promise.block_and_take();
            // Now check if we had an old value, and if so push it on the deletion queue
            match self.present.take() {
                None => {}
                Some(value) => {
                    self.deletion_queue.push(value);
                }
            }
            self.present = match result {
                Ok(value) => Some(value),
                Err(err) => {
                    error!("Error inside promise: {err}");
                    None
                }
            }
        }
    }

    /// Get a reference to the current value, or None if we have nothing.
    pub fn value(&self) -> Option<&T> {
        self.present.as_ref()
    }

    #[allow(dead_code)]
    pub fn value_mut(&mut self) -> Option<&mut T> {
        self.present.as_mut()
    }

    pub fn take(&mut self) -> Option<T> {
        self.present.take()
    }

    /// Promise a new value that will replace the old value.
    /// If there was already a promise running, it will be dropped.
    pub fn promise(&mut self, future: Promise<Result<T>>) {
        self.future = Some(future);
    }
}

impl<T: Debug + Send + 'static> Debug for PromisedValue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PromisedValue(present: {:#?}, has_future: {:#?})",
            self.present,
            self.future.is_some()
        )
    }
}
