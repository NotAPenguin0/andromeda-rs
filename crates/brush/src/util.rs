use glam::Vec3;

/// Returns true if the position is on the terrain mesh, false if outside.
pub fn position_on_terrain(position: Vec3) -> bool {
    // If any of the values inside the position are NaN or infinite, the position is outside
    // of the rendered terrain mesh and we do not want to actually use the brush.
    !position.is_nan() && position.is_finite()
}
