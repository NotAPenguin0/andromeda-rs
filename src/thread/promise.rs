use anyhow::Result;
use poll_promise::Promise;

use crate::thread::yield_now;

pub trait SpawnPromise {
    type Output: Send + 'static;

    fn spawn<F: FnOnce() -> Self::Output + Send + 'static>(func: F) -> Self;
}

impl<T: Send + 'static> SpawnPromise for Promise<T> {
    type Output = T;

    /// Spawns a new promise in a rayon task.
    fn spawn<F: FnOnce() -> Self::Output + Send + 'static>(func: F) -> Self {
        let (sender, promise) = Self::new();
        rayon::spawn(move || {
            let value = func();
            sender.send(value);
        });
        promise
    }
}

pub trait WaitAndYield {
    type Output;

    fn wait_and_yield(self) -> Self::Output;
}

impl<T: Send + 'static> WaitAndYield for Promise<T> {
    type Output = T;

    /// Wait for a promise cooperatively by yielding to either rayon or the OS.
    /// Note: this seems to have some issues
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

    /// Spawns a new synchronous promise that completes with `(value, func(&value))`,
    /// where `value` is the value that `self` completes with.
    fn then<U: Send + 'static, F: FnOnce(&Self::Output) -> U + Send + 'static>(
        self,
        func: F,
    ) -> Promise<(Self::Output, U)>;
}

impl<T: Send + 'static> ThenPromise for Promise<T> {
    type Output = T;

    fn then<U: Send + 'static, F: FnOnce(&T) -> U + Send + 'static>(
        self,
        func: F,
    ) -> Promise<(T, U)> {
        Promise::<(T, U)>::spawn_blocking(move || {
            let first = self.block_and_take();
            let second = func(&first);
            (first, second)
        })
    }
}

pub trait ThenTry {
    type Output: Send;

    /// Spawns a new promise that completes with
    /// - `Err(e)` if `self` completes with `Err(e)`
    /// - `Err(e)` if `self` completes with `Ok(value)` and `func(&value)` returns `Err(e)`
    /// - `Ok(value, second)` if `self` completes with `Ok(value)` and `func(&value)` returns `Ok(second)`
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
        Promise::<Result<(T, U)>>::spawn_blocking(move || {
            let first = self.block_and_take()?;
            let second = func(&first)?;
            Ok((first, second))
        })
    }
}

pub trait ThenMap {
    type Output: Send;

    /// Spawn a new promise that completes with `func(value)`, where `value` is the value `self` completes with.
    fn then_map<U: Send + 'static, F: FnOnce(Self::Output) -> U + Send + 'static>(
        self,
        func: F,
    ) -> Promise<U>;
}

impl<T: Send + 'static> ThenMap for Promise<T> {
    type Output = T;

    fn then_map<U: Send + 'static, F: FnOnce(T) -> U + Send + 'static>(
        self,
        func: F,
    ) -> Promise<U> {
        Promise::<U>::spawn_blocking(move || {
            let value = self.block_and_take();
            func(value)
        })
    }
}

pub trait ThenTryMap {
    type Output: Send;

    /// Spawn a new synchronous promise that completes with
    /// - `Err(e)` if `self` completed with `Err(e)`
    /// - `Err(e)` if `self` completed with `Ok(value)` and `func(value)` returned `Err(e)`
    /// - `Ok(second)` if `self` completed with `Ok(value)` and `func(value)` returned `Ok(second)`
    fn then_try_map<U: Send + 'static, F: FnOnce(Self::Output) -> Result<U> + Send + 'static>(
        self,
        func: F,
    ) -> Promise<Result<U>>;
}

impl<T: Send + 'static> ThenTryMap for Promise<Result<T>> {
    type Output = T;

    fn then_try_map<U: Send + 'static, F: FnOnce(T) -> Result<U> + Send + 'static>(
        self,
        func: F,
    ) -> Promise<Result<U>> {
        Promise::<Result<U>>::spawn_blocking(move || {
            let value = self.block_and_take()?;
            func(value)
        })
    }
}

pub trait ThenInto {
    type Output: Send;

    /// Spawns a new synchronous promise that completes with `value.into()`, where `value` is the value
    /// `self` completes with
    fn then_into<U: Send + 'static>(self) -> Promise<U>
    where
        Self::Output: Into<U>;
}

impl<T: Send + 'static> ThenInto for Promise<T> {
    type Output = T;

    fn then_into<U: Send + 'static>(self) -> Promise<U>
    where
        T: Into<U>, {
        Promise::spawn_blocking(move || {
            let value = self.block_and_take();
            value.into()
        })
    }
}

pub trait ThenTryInto {
    type Output: Send;

    /// Spawns a new synchronous promise that completes with
    /// - `Err(e)` if `self` completed with `Err(e)`
    /// - `Ok(value.into())` if `self` completed with `Ok(value)`
    fn then_try_into<U: Send + 'static>(self) -> Promise<Result<U>>
    where
        Self::Output: Into<U>;
}

impl<T: Send + 'static> ThenTryInto for Promise<Result<T>> {
    type Output = T;

    fn then_try_into<U: Send + 'static>(self) -> Promise<Result<U>>
    where
        T: Into<U>, {
        Promise::spawn_blocking(move || {
            let value = self.block_and_take()?;
            Ok(value.into())
        })
    }
}
