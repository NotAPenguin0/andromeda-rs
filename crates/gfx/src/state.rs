use glam::{Mat4, UVec2, Vec3};
use phobos::VirtualResource;

pub struct SceneResources {
    pub color: VirtualResource,
    pub depth: VirtualResource,
}

/// Stores world state in a format that the renderer needs, such as
/// normalized direction vectors instead of rotations,
/// camera view and projection matrices, etc.
#[derive(Debug, Default)]
pub struct RenderState {
    /// Camera view matrix
    pub view: Mat4,
    /// Camera projection matrix
    pub projection: Mat4,
    /// Premultiplied `projection * view` matrix
    pub projection_view: Mat4,
    /// Inverse of the projection matrix
    pub inverse_projection: Mat4,
    /// Inverse of the view matrix
    pub inverse_view: Mat4,
    /// Inverse of `projection * view`
    pub inverse_projection_view: Mat4,
    /// Inverse of the camera's view matrix with the translation component removed
    pub inverse_view_rotation: Mat4,
    /// Direction vector pointing away from the sun
    pub sun_direction: Vec3,
    /// Camera position in world space
    pub cam_position: Vec3,
    /// Main render target size in pixels
    pub render_size: UVec2,
    /// Camera near plane
    pub near: f32,
    /// Camera far plane
    pub far: f32,
    /// Camera FOV
    pub fov: f32,
    /// Previous projection-view matrix
    pub previous_pv: Mat4,
}
