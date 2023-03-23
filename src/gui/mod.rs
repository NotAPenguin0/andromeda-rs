use egui::{Color32, Pos2, Rect, Sense};
use tiny_tokio_actor::{async_trait, Actor, ActorContext, Handler, Message, SystemEvent};
use tokio::runtime::Handle;

use crate::app::RootActorSystem;
use crate::gfx::world::World;
use crate::gui::editor::camera_controller::{control_camera, DragWorld, MouseOverWorld};
use crate::gui::util::image::Image;
use crate::gui::util::size::USize;
use crate::gui::widgets::drag::Drag;

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
            Drag::new("Sun direction", &mut world.sun_direction).show(ui);
            egui::CollapsingHeader::new("Atmosphere").show(ui, |ui| {
                Drag::new("Planet radius", &mut world.atmosphere.planet_radius)
                    .suffix(" km")
                    .scale(10e-4)
                    .show(ui);
                Drag::new("Atmosphere radius", &mut world.atmosphere.atmosphere_radius)
                    .suffix(" km")
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
                    .suffix(" km")
                    .scale(10e-4)
                    .show(ui);
                Drag::new("Mie scattering", &mut world.atmosphere.mie_coefficients)
                    .speed(0.1)
                    .scale(10e4)
                    .digits(3)
                    .show(ui);
                Drag::new("Mie albedo", &mut world.atmosphere.mie_albedo).speed(0.01).show(ui);
                Drag::new("Mie G", &mut world.atmosphere.mie_g).speed(0.01).show(ui);
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

pub fn build_ui(context: &egui::Context, actors: &RootActorSystem, world: &mut World) {
    egui::CentralPanel::default().show(&context, |ui| {
        ui.heading("Editor");

        editor::world_view::show(&context, &actors);
        environment_panel(context, world);
    });
}
