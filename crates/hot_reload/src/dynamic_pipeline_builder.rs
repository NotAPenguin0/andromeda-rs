use std::path::PathBuf;

use anyhow::Result;
use inject::DI;
use phobos::{vk, ComputePipelineBuilder, PipelineBuilder, PipelineCache};
use scheduler::EventBus;

use crate::AddShaderEvent;

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
            pipeline: self.inner.name().into(),
        });
        self
    }

    /// Builds the pipeline using hot-reloadable shaders. You do not need to call add_named_pipeline() anymore after this
    pub fn build(self, bus: &mut EventBus<DI>, mut cache: PipelineCache) -> Result<()> {
        // TODO: Add pipeline cache to DI?
        let pci = self.inner.build();
        cache.create_named_pipeline(pci)?;

        let _ = self
            .shaders
            .into_iter()
            .map(|shader| {
                bus.publish(&AddShaderEvent {
                    path: shader.path,
                    stage: shader.stage,
                    pipeline: shader.pipeline,
                })
                .unwrap();
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
            pipeline: self.inner.name().into(),
        });
        self
    }

    pub fn build(self, bus: &mut EventBus<DI>, mut cache: PipelineCache) -> Result<()> {
        let pci = self.inner.build();
        cache.create_named_compute_pipeline(pci)?;

        let shader = self.shader.expect("Must set a shader");
        bus.publish(&AddShaderEvent {
            path: shader.path,
            stage: shader.stage,
            pipeline: shader.pipeline,
        })?;
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
