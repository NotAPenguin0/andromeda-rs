use anyhow::Result;
use brush::{Brush, BrushSettings, SmoothHeight};
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
    pub radius: u32,
}

/// Stores information on what kind of overlays must be drawn over the world view.
/// Access through DI.
#[derive(Debug, Default)]
pub struct WorldOverlayInfo {
    pub brush_decal: Option<BrushDecalInfo>,
}

#[derive(Debug)]
pub struct Editor {
    context: egui::Context,
    bus: EventBus<DI>,
    brush_widget: BrushWidget,
}

impl Editor {
    pub fn new(context: egui::Context, bus: EventBus<DI>) -> Self {
        Self {
            context,
            bus: bus.clone(),
            brush_widget: BrushWidget {
                bus,
                settings: Default::default(),
                active_brush: Some(Brush::new(SmoothHeight {})),
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

        self.context.request_repaint();
    }
}

impl System<DI> for Editor {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>)
    where
        Self: Sized, {
        event_bus.subscribe(system, handle_editor_tick);
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
