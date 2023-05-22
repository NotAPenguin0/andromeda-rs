use scheduler::Event;

pub struct ErrorEvent {
    pub message: String,
}

impl Event for ErrorEvent {}

#[macro_export]
macro_rules! publish_error {
    ($bus:ident, $fmt:expr) => {
        $bus.publish($crate::ErrorEvent { message: format!($fmt) })
    };

    ($bus:ident, $fmt:expr, $($args:tt)*) => {
        $bus.publish($crate::ErrorEvent { message: format!($fmt, $($args)*) })
    };
}
