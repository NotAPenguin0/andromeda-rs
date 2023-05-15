use anyhow::Result;
use log::error;

/// Unwrap without panicking, instead printing a stack trace.
pub trait SafeUnwrap {
    type Output;

    fn safe_unwrap(self) -> Self::Output;
}

impl SafeUnwrap for Result<()> {
    type Output = ();

    fn safe_unwrap(self) -> Self::Output {
        match self {
            Ok(_) => {}
            Err(error) => {
                error!("{}", error);
            }
        }
    }
}

impl SafeUnwrap for Result<Vec<()>> {
    type Output = ();

    fn safe_unwrap(self) -> Self::Output {
        match self {
            Ok(_) => {}
            Err(error) => {
                error!("{}", error);
            }
        }
    }
}
