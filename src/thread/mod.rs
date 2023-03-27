use poll_promise::Promise;

pub fn spawn_promise<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(func: F) -> Promise<T> {
    let (sender, promise) = Promise::new();
    rayon::spawn(move || {
        let value = func();
        sender.send(value);
    });
    promise
}
