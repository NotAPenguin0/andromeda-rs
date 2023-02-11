use tiny_tokio_actor::*;
use std::sync::atomic::{AtomicBool, Ordering};

use atomic_enum::atomic_enum;

use crate::event::Event;

#[derive(Clone, Copy)]
pub struct RepaintAll;

impl Message for RepaintAll {
    type Response = ();
}

#[derive(Clone, Copy)]
pub struct RepaintUI;

impl Message for RepaintUI {
    type Response = ();
}

#[atomic_enum]
#[derive(Default, PartialEq, Eq)]
pub enum RepaintStatus {
    All,
    UIOnly,
    #[default]
    None,
}

#[derive(Clone, Copy)]
pub struct CheckRepaint;

impl Message for CheckRepaint {
    type Response = RepaintStatus;
}

#[derive(Clone, Copy)]
pub struct ResetRepaint;

impl Message for ResetRepaint {
    type Response = ();
}

/// Listens to repaint events from the rest of the application.
/// This is reset once every frame, after which it will listen again.
pub struct RepaintListener {
    pub repaint_requested: AtomicRepaintStatus,
}

impl Default for RepaintListener {
    fn default() -> Self {
        Self {
            repaint_requested: AtomicRepaintStatus::new(RepaintStatus::None)
        }
    }
}

impl Actor<Event> for RepaintListener {}

#[async_trait]
impl Handler<Event, RepaintAll> for RepaintListener {
    async fn handle(&mut self, _: RepaintAll, _: &mut ActorContext<Event>) -> () {
        self.repaint_requested.store(RepaintStatus::All, Ordering::Relaxed);
    }
}

#[async_trait]
impl Handler<Event, RepaintUI> for RepaintListener {
    async fn handle(&mut self, _: RepaintUI, _: &mut ActorContext<Event>) -> () {
        self.repaint_requested.store(RepaintStatus::UIOnly, Ordering::Relaxed);
    }
}

#[async_trait]
impl Handler<Event, CheckRepaint> for RepaintListener {
    async fn handle(&mut self, _: CheckRepaint, _: &mut ActorContext<Event>) -> RepaintStatus {
        self.repaint_requested.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl Handler<Event, ResetRepaint> for RepaintListener {
    async fn handle(&mut self, _: ResetRepaint, _: &mut ActorContext<Event>) -> () {
        self.repaint_requested.store(RepaintStatus::None, Ordering::Relaxed);
    }
}