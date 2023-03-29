use anyhow::Result;
use poll_promise::Promise;

use crate::thread::yield_now;

pub fn spawn_promise<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(func: F) -> Promise<T> {
    let (sender, promise) = Promise::new();
    rayon::spawn(move || {
        let value = func();
        sender.send(value);
    });
    promise
}

pub trait WaitAndYield {
    type Output;

    fn wait_and_yield(self) -> Self::Output;
}

impl<T: Send + 'static> WaitAndYield for Promise<T> {
    type Output = T;

    /// Wait for a promise cooperatively by yielding to either rayon or the OS.
    fn wait_and_yield(self) -> Self::Output {
        loop {
            if self.poll().is_ready() {
                // Does not actually block, since we just verified this is ready
                return self.block_and_take();
            }
            yield_now();
        }
    }
}

pub trait ThenPromise {
    type Output: Send;

    fn then<U: Send + 'static, F: FnOnce(&Self::Output) -> U + Send + 'static>(
        self,
        func: F,
    ) -> Promise<(Self::Output, U)>;
}

impl<T: Send + 'static> ThenPromise for Promise<T> {
    type Output = T;

    /// After the first promise completes, spawns a new promise with the previous one as its argument. Returns a promise with both values.
    fn then<U: Send + 'static, F: FnOnce(&T) -> U + Send + 'static>(
        self,
        func: F,
    ) -> Promise<(T, U)> {
        spawn_promise(move || {
            let first = self.wait_and_yield();
            let second = func(&first);
            (first, second)
        })
    }
}

pub trait ThenTry {
    type Output: Send;

    fn then_try<U: Send + 'static, F: FnOnce(&Self::Output) -> Result<U> + Send + 'static>(
        self,
        func: F,
    ) -> Promise<Result<(Self::Output, U)>>;
}

impl<T: Send + 'static> ThenTry for Promise<Result<T>> {
    type Output = T;

    fn then_try<U: Send + 'static, F: FnOnce(&T) -> Result<U> + Send + 'static>(
        self,
        func: F,
    ) -> Promise<Result<(T, U)>> {
        spawn_promise(move || {
            let first = self.wait_and_yield()?;
            let second = func(&first)?;
            Ok((first, second))
        })
    }
}

pub trait MapPromise {
    type Output: Send;

    fn map<U: Send + 'static, F: FnOnce(Self::Output) -> U + Send + 'static>(
        self,
        func: F,
    ) -> Promise<U>;
}

impl<T: Send + 'static> MapPromise for Promise<T> {
    type Output = T;

    fn map<U: Send + 'static, F: FnOnce(T) -> U + Send + 'static>(self, func: F) -> Promise<U> {
        spawn_promise(move || {
            let value = self.wait_and_yield();
            func(value)
        })
    }
}
