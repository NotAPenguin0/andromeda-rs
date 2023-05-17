use ::util::mouse_position::WorldMousePosition;
use ::util::SafeUnwrap;
use anyhow::Result;
use events::DragWorldView;
use gfx::SharedContext;
use glam::Vec3;
use hot_reload::IntoDynamic;
use inject::DI;
use phobos::ComputePipelineBuilder;
use scheduler::{Event, EventBus, EventContext, StoredSystem, System};

pub mod brushes;
pub mod util;

type BrushEventReceiver = tokio::sync::mpsc::Receiver<BrushEvent>;
type BrushEventSender = tokio::sync::mpsc::Sender<BrushEvent>;

#[derive(Debug)]
struct BrushSystem {
    event_sender: BrushEventSender,
}

impl BrushSystem {
    pub fn new(tx: BrushEventSender) -> Self {
        Self {
            event_sender: tx,
        }
    }
}

impl System<DI> for BrushSystem {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>) {
        event_bus.subscribe(system, handle_drag_world_view);
        event_bus.subscribe(system, handle_begin_stroke);
        event_bus.subscribe(system, handle_end_stroke);
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Brush {
    SmoothHeight,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct BrushSettings {
    pub radius: f32,
    pub weight: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct BeginStrokeEvent {
    pub settings: BrushSettings,
    pub brush: Brush,
}

pub struct EndStrokeEvent;

impl Event for BeginStrokeEvent {}
impl Event for EndStrokeEvent {}

#[derive(Debug)]
enum BrushEvent {
    BeginStroke {
        settings: BrushSettings,
        brush: Brush,
    },
    StrokeAt(Vec3),
    EndStroke,
}

fn dispatch_brush_fn(
    bus: &EventBus<DI>,
    position: Vec3,
    brush: &Brush,
    settings: &BrushSettings,
) -> Result<()> {
    match brush {
        Brush::SmoothHeight => brushes::height::apply(bus, position, settings),
    }
}

fn brush_task(bus: EventBus<DI>, mut recv: BrushEventReceiver) {
    let mut current_settings = BrushSettings::default();
    let mut current_brush = None;

    // While the sender is not dropped, we can keep waiting for events
    while let Some(event) = recv.blocking_recv() {
        match event {
            BrushEvent::BeginStroke {
                settings,
                brush,
            } => {
                current_brush = Some(brush);
                current_settings = settings;
            }
            BrushEvent::StrokeAt(position) => {
                // Only actually stroke if a brush is active
                match &current_brush {
                    None => {}
                    Some(brush) => {
                        dispatch_brush_fn(&bus, position, brush, &current_settings).safe_unwrap();
                    }
                }
            }
            BrushEvent::EndStroke => {
                current_brush = None;
            }
        }
    }
}

fn handle_drag_world_view(
    system: &mut BrushSystem,
    _drag: &DragWorldView,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    let di = ctx.read().unwrap();
    let mouse = di.read_sync::<WorldMousePosition>().unwrap();
    match mouse.world_space {
        None => {}
        Some(pos) => {
            system
                .event_sender
                .blocking_send(BrushEvent::StrokeAt(pos))?;
        }
    };
    Ok(())
}

fn handle_begin_stroke(
    system: &mut BrushSystem,
    stroke: &BeginStrokeEvent,
    _ctx: &mut EventContext<DI>,
) -> Result<()> {
    system.event_sender.blocking_send(BrushEvent::BeginStroke {
        settings: stroke.settings,
        brush: stroke.brush,
    })?;
    Ok(())
}

fn handle_end_stroke(
    system: &mut BrushSystem,
    _stroke: &EndStrokeEvent,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    system.event_sender.blocking_send(BrushEvent::EndStroke)?;
    Ok(())
}

fn create_brush_pipeline(bus: &EventBus<DI>) -> Result<()> {
    let di = bus.data().read().unwrap();
    let gfx = di.get::<SharedContext>().cloned().unwrap();
    ComputePipelineBuilder::new("height_brush")
        // Make sure this pipeline is persistent so we don't constantly recompile it
        .persistent()
        .into_dynamic()
        .set_shader("shaders/src/height_brush.cs.hlsl")
        .build(bus, gfx.pipelines.clone())?;
    ComputePipelineBuilder::new("normal_recompute")
        .persistent()
        .into_dynamic()
        .set_shader("shaders/src/normal_recompute.cs.hlsl")
        .build(bus, gfx.pipelines.clone())?;
    ComputePipelineBuilder::new("blur_rect")
        .persistent()
        .into_dynamic()
        .set_shader("shaders/src/blur_rect.cs.hlsl")
        .build(bus, gfx.pipelines)?;
    Ok(())
}

pub fn initialize(bus: &EventBus<DI>) -> Result<()> {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let system = BrushSystem::new(tx);
    bus.add_system(system);
    create_brush_pipeline(bus)?;
    let bus = bus.clone();
    tokio::task::spawn_blocking(|| brush_task(bus, rx));
    Ok(())
}
