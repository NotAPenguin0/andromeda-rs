use egui::{Color32, Pos2, Rect, Sense};
use tiny_tokio_actor::{async_trait, Actor, ActorContext, Handler, Message, SystemEvent};
use tokio::runtime::Handle;

use crate::app::RootActorSystem;
use crate::gfx::world::World;
use crate::gui::editor::camera_controller::{control_camera, DragWorld, MouseOverWorld};
use crate::gui::util::format::{format_km, format_meters, parse_km, parse_meters};
use crate::gui::util::image::Image;
use crate::gui::util::size::USize;
use crate::gui::widgets::drag::{drag, drag3_angle, drag3_scaled, drag_fmt_scaled};

pub mod editor;
pub mod util;
pub mod widgets;

#[derive(Debug, Copy, Clone)]
pub struct ResizeSceneTexture(USize);

impl Message for ResizeSceneTexture {
    type Response = Option<Image>;
}

#[derive(Debug, Copy, Clone)]
pub struct QuerySceneTextureSize;

impl Message for QuerySceneTextureSize {
    type Response = Option<USize>;
}

#[derive(Debug, Copy, Clone)]
pub struct QueryCurrentSceneTexture;

impl Message for QueryCurrentSceneTexture {
    type Response = Option<Image>;
}

#[derive(Debug, Copy, Clone)]
pub struct SetNewTexture(pub Image);

impl Message for SetNewTexture {
    type Response = ();
}

#[derive(Default)]
pub struct TargetResizeActor {
    current_image: Option<Image>,
    new_size: Option<USize>,
}

impl<E> Actor<E> for TargetResizeActor where E: SystemEvent {}

#[async_trait]
impl<E> Handler<E, ResizeSceneTexture> for TargetResizeActor
where
    E: SystemEvent,
{
    async fn handle(&mut self, msg: ResizeSceneTexture, _ctx: &mut ActorContext<E>) -> Option<Image> {
        if let Some(cur) = &self.current_image {
            if cur.size != msg.0 {
                self.new_size = Some(msg.0);
            }
        } else {
            self.new_size = Some(msg.0);
        }

        return self.current_image;
    }
}

#[async_trait]
impl<E> Handler<E, QuerySceneTextureSize> for TargetResizeActor
where
    E: SystemEvent,
{
    async fn handle(&mut self, _msg: QuerySceneTextureSize, _ctx: &mut ActorContext<E>) -> Option<USize> {
        self.new_size
    }
}

#[async_trait]
impl<E> Handler<E, QueryCurrentSceneTexture> for TargetResizeActor
where
    E: SystemEvent,
{
    async fn handle(&mut self, _msg: QueryCurrentSceneTexture, _ctx: &mut ActorContext<E>) -> Option<Image> {
        self.current_image
    }
}

#[async_trait]
impl<E> Handler<E, SetNewTexture> for TargetResizeActor
where
    E: SystemEvent,
{
    async fn handle(&mut self, msg: SetNewTexture, _ctx: &mut ActorContext<E>) -> () {
        self.current_image = Some(msg.0);
        self.new_size = None;
    }
}

fn environment_panel(context: &egui::Context, world: &mut World) {
    egui::Window::new("Environment Settings")
        .resizable(true)
        .movable(true)
        .show(&context, |ui| {
            drag3_angle(ui, "Sun direction", &mut world.sun_direction.0);
            egui::CollapsingHeader::new("Atmosphere").show(ui, |ui| {
                drag_fmt_scaled(
                    ui,
                    "Planet radius",
                    format_km,
                    parse_km,
                    &mut world.atmosphere.planet_radius,
                    1.0,
                    10e-4,
                );
                drag_fmt_scaled(
                    ui,
                    "Atmosphere radius",
                    format_km,
                    parse_km,
                    &mut world.atmosphere.atmosphere_radius,
                    1.0,
                    10e-4,
                );
                drag(ui, "Sun intensity", &mut world.atmosphere.sun_intensity, 0.1);
                drag3_scaled(
                    ui,
                    "Rayleigh scattering",
                    &mut world.atmosphere.rayleigh_coefficients,
                    0.1,
                    10e5,
                    3,
                );
                drag_fmt_scaled(
                    ui,
                    "Rayleigh scatter height",
                    format_km,
                    parse_km,
                    &mut world.atmosphere.rayleigh_scatter_height,
                    1.0,
                    10e-4,
                );
                drag3_scaled(ui, "Mie scattering", &mut world.atmosphere.mie_coefficients, 0.1, 10e4, 3);
                drag(ui, "Mie albedo", &mut world.atmosphere.mie_albedo, 0.01);
                drag(ui, "Mie G", &mut world.atmosphere.mie_g, 0.01);
                drag_fmt_scaled(
                    ui,
                    "Mie scatter height",
                    format_meters,
                    parse_meters,
                    &mut world.atmosphere.mie_scatter_height,
                    1.0,
                    1.0,
                );
                drag3_scaled(ui, "Ozone scattering", &mut world.atmosphere.ozone_coefficients, 0.1, 10e7, 3);
            });
        });
}

pub fn build_ui(context: &egui::Context, actors: &RootActorSystem, world: &mut World) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        editor::world_view::show(&context, &actors);
        environment_panel(context, world);
    });
}
