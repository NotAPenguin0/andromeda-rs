#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum FileType {
    Png,
    NetCDF,
    Unknown(String),
}
