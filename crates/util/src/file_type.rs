use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum FileType {
    Png,
    Unknown(String),
}

impl<P: AsRef<Path>> From<P> for FileType {
    fn from(path: P) -> Self {
        let path = path.as_ref();
        let extension = path.extension().unwrap_or(OsStr::new(""));
        if extension == OsStr::new("png") {
            FileType::Png
        } else {
            FileType::Unknown(extension.to_str().unwrap_or("").to_string())
        }
    }
}
