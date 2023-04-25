use glam::Vec3;

#[derive(Debug, Default, Copy, Clone)]
pub struct AtmosphereInfo {
    pub planet_radius: f32,
    pub atmosphere_radius: f32,
    pub rayleigh_coefficients: Vec3,
    pub rayleigh_scatter_height: f32,
    pub mie_coefficients: Vec3,
    pub mie_albedo: f32,
    pub mie_scatter_height: f32,
    pub mie_g: f32,
    pub ozone_coefficients: Vec3,
    pub sun_intensity: f32,
}

impl AtmosphereInfo {
    /// Returns earth-like atmosphere parameters
    pub fn earth() -> Self {
        Self {
            planet_radius: 6371000.0,
            atmosphere_radius: 6471000.0,
            rayleigh_coefficients: Vec3::new(0.0000058, 0.0000133, 0.00003331),
            rayleigh_scatter_height: 8000.0,
            mie_coefficients: Vec3::new(0.000021, 0.000021, 0.000021),
            mie_albedo: 0.9,
            mie_scatter_height: 1200.0,
            mie_g: 0.8,
            ozone_coefficients: Vec3::new(0.0000007729596, 0.00000066771764, 0.00000007049316),
            sun_intensity: 22.0,
        }
    }
}
