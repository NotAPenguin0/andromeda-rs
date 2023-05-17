use assets::TerrainOptions;
use glam::{Vec2, Vec3};

pub fn terrain_uv_at(world_pos: Vec3, options: &TerrainOptions) -> Vec2 {
    // First compute outer bounds of the terrain mesh
    let min_x = options.min_x();
    let min_y = options.min_y();
    let max_x = options.max_x();
    let max_y = options.max_y();
    // Then we get the length of the terrain in each dimension
    let dx = (max_x - min_x).abs();
    let dy = (max_y - min_y).abs();
    // Now we can simple calculate the ratio between world_pos and the length in each dimension
    // to get the uvs.
    // Note that we use the z coordinate since y is up, and our terrain is in the flat plane.
    // We will assume our terrain is properly centered. In this case, the uvs we get are
    // in the [-0.5, 0.5] range, so we need to remap them to [0, 1]. Since this range is
    // of the same size, we can just add 0.5
    let uv = Vec2::new(world_pos.x / dx, world_pos.z / dy);
    uv + 0.5
}
