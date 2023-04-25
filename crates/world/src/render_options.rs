#[derive(Debug)]
pub struct RenderOptions {
    pub tessellation_level: u32,
    pub wireframe: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            tessellation_level: 8,
            wireframe: false,
        }
    }
}
