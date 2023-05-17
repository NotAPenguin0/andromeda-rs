use anyhow::Result;
use events::Tick;
use inject::DI;
use scheduler::{EventBus, EventContext, StoredSystem, System};
use world::World;

pub mod camera_controller;
pub mod environment;
pub mod performance;
pub mod render_options;
pub mod terrain_options;
pub mod world_view;

#[derive(Debug)]
pub struct Editor {
    context: egui::Context,
    bus: EventBus<DI>,
}

impl Editor {
    pub fn new(context: egui::Context, bus: EventBus<DI>) -> Self {
        Self {
            context,
            bus,
        }
    }

    pub fn show(&self, world: &mut World) {
        egui::CentralPanel::default().show(&self.context, |ui| {
            ui.heading("Editor");

            world_view::show(&self.context, &self.bus);
            environment::show(&self.context, world);
            render_options::show(&self.context, world);
            terrain_options::show(&self.context, &self.bus, world);
            performance::show(&self.context, &self.bus);
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
