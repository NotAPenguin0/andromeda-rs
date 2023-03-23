use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use futures::executor::block_on;
use phobos::{vk, PipelineBuilder, PipelineCache};
use tiny_tokio_actor::ActorRef;

use crate::event::Event;
use crate::hot_reload;

pub trait IntoDynamic {
    type Target;

    fn into_dynamic(self) -> Self::Target;
}

pub struct DynamicPipelineBuilder {
    inner: PipelineBuilder,
    shaders: Vec<hot_reload::AddShader>,
}

impl DynamicPipelineBuilder {
    pub fn attach_shader(mut self, path: impl Into<PathBuf>, stage: vk::ShaderStageFlags) -> Self {
        self.shaders.push(hot_reload::AddShader {
            path: path.into(),
            stage,
            pipeline: self.inner.get_name().into(),
        });
        self
    }

    /// Builds the pipeline using hot-reloadable shaders. You do not need to call add_named_pipeline() anymore after this
    pub fn build(self, hot_reload: ActorRef<Event, hot_reload::ShaderReloadActor>, cache: Arc<Mutex<PipelineCache>>) -> Result<()> {
        let pci = self.inner.build();
        {
            let mut cache = cache.lock().unwrap();
            cache.create_named_pipeline(pci)?;
        }

        let _ = self
            .shaders
            .into_iter()
            .map(|shader| {
                block_on(hot_reload.ask(shader)).unwrap();
            })
            .collect::<Vec<_>>();

        Ok(())
    }
}

impl IntoDynamic for PipelineBuilder {
    type Target = DynamicPipelineBuilder;

    fn into_dynamic(self) -> Self::Target {
        DynamicPipelineBuilder {
            inner: self,
            shaders: vec![],
        }
    }
}
