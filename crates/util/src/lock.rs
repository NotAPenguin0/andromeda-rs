use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync;
use std::sync::{LockResult, PoisonError};

use log::trace;

/// Wrapper around a [`RwLock`] that provides additional logging and times
/// how long it is blocking threads. All features can be toggled using feature flags.
/// If no features are enabled, then this is a zero-cost wrapper around RwLock.
#[derive(Debug)]
pub struct RwLock<T> {
    lock: sync::RwLock<T>,
    name: Option<String>,
}

#[derive(Copy, Clone)]
enum LockIdentifier<'a> {
    Pointer(u64),
    Name(&'a str),
}

impl<'a> Display for LockIdentifier<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            LockIdentifier::Pointer(value) => {
                write!(f, "{value:x}")
            }
            LockIdentifier::Name(name) => {
                write!(f, "{name}")
            }
        }
    }
}

fn log_lock_operation(identifier: LockIdentifier, operation: &str, mode: &str) {
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("[unnamed thread]");
    trace!("lock [{identifier}] {operation} {mode} lock from thread [{thread_name}]");
}

pub struct RwLockReadGuard<'a, T> {
    guard: sync::RwLockReadGuard<'a, T>,
    identifier: LockIdentifier<'a>,
}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = sync::RwLockReadGuard<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        log_lock_operation(self.identifier, "released", "read");
    }
}

pub struct RwLockWriteGuard<'a, T> {
    guard: sync::RwLockWriteGuard<'a, T>,
    identifier: LockIdentifier<'a>,
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
        log_lock_operation(self.identifier, "released", "write");
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
        let result = self.lock.read();
        log_lock_operation(self.identifier(), "acquired", "read");
        match result {
            Ok(guard) => Ok(RwLockReadGuard {
                guard,
                identifier: self.identifier(),
            }),
            Err(poison) => Err(PoisonError::new(RwLockReadGuard {
                guard: poison.into_inner(),
                identifier: self.identifier(),
            })),
        }
    }

    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        let result = self.lock.write();
        log_lock_operation(self.identifier(), "acquired", "write");
        match result {
            Ok(guard) => Ok(RwLockWriteGuard {
                guard,
                identifier: self.identifier(),
            }),
            Err(poison) => Err(PoisonError::new(RwLockWriteGuard {
                guard: poison.into_inner(),
                identifier: self.identifier(),
            })),
        }
    }
}
