use tiny_tokio_actor::{ActorRef, ActorSystem, EventBus};
use anyhow::Result;
use futures::executor::block_on;

use crate::app::{repaint, RepaintAll, RepaintListener};
use crate::core::Event;
use crate::{gfx, gui};
use crate::gui::TargetResizeActor;
use crate::hot_reload::ShaderReloadActor;


/// Stores the actor system and actor refs to each 'root' actor.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct RootActorSystem {
    #[derivative(Debug="ignore")]
    pub system: ActorSystem<Event>,
    pub scene_texture: ActorRef<Event, TargetResizeActor>,
    pub repaint: ActorRef<Event, RepaintListener>,
    pub shader_reload: ActorRef<Event, ShaderReloadActor>,
}

impl RootActorSystem {
    pub async fn new(gfx: &gfx::SharedContext) -> Result<Self> {
        let bus = EventBus::new(100);
        let system = ActorSystem::new("Main task system", bus);
        let repaint = system.create_actor("repaint_listener", RepaintListener::default()).await?;
        let shader_reload = ShaderReloadActor::new(
            gfx.pipelines.clone(),
            repaint.clone(),
            &system,
            "shader_hot_reload",
            "shaders/src/",
            true
        ).await?;

        // Register the output image with the UI integration
        let scene_texture = system.create_actor("target_resize", TargetResizeActor::default()).await?;
        // Initially paint the scene
        repaint.ask(RepaintAll).await?;

        Ok(Self {
            system,
            scene_texture,
            repaint,
            shader_reload,
        })
    }

    pub async fn update_repaint_status(&mut self) -> Result<repaint::RepaintStatus> {
        let status = self.repaint.ask(repaint::CheckRepaint).await?;
        // Only send a reset message if the repaint status was to repaint
        if status != repaint::RepaintStatus::None {
            self.repaint.tell(repaint::ResetRepaint)?;
        }
        Ok(status)
    }

    pub async fn update_rt_size(&mut self, ui: &mut gui::UIIntegration, renderer: &mut gfx::WorldRenderer) -> Result<()> {
        // Query current render target size from system
        let size = self.scene_texture.ask(gui::QuerySceneTextureSize).await?;
        // If there was a resize request
        if let Some(size) = size {
            // Grab old image and unregister it
            let old = self.scene_texture.ask(gui::QueryCurrentSceneTexture).await?;
            if let Some(old) = old {
                ui.unregister_texture(old);
            }
            // Request a repaint
            self.repaint.tell(repaint::RepaintAll)?;
            // Grab a new image
            let image = renderer.resize_target(size, ui)?;
            // Send it to the resize handler
            self.scene_texture.ask(gui::SetNewTexture{0: image}).await?;
        }
        Ok::<(), anyhow::Error>(())
    }
}

impl Drop for RootActorSystem {
    fn drop(&mut self) {
        block_on( async {
            self.system.stop_actor(self.shader_reload.path()).await;
            self.system.stop_actor(self.repaint.path()).await;
            self.system.stop_actor(self.scene_texture.path()).await;
        });
    }
}