use scheduler::Event;

pub enum MessageLevel {
    Success,
    Info,
    Warning,
    Error,
}

pub struct MessageEvent {
    pub level: MessageLevel,
    pub message: String,
}

impl Event for MessageEvent {}

#[macro_export]
macro_rules! publish_error {
    ($bus:ident, $fmt:expr) => {
        let _ = $bus.publish($crate::MessageEvent { level: $crate::MessageLevel::Error, message: format!($fmt) });
    };

    ($bus:ident, $fmt:expr, $($args:tt)*) => {
        let _ = $bus.publish($crate::MessageEvent { level: $crate::MessageLevel::Error, message: format!($fmt, $($args)*) });
    };
}

#[macro_export]
macro_rules! publish_success {
    ($bus:ident, $fmt:expr) => {
        let _ = $bus.publish($crate::MessageEvent { level: $crate::MessageLevel::Success, message: format!($fmt) });
    };

    ($bus:ident, $fmt:expr, $($args:tt)*) => {
        let _ = $bus.publish($crate::MessageEvent { level: $crate::MessageLevel::Success, message: format!($fmt, $($args)*) });
    };
}

#[macro_export]
macro_rules! publish_info {
    ($bus:ident, $fmt:expr) => {
        let _ = $bus.publish($crate::MessageEvent { level: $crate::MessageLevel::Info, message: format!($fmt) });
    };

    ($bus:ident, $fmt:expr, $($args:tt)*) => {
        let _ = $bus.publish($crate::MessageEvent { level: $crate::MessageLevel::Info, message: format!($fmt, $($args)*) });
    };
}

#[macro_export]
macro_rules! publish_warn {
    ($bus:ident, $fmt:expr) => {
        let _ = $bus.publish($crate::MessageEvent { level: $crate::MessageLevel::Warning, message: format!($fmt) });
    };

    ($bus:ident, $fmt:expr, $($args:tt)*) => {
        let _ = $bus.publish($crate::MessageEvent { level: $crate::MessageLevel::Warning, message: format!($fmt, $($args)*) });
    };
}
