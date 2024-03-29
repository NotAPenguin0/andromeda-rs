use glam::{Vec2, Vec3};

#[derive(Debug)]
pub struct WorldMousePosition {
    /// Holds a value if the mouse is over the world view,
    /// no value otherwise.
    pub screen_space: Option<Vec2>,
    /// Holds a value if the mouse position is over some geometry,
    /// no value otherwise.
    pub world_space: Option<Vec3>,
    /// If the mouse is over the terrain plane, holds the UV coordinates
    /// of the pixel the mouse is pointing at.
    pub terrain_uv: Option<Vec2>,
}
