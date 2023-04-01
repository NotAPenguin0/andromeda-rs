use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use phobos::{vk, ComputePipelineBuilder, PipelineBuilder, PipelineCache};

use crate::hot_reload::ShaderReload;

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

#[derive(Debug)]
pub struct DynamicComputePipelineBuilder {
    inner: ComputePipelineBuilder,
    shader: Option<ShaderInfo>,
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
    pub fn build(
        self,
        mut hot_reload: ShaderReload,
        cache: Arc<Mutex<PipelineCache>>,
    ) -> Result<()> {
        let pci = self.inner.build();
        {
            let mut cache = cache.lock().unwrap();
            cache.create_named_pipeline(pci)?;
        }

        let _ = self
            .shaders
            .into_iter()
            .map(|shader| {
                hot_reload.add_shader(shader.path, shader.stage, shader.pipeline);
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

impl DynamicComputePipelineBuilder {
    pub fn set_shader(mut self, path: impl Into<PathBuf>) -> Self {
        self.shader = Some(ShaderInfo {
            path: path.into(),
            stage: vk::ShaderStageFlags::COMPUTE,
            pipeline: self.inner.get_name().into(),
        });
        self
    }

    pub fn build(
        self,
        mut hot_reload: ShaderReload,
        cache: Arc<Mutex<PipelineCache>>,
    ) -> Result<()> {
        let pci = self.inner.build();
        {
            let mut cache = cache.lock().unwrap();
            cache.create_named_compute_pipeline(pci)?;
        }

        let shader = self.shader.expect("Must set a shader");
        hot_reload.add_shader(shader.path, shader.stage, shader.pipeline);
        Ok(())
    }
}

impl IntoDynamic for ComputePipelineBuilder {
    type Target = DynamicComputePipelineBuilder;

    fn into_dynamic(self) -> Self::Target {
        DynamicComputePipelineBuilder {
            inner: self,
            shader: None,
        }
    }
}
