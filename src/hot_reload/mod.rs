mod file_watcher;

pub mod dynamic_pipeline_builder;
pub use dynamic_pipeline_builder::*;

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use phobos as ph;
use phobos::vk;

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tiny_tokio_actor::*;
use tokio::task::JoinHandle;

use anyhow::Result;

use notify::EventKind;

use crate::repaint;
use crate::event::Event;
use crate::safe_error::SafeUnwrap;

use shaderc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone)]
struct ShaderInfo {
    stage: vk::ShaderStageFlags,
    pipelines: Vec<String>,
}

pub struct ShaderReloadActor {
    pipelines: Arc<Mutex<ph::PipelineCache>>,
    shaders: HashMap<PathBuf, ShaderInfo>,
    watch_task: Option<Arc<JoinHandle<Result<()>>>>,
    repaint: ActorRef<Event, repaint::RepaintListener>,
}

unsafe impl Send for ShaderReloadActor {}
unsafe impl Sync for ShaderReloadActor {}

#[async_trait]
impl<E> Actor<E> for ShaderReloadActor where E: SystemEvent {
    async fn post_stop(&mut self, _ctx: &mut ActorContext<E>) {
        // Kill the owned file watcher
        match &self.watch_task {
            None => {}
            Some(task) => { task.abort() }
        }
    }
}

#[derive(Clone)]
struct SetJoinHandle(Arc<JoinHandle<Result<()>>>);

#[derive(Debug, Clone)]
struct FileEventMessage(notify::Event);

/// Send this message to the shader reload actor to register a shader to a graphics pipeline.
#[derive(Debug, Clone)]
pub struct AddShader {
    pub path: PathBuf,
    pub stage: vk::ShaderStageFlags,
    pub pipeline: String,
}

impl Message for SetJoinHandle {
    type Response = ();
}

impl Message for FileEventMessage {
    type Response = ();
}

impl Message for AddShader {
    type Response = ();
}

#[async_trait]
impl<E> Handler<E, SetJoinHandle> for ShaderReloadActor where E: SystemEvent {
    async fn handle(&mut self, msg: SetJoinHandle, ctx: &mut ActorContext<E>) -> () {
        self.watch_task = Some(msg.0);
    }
}

#[async_trait]
impl<E> Handler<E, FileEventMessage> for ShaderReloadActor where E: SystemEvent {
    async fn handle(&mut self, msg: FileEventMessage, ctx: &mut ActorContext<E>) -> () {
        match msg.0 {
            notify::Event { kind, paths, .. } => {
                match kind {
                    EventKind::Modify(_) => {
                        for path in paths {
                            self.reload_file(path).await.safe_unwrap();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[async_trait]
impl<E> Handler<E, AddShader> for ShaderReloadActor where E: SystemEvent {
    async fn handle(&mut self, msg: AddShader, ctx: &mut ActorContext<E>) -> () {
        info!("Pipeline {:?} added to watch for shader {:?}", msg.pipeline, msg.path);
        let entry = self.shaders.entry(fs::canonicalize(msg.path.clone()).unwrap());
        match entry {
            Entry::Occupied(entry) => {
                entry.into_mut().pipelines.push(msg.pipeline.clone());
            }
            Entry::Vacant(entry) => {
                entry.insert(ShaderInfo { stage: msg.stage, pipelines: vec![msg.pipeline.clone()] });
            }
        };
        self.reload_pipeline(msg.path.as_path(), &msg.pipeline, msg.stage).await.safe_unwrap();
    }
}

impl ShaderReloadActor {
    pub async fn new<E>(pipelines: Arc<Mutex<ph::PipelineCache>>, repaint: ActorRef<Event, repaint::RepaintListener>, system: &ActorSystem<E>, name: &str, path: impl Into<PathBuf>, recursive: bool) -> Result<ActorRef<E, Self>>
        where
            E: SystemEvent {

        let actor = system.create_actor(name, ShaderReloadActor {
            pipelines,
            shaders: HashMap::new(),
            watch_task: None,
            repaint
        }).await?;

        let copy = actor.clone();
        let task = tokio::spawn(file_watcher::async_watch(path.into(), recursive, move |event| {
            actor.tell(FileEventMessage(event)).unwrap();
        }));

        copy.tell(SetJoinHandle{0: Arc::new(task)})?;

        Ok(copy)
    }

    async fn reload_pipeline(&mut self, shader: &Path, pipeline: &str, stage: vk::ShaderStageFlags) -> Result<()> {
        info!("Reloading pipeline {:?}", pipeline);
        let mut file = File::open(shader).await?;
        let mut glsl = String::new();
        file.read_to_string(&mut glsl).await?;
        let kind = match stage {
            vk::ShaderStageFlags::VERTEX => shaderc::ShaderKind::Vertex,
            vk::ShaderStageFlags::FRAGMENT => shaderc::ShaderKind::Fragment,
            _ => todo!()
        };
        let mut compiler = shaderc::Compiler::new().unwrap();
        let mut options = shaderc::CompileOptions::new().unwrap();
        let result = compiler.compile_into_spirv(&glsl, kind, shader.file_name().unwrap().to_str().unwrap(), "main", Some(&options))?;
        {
            let mut pipelines = self.pipelines.lock().unwrap();
            let mut pci = pipelines.pipeline_info(pipeline).ok_or(ph::Error::PipelineNotFound(pipeline.to_owned()))?.clone();
            // Update the used shader. We do this by first removing the shader with the reloaded stage, then pushing the new shader
            pci.shaders.retain(|shader| shader.stage != stage);
            let binary = result.as_binary().to_vec();
            pci.shaders.push(ph::ShaderCreateInfo::from_spirv(stage, binary));
            // This fixes a validation layer message, but I have no idea why
            pci.build_inner();
            // Register as new pipeline, this will update the PCI
            pipelines.create_named_pipeline(pci)?;
        }

        Ok(())
    }

    async fn reload_file(&mut self, path: PathBuf) -> Result<()> {
        // CLion always saves quickly files with an ~ suffix first for some reason, so we add a quick hack to ignore this temporary file
        if path.file_name().unwrap().to_str().unwrap().ends_with("~") { return Ok(()); }
        info!("Reloading shader file {:?}", path.file_name().unwrap());
        // Get all involved pipelines
        let info = self.shaders.get(path.as_path()).ok_or(anyhow::anyhow!("Shader path not in watchlist: {:?}", path.file_name().unwrap())).cloned()?;
        for pipeline in &info.pipelines {
            self.reload_pipeline(&path, pipeline, info.stage).await?;
        }
        // Pipelines reloaded, so scene has to be repainted
        self.repaint.tell(repaint::RepaintAll)?;
        Ok(())
    }
}