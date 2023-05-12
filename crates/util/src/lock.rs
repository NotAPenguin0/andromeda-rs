use std::backtrace::{Backtrace, BacktraceStatus};
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync;
use std::sync::{LockResult, PoisonError};
use std::time::Duration;

use log::{info, trace, warn};
use tokio::select;

pub const RWLOCK_HOLD_WARN_TIMEOUT_MS: u64 = 100;
pub const RWLOCK_WAIT_WARN_TIMEOUT_MS: u64 = 100;

/// Wrapper around a [`RwLock`] that provides additional logging and times
/// how long it is blocking threads. All features can be toggled using feature flags.
/// If no features are enabled, then this is a zero-cost wrapper around RwLock.
#[derive(Debug)]
pub struct RwLock<T> {
    lock: sync::RwLock<T>,
    name: Option<String>,
}

#[derive(Clone, Copy, Debug)]
enum LockIdentifier<'a> {
    Pointer(u64),
    Name(&'a str),
}

#[derive(Copy, Clone, Debug)]
enum LockOperation {
    Acquire,
    Release,
}

#[derive(Copy, Clone, Debug)]
enum LockMode {
    Read,
    Write,
}

impl<'a> Display for LockIdentifier<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            LockIdentifier::Pointer(value) => {
                write!(f, "0x{value:X}")
            }
            LockIdentifier::Name(name) => {
                write!(f, "{name}")
            }
        }
    }
}

impl Display for LockOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LockOperation::Acquire => {
                write!(f, "Acq")
            }
            LockOperation::Release => {
                write!(f, "Rel")
            }
        }
    }
}

impl Display for LockMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LockMode::Read => {
                write!(f, "R")
            }
            LockMode::Write => {
                write!(f, "W")
            }
        }
    }
}

fn log_lock_operation(identifier: LockIdentifier, operation: LockOperation, mode: LockMode) {
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("unnamed thread");
    trace!("Lock: [{identifier}] [{operation}] [{mode}] from thread [{thread_name}]");
}

type Sender = tokio::sync::oneshot::Sender<()>;
type Receiver = tokio::sync::oneshot::Receiver<()>;

pub struct RwLockReadGuard<'a, T> {
    guard: sync::RwLockReadGuard<'a, T>,
    identifier: LockIdentifier<'a>,
    #[cfg(feature = "time-locks")]
    release_tx: Option<Sender>,
}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = sync::RwLockReadGuard<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        #[cfg(feature = "time-locks")]
        {
            let tx = self.release_tx.take().unwrap();
            let _ = tx.send(());
        }
        #[cfg(feature = "log-read-locks")]
        log_lock_operation(self.identifier, LockOperation::Release, LockMode::Read);
    }
}

pub struct RwLockWriteGuard<'a, T> {
    guard: sync::RwLockWriteGuard<'a, T>,
    identifier: LockIdentifier<'a>,
    #[cfg(feature = "time-locks")]
    release_tx: Option<Sender>,
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
    type Target = sync::RwLockWriteGuard<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        #[cfg(feature = "time-locks")]
        {
            let tx = self.release_tx.take().unwrap();
            let _ = tx.send(());
        }
        #[cfg(feature = "log-write-locks")]
        log_lock_operation(self.identifier, LockOperation::Release, LockMode::Write);
    }
}

async fn timeout_task(
    rx: Receiver,
    timeout: Duration,
    message: String,
    backtrace: Option<Backtrace>,
) {
    let timeout_fut = tokio::time::sleep(timeout);
    // If the timeout future completes before the channel receives a message that the lock was dropped,
    // we log a warning
    select! {
        _ = timeout_fut => {
            warn!("{message}");
            #[cfg(feature="log-lock-backtrace")]
            match backtrace {
                None => info!("Lock: no backtrace provided"),
                Some(backtrace) => {
                    let status = backtrace.status();
                    match status {
                        BacktraceStatus::Disabled => { warn!("Lock: Backtrace provided but not enabled. Run with RUST_BACKTRACE=1 to enable."); }
                        BacktraceStatus::Unsupported => { warn!("Lock: Backtrace not supported."); }
                        BacktraceStatus::Captured => { warn!("Lock: Backtrace provided: {backtrace}"); }
                        _ => { info!("Unhandled backtrace status value {status:?}. Maybe this was added in a future version of Rust?") }
                    }
                }
            }
        },
        _ = rx => {

        },
    }
}

impl<T> RwLock<T> {
    fn identifier(&self) -> LockIdentifier<'_> {
        match &self.name {
            None => {
                let ptr: *const RwLock<T> = self;
                LockIdentifier::Pointer(ptr as u64)
            }
            Some(name) => LockIdentifier::Name(name),
        }
    }

    fn spawn_timeout_task(
        timeout: Duration,
        message: String,
        backtrace: Option<Backtrace>,
    ) -> Sender {
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(timeout_task(rx, timeout, message, backtrace));
        tx
    }

    fn spawn_lock_hold_timeout_task(&self, mode: LockMode) -> Sender {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed thread").to_owned();
        let timeout = Duration::from_millis(RWLOCK_HOLD_WARN_TIMEOUT_MS);
        #[cfg(feature = "log-lock-backtrace")]
        let backtrace = Some(Backtrace::capture());
        #[cfg(not(feature = "log-lock-backtrace"))]
        let backtrace = None;
        Self::spawn_timeout_task(timeout, format!("Lock: [{}] [{mode}] was held for over {RWLOCK_HOLD_WARN_TIMEOUT_MS}ms on thread [{thread_name}]", self.identifier()), backtrace)
    }

    fn spawn_lock_wait_timeout_task(&self, mode: LockMode) -> Sender {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed thread").to_owned();
        let timeout = Duration::from_millis(RWLOCK_WAIT_WARN_TIMEOUT_MS);
        #[cfg(feature = "log-lock-backtrace")]
        let backtrace = Some(Backtrace::capture());
        #[cfg(not(feature = "log-lock-backtrace"))]
        let backtrace = None;
        Self::spawn_timeout_task(timeout, format!("Lock: [{}] [{mode}] has been waiting for over {RWLOCK_WAIT_WARN_TIMEOUT_MS}ms on thread [{thread_name}]", self.identifier()), backtrace)
    }

    fn acquire_read(&self) -> LockResult<sync::RwLockReadGuard<'_, T>> {
        #[cfg(feature = "time-locks")]
        let tx = self.spawn_lock_wait_timeout_task(LockMode::Read);
        let result = self.lock.read();
        #[cfg(feature = "time-locks")]
        let _ = tx.send(());
        #[cfg(feature = "log-write-locks")]
        log_lock_operation(self.identifier(), LockOperation::Acquire, LockMode::Write);
        result
    }

    fn acquire_write(&self) -> LockResult<sync::RwLockWriteGuard<'_, T>> {
        #[cfg(feature = "time-locks")]
        let tx = self.spawn_lock_wait_timeout_task(LockMode::Write);
        let result = self.lock.write();
        #[cfg(feature = "time-locks")]
        let _ = tx.send(());
        #[cfg(feature = "log-write-locks")]
        log_lock_operation(self.identifier(), LockOperation::Acquire, LockMode::Write);
        result
    }
}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            lock: sync::RwLock::new(value),
            name: None,
        }
    }

    pub fn with_name(value: T, name: impl Into<String>) -> Self {
        Self {
            lock: sync::RwLock::new(value),
            name: Some(name.into()),
        }
    }

    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        let result = self.acquire_read();
        #[cfg(feature = "time-locks")]
        let tx = self.spawn_lock_hold_timeout_task(LockMode::Read);
        match result {
            Ok(guard) => Ok(RwLockReadGuard {
                guard,
                identifier: self.identifier(),
                #[cfg(feature = "time-locks")]
                release_tx: Some(tx),
            }),
            Err(poison) => Err(PoisonError::new(RwLockReadGuard {
                guard: poison.into_inner(),
                identifier: self.identifier(),
                #[cfg(feature = "time-locks")]
                release_tx: Some(tx),
            })),
        }
    }

    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        let result = self.acquire_write();
        #[cfg(feature = "time-locks")]
        let tx = self.spawn_lock_hold_timeout_task(LockMode::Write);
        match result {
            Ok(guard) => Ok(RwLockWriteGuard {
                guard,
                identifier: self.identifier(),
                #[cfg(feature = "time-locks")]
                release_tx: Some(tx),
            }),
            Err(poison) => Err(PoisonError::new(RwLockWriteGuard {
                guard: poison.into_inner(),
                identifier: self.identifier(),
                #[cfg(feature = "time-locks")]
                release_tx: Some(tx),
            })),
        }
    }
}
