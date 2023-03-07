mod file_watcher;

pub mod dynamic_pipeline_builder;
pub use dynamic_pipeline_builder::*;

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use phobos as ph;
use phobos::vk;

use std::fmt::Debug;
use std::{env, fs};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use std::sync::{Arc, Mutex};

use tiny_tokio_actor::*;
use tokio::task::JoinHandle;

use anyhow::{anyhow, Result};

use notify::EventKind;

use crate::repaint;
use crate::event::Event;
use crate::safe_error::SafeUnwrap;

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

#[derive(Clone, Message)]
struct SetJoinHandle(Arc<JoinHandle<Result<()>>>);

#[derive(Debug, Clone, Message)]
struct FileEventMessage(notify::Event);

/// Send this message to the shader reload actor to register a shader to a graphics pipeline.
#[derive(Debug, Clone, Message)]
pub struct AddShader {
    pub path: PathBuf,
    pub stage: vk::ShaderStageFlags,
    pub pipeline: String,
}

#[async_trait]
impl<E> Handler<E, SetJoinHandle> for ShaderReloadActor where E: SystemEvent {
    async fn handle(&mut self, msg: SetJoinHandle, _ctx: &mut ActorContext<E>) -> () {
        self.watch_task = Some(msg.0);
    }
}

#[async_trait]
impl<E> Handler<E, FileEventMessage> for ShaderReloadActor where E: SystemEvent {
    async fn handle(&mut self, msg: FileEventMessage, _ctx: &mut ActorContext<E>) -> () {
        match msg.0 {
            notify::Event { kind, paths, .. } => {
                match kind {
                    EventKind::Modify(_) => {
                        for path in paths {
                            if path.extension().unwrap_or(OsStr::new("")) == OsString::from("hlsl") {
                                self.reload_file(path).await.safe_unwrap();
                            }
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
    async fn handle(&mut self, msg: AddShader, _ctx: &mut ActorContext<E>) -> () {
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
    pub async fn new<E>(pipelines: Arc<Mutex<ph::PipelineCache>>, system: &ActorSystem<E>, name: &str, path: impl Into<PathBuf>, recursive: bool) -> Result<ActorRef<E, Self>>
        where
            E: SystemEvent {

        let actor = system.create_actor(name, ShaderReloadActor {
            pipelines,
            shaders: HashMap::new(),
            watch_task: None,
        }).await?;

        let copy = actor.clone();
        let task = tokio::spawn(file_watcher::async_watch(path.into(), recursive, move |event| {
            actor.tell(FileEventMessage(event)).unwrap();
        }));

        copy.tell(SetJoinHandle{0: Arc::new(task)})?;

        Ok(copy)
    }

    fn get_dxc_path() -> Result<PathBuf> {
        Ok(env::var("VULKAN_SDK").map(|sdk| {
            PathBuf::from(&sdk).join("Bin/dxc")
        })?)
    }

    fn get_output_path(path: &Path) -> Result<PathBuf> {
        let prefix = path.parent().unwrap();
        fs::create_dir_all(prefix)?;
        Ok(prefix.join("out/").join(path.file_name().unwrap().to_str().unwrap().to_owned() + ".spv"))
    }

    fn hlsl_profile(stage: vk::ShaderStageFlags) -> Result<String> {
        Ok(match stage {
            vk::ShaderStageFlags::VERTEX => "vs",
            vk::ShaderStageFlags::FRAGMENT => "ps",
            _ => todo!()
        }.to_owned() + "_6_7")
    }

    async fn load_spirv_file(path: &Path) -> Result<Vec<u32>> {
        let mut f = File::open(&path).await?;
        let metadata = fs::metadata(&path)?;
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer).await?;
        let (_, binary, _) = unsafe { buffer.align_to::<u32>() };
        Ok(Vec::from(binary))
    }

    async fn compile_hlsl(path: &Path, stage: vk::ShaderStageFlags) -> Result<Vec<u32>> {
        let out = Self::get_output_path(path)?;
        let dxc = Self::get_dxc_path()?;
        let output = Command::new(dxc)
            // Entry point: 'main'
            .arg("-E main")
            // Output file
            .arg("-Fo".to_owned() + out.to_str().unwrap())
            // HLSL version 2021
            .arg("-HV 2021")
            // HLSL profile depending on shader stage
            .arg("-T ".to_owned() + &Self::hlsl_profile(stage)?)
            // Emit SPIR-V reflection info.
            // Note that we disable this for now, because this causes DXC to emit the SPV_GOOGLE_hlsl_functionality1 extension,
            // which we then have to enable in Vulkan. This is possible, but not really desired and ash does not support it, so preferably
            // reflection just works without this flag too.
            // .arg("-fspv-reflect")
            // SPIR-V target env
            .arg("-fspv-target-env=vulkan1.3")
            // Actually generate SPIR-V
            .arg("-spirv")
            // Our input file
            .arg(path)
            .output()
            .await?;
        match output.status.success() {
            true => {
                Self::load_spirv_file(&out).await
            }
            false => {
                Err(anyhow!("Error compiling shader {:?}: {}", path, String::from_utf8(output.stderr).unwrap()))
            }
        }
    }

    async fn reload_pipeline(&mut self, shader: &Path, pipeline: &str, stage: vk::ShaderStageFlags) -> Result<()> {
        info!("Reloading pipeline {:?}", pipeline);
        // let mut file = File::open(shader).await?;
        // let mut source = String::new();
        // file.read_to_string(&mut source).await?;
        // let kind = match stage {
        //     vk::ShaderStageFlags::VERTEX => shaderc::ShaderKind::Vertex,
        //     vk::ShaderStageFlags::FRAGMENT => shaderc::ShaderKind::Fragment,
        //     _ => todo!()
        // };
        //
        // let mut compiler = shaderc::Compiler::new().unwrap();
        // let mut options = shaderc::CompileOptions::new().unwrap();
        // let result = compiler.compile_into_spirv(&source, kind, shader.file_name().unwrap().to_str().unwrap(), "main", Some(&options))?;
        let binary = Self::compile_hlsl(shader, stage).await?;
        {
            let mut pipelines = self.pipelines.lock().unwrap();
            let mut pci = pipelines.pipeline_info(pipeline).ok_or(ph::Error::PipelineNotFound(pipeline.to_owned()))?.clone();
            // Update the used shader. We do this by first removing the shader with the reloaded stage, then pushing the new shader
            pci.shaders.retain(|shader| shader.stage != stage);
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
        Ok(())
    }
}