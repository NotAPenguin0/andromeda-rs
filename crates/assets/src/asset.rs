use anyhow::Result;
use inject::DI;
use scheduler::EventBus;

pub trait Asset {
    type LoadInfo: Send + 'static;

    fn load(info: Self::LoadInfo, bus: EventBus<DI>) -> Result<Self>
    where
        Self: Sized;
}
