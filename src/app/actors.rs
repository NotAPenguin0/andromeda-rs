use anyhow::Result;
use tiny_tokio_actor::{ActorRef, ActorSystem, EventBus};
use tokio::runtime::Handle;

use crate::core::{AddInputListener, Event};
use crate::gui::editor::camera_controller::{CameraController, CameraScrollListener};
use crate::gui::editor::world_view::{QueryCurrentSceneTexture, QuerySceneTextureSize, SetNewTexture, TargetResizeActor};
use crate::gui::util::integration::UIIntegration;
use crate::hot_reload::ShaderReloadActor;
use crate::{core, gfx, hot_reload, state};

/// Stores the actor system and actor refs to each 'root' actor.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct RootActorSystem {
    pub scene_texture: ActorRef<Event, TargetResizeActor>,
    pub shader_reload: ActorRef<Event, ShaderReloadActor>,
    pub camera: ActorRef<Event, state::Camera>,
    pub input: ActorRef<Event, core::Input>,
    pub camera_controller: ActorRef<Event, CameraController>,
    #[derivative(Debug = "ignore")]
    pub system: ActorSystem<Event>,
}

impl RootActorSystem {
    pub async fn new(gfx: &gfx::SharedContext) -> Result<Self> {
        let bus = EventBus::new(100);
        let system = ActorSystem::new("Main task system", bus);
        let shader_reload = ShaderReloadActor::new(gfx.pipelines.clone(), &system, "shader_hot_reload", "shaders/", true).await?;

        // Register the output image with the UI integration
        let scene_texture = system.create_actor("target_resize", TargetResizeActor::default()).await?;

        let camera = system.create_actor("camera_state", state::Camera::default()).await?;
        let input = system.create_actor("input", core::Input::default()).await?;

        let camera_controller = system
            .create_actor("camera_controller", CameraController::new(input.clone(), camera.clone()))
            .await?;

        input.tell(AddInputListener(CameraScrollListener::new(camera_controller.clone())))?;

        Ok(Self {
            system,
            scene_texture,
            shader_reload,
            camera,
            input,
            camera_controller,
        })
    }

    pub async fn update_rt_size(&mut self, ui: &mut UIIntegration, renderer: &mut gfx::WorldRenderer) -> Result<()> {
        // Query current render target size from system
        let size = self.scene_texture.ask(QuerySceneTextureSize).await?;
        // If there was a resize request
        if let Some(size) = size {
            // Grab old image and unregister it
            let old = self.scene_texture.ask(QueryCurrentSceneTexture).await?;
            if let Some(old) = old {
                ui.unregister_texture(old);
            }
            // Grab a new image
            let image = renderer.resize_target(size, ui)?;
            // Send it to the resize handler
            self.scene_texture
                .ask(SetNewTexture {
                    0: image,
                })
                .await?;
        }
        Ok::<(), anyhow::Error>(())
    }
}

impl Drop for RootActorSystem {
    fn drop(&mut self) {
        Handle::current().block_on(async {
            self.shader_reload.ask(hot_reload::Kill).await.unwrap();
            self.system.stop_actor(self.shader_reload.path()).await;
            self.system.stop_actor(self.scene_texture.path()).await;
            self.system.stop_actor(self.camera.path()).await;
            self.system.stop_actor(self.input.path()).await;
            self.system.stop_actor(self.camera_controller.path()).await;
        });
    }
}
