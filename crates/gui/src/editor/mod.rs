use std::time::Duration;

use anyhow::Result;
use brush::BrushSettings;
use derivative::Derivative;
use egui_notify::{ToastLevel, Toasts};
use error::{MessageEvent, MessageLevel};
use events::Tick;
use inject::DI;
use scheduler::{EventBus, EventContext, StoredSystem, System};
use util::SafeUnwrap;
use world::World;

use crate::editor::brushes::BrushWidget;

pub mod brushes;
pub mod camera_controller;
pub mod environment;
pub mod performance;
pub mod render_options;
pub mod terrain_options;
pub mod world_view;

#[derive(Debug)]
pub struct BrushDecalInfo {
    /// Radius of the brush decal, in texels on the heightmap.
    pub radius: f32,
    /// Extra data that is passed to the shader if present.
    /// Note that if this is present, the data MUST be used in the shader.
    pub data: Option<[f32; 4]>,
    /// Shader used for the decal
    pub shader: String,
}

/// Stores information on what kind of overlays must be drawn over the world view.
/// Access through DI.
#[derive(Debug, Default)]
pub struct WorldOverlayInfo {
    pub brush_decal: Option<BrushDecalInfo>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Editor {
    context: egui::Context,
    #[derivative(Debug = "ignore")]
    notify: Toasts,
    bus: EventBus<DI>,
    brush_widget: BrushWidget,
}

impl Editor {
    pub fn new(context: egui::Context, bus: EventBus<DI>) -> Self {
        let notify = Toasts::default();
        Self {
            context,
            notify,
            bus: bus.clone(),
            brush_widget: BrushWidget {
                bus,
                settings: BrushSettings {
                    radius: 32.0,
                    weight: 1.0,
                    invert: false,
                    once: false,
                },
                active_brush: None,
            },
        }
    }

    pub fn show(&mut self, world: &mut World) {
        egui::CentralPanel::default().show(&self.context, |ui| {
            ui.heading("Editor");

            world_view::show(&self.context, &self.bus, &mut self.brush_widget);
            environment::show(&self.context, world);
            render_options::show(&self.context, world);
            terrain_options::show(&self.context, &self.bus, world);
            performance::show(&self.context, &self.bus);
            self.brush_widget.show(&self.context).safe_unwrap();
        });

        // Show all notifications
        self.notify.show(&self.context);
        self.context.request_repaint();
    }
}

impl System<DI> for Editor {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>)
    where
        Self: Sized, {
        event_bus.subscribe(system, handle_editor_tick);
        event_bus.subscribe_sink(system, handle_error_sink);
    }
}

/// # DI Access
/// - Write [`World`]
fn handle_editor_tick(
    editor: &mut Editor,
    _event: &Tick,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    let inject = ctx.read().unwrap();
    let mut world = inject.write_sync::<World>().unwrap();
    editor.show(&mut world);
    Ok(())
}

fn to_toast_level(lvl: MessageLevel) -> ToastLevel {
    match lvl {
        MessageLevel::Success => ToastLevel::Success,
        MessageLevel::Info => ToastLevel::Info,
        MessageLevel::Warning => ToastLevel::Warning,
        MessageLevel::Error => ToastLevel::Error,
    }
}

fn handle_error_sink(
    editor: &mut Editor,
    event: MessageEvent,
    _ctx: &mut EventContext<DI>,
) -> Result<()> {
    editor
        .notify
        .basic(event.message)
        .set_level(to_toast_level(event.level))
        .set_closable(true)
        .set_duration(Some(Duration::from_secs(3)));
    Ok(())
}
