use rayon::Yield;

pub mod io;
pub mod promise;
pub mod promised_value;

pub struct SendSyncPtr<T> {
    pointer: *const T,
}

impl<T> SendSyncPtr<T> {
    /// # Safety
    /// The caller must guarantee this pointer is accessed in a safe manner
    pub unsafe fn new(pointer: *const T) -> Self {
        Self {
            pointer,
        }
    }

    /// # Safety
    /// The caller must guarantee this pointer is accessed in a safe manner
    pub unsafe fn get(&self) -> *const T {
        self.pointer
    }
}

unsafe impl<T> Send for SendSyncPtr<T> {}

unsafe impl<T> Sync for SendSyncPtr<T> {}

pub fn yield_now() {
    match rayon::yield_now() {
        Some(Yield::Executed) => {}
        _ => std::thread::yield_now(),
    }
}
