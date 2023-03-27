use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use phobos::{vk, PipelineBuilder, PipelineCache};

use crate::hot_reload::SyncShaderReload;

pub trait IntoDynamic {
    type Target;

    fn into_dynamic(self) -> Self::Target;
}

#[derive(Debug)]
struct ShaderInfo {
    path: PathBuf,
    stage: vk::ShaderStageFlags,
    pipeline: String,
}

#[derive(Debug)]
pub struct DynamicPipelineBuilder {
    inner: PipelineBuilder,
    shaders: Vec<ShaderInfo>,
}

impl DynamicPipelineBuilder {
    pub fn attach_shader(mut self, path: impl Into<PathBuf>, stage: vk::ShaderStageFlags) -> Self {
        self.shaders.push(ShaderInfo {
            path: path.into(),
            stage,
            pipeline: self.inner.get_name().into(),
        });
        self
    }

    /// Builds the pipeline using hot-reloadable shaders. You do not need to call add_named_pipeline() anymore after this
    pub fn build(self, hot_reload: SyncShaderReload, cache: Arc<Mutex<PipelineCache>>) -> Result<()> {
        let pci = self.inner.build();
        {
            let mut cache = cache.lock().unwrap();
            cache.create_named_pipeline(pci)?;
        }

        let mut reload = hot_reload.write().unwrap();
        let _ = self
            .shaders
            .into_iter()
            .map(|shader| {
                reload.add_shader(shader.path, shader.stage, shader.pipeline);
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
