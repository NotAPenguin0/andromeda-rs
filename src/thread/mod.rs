use poll_promise::Promise;

pub fn spawn_promise<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(func: F) -> Promise<T> {
    let (sender, promise) = Promise::new();
    rayon::spawn(move || {
        let value = func();
        sender.send(value);
    });
    promise
}

pub struct SendSyncPtr<T> {
    pointer: *const T,
}

impl<T> SendSyncPtr<T> {
    pub unsafe fn new(pointer: *const T) -> Self {
        Self {
            pointer,
        }
    }

    pub unsafe fn get(&self) -> *const T {
        self.pointer
    }
}

unsafe impl<T> Send for SendSyncPtr<T> {}

unsafe impl<T> Sync for SendSyncPtr<T> {}
