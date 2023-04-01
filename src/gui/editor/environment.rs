use crate::gui::widgets::drag::Drag;
use crate::state::world::World;

pub fn show(context: &egui::Context, world: &mut World) {
    egui::Window::new("Environment Settings")
        .resizable(true)
        .movable(true)
        .show(context, |ui| {
            Drag::new("Sun direction", &mut world.sun_direction).show(ui);
            egui::CollapsingHeader::new("Atmosphere").show(ui, |ui| {
                Drag::new("Planet radius", &mut world.atmosphere.planet_radius)
                    .suffix(" km")
                    .scale(10e-4)
                    .show(ui);
                Drag::new("Atmosphere radius", &mut world.atmosphere.atmosphere_radius)
                    .suffix(" km")
                    .relative_to(world.atmosphere.planet_radius)
                    .scale(10e-4)
                    .show(ui);
                Drag::new("Sun intensity", &mut world.atmosphere.sun_intensity)
                    .speed(0.1)
                    .show(ui);
                Drag::new("Rayleigh scattering", &mut world.atmosphere.rayleigh_coefficients)
                    .speed(0.1)
                    .scale(10e5)
                    .digits(3)
                    .show(ui);
                Drag::new("Rayleigh scatter height", &mut world.atmosphere.rayleigh_scatter_height)
                    .suffix(" m")
                    .show(ui);
                Drag::new("Mie scattering", &mut world.atmosphere.mie_coefficients)
                    .speed(0.1)
                    .scale(10e4)
                    .digits(3)
                    .show(ui);
                Drag::new("Mie albedo", &mut world.atmosphere.mie_albedo)
                    .speed(0.01)
                    .show(ui);
                Drag::new("Mie G", &mut world.atmosphere.mie_g)
                    .speed(0.01)
                    .show(ui);
                Drag::new("Mie scatter height", &mut world.atmosphere.mie_scatter_height)
                    .suffix(" m")
                    .show(ui);
                Drag::new("Ozone scattering", &mut world.atmosphere.ozone_coefficients)
                    .speed(0.1)
                    .scale(10e7)
                    .digits(3)
                    .show(ui);
            });
        });
}
