use anyhow::Result;
use tiny_tokio_actor::{ActorRef, ActorSystem, EventBus};
use tokio::runtime::Handle;

use crate::core::Event;
use crate::gui::util::integration::UIIntegration;
use crate::hot_reload::ShaderReloadActor;
use crate::{core, gfx, hot_reload, state};

/// Stores the actor system and actor refs to each 'root' actor.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct RootActorSystem {
    pub shader_reload: ActorRef<Event, ShaderReloadActor>,
    #[derivative(Debug = "ignore")]
    pub system: ActorSystem<Event>,
}

impl RootActorSystem {
    pub async fn new(gfx: &gfx::SharedContext) -> Result<Self> {
        let bus = EventBus::new(100);
        let system = ActorSystem::new("Main task system", bus);
        let shader_reload = ShaderReloadActor::new(gfx.pipelines.clone(), &system, "shader_hot_reload", "shaders/", true).await?;

        Ok(Self {
            system,
            shader_reload,
        })
    }
}

impl Drop for RootActorSystem {
    fn drop(&mut self) {
        Handle::current().block_on(async {
            self.shader_reload.ask(hot_reload::Kill).await.unwrap();
            self.system.stop_actor(self.shader_reload.path()).await;
        });
    }
}
