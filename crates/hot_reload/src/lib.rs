use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::{env, fs};

use anyhow::{ensure, Result};
pub use dynamic_pipeline_builder::*;
use inject::DI;
use log::info;
use notify::EventKind;
use phobos::{prelude as ph, vk, PipelineCache, PipelineType};
use scheduler::{Event, EventBus, EventContext, StoredSystem, System};
use tokio::task::JoinHandle;
use util::safe_error::SafeUnwrap;
use util::RwLock;

pub mod dynamic_pipeline_builder;
mod file_watcher;

pub struct AddShaderEvent {
    path: PathBuf,
    stage: vk::ShaderStageFlags,
    pipeline: String,
}

impl Event for AddShaderEvent {}

#[derive(Debug, Clone)]
struct ShaderInfo {
    stage: vk::ShaderStageFlags,
    pipelines: Vec<String>,
}

#[derive(Debug)]
pub struct ShaderReloadInner {
    pipelines: PipelineCache,
    shaders: HashMap<PathBuf, ShaderInfo>,
    watch_tasks: Vec<JoinHandle<Result<()>>>,
}

#[derive(Debug, Clone)]
pub struct ShaderReload {
    inner: Arc<RwLock<ShaderReloadInner>>,
}

impl ShaderReload {
    pub fn new(
        pipelines: PipelineCache,
        path: impl Into<PathBuf>,
        recursive: bool,
    ) -> Result<Self> {
        let this = ShaderReload {
            inner: Arc::new(RwLock::new(ShaderReloadInner {
                pipelines,
                shaders: HashMap::default(),
                watch_tasks: vec![],
            })),
        };

        let copy = this.clone();
        let watcher =
            tokio::spawn(file_watcher::async_watch(path.into(), recursive, move |event| {
                copy.handle_file_event(event);
            }));

        this.inner.write().unwrap().watch_tasks.push(watcher);

        Ok(this)
    }

    pub fn add_shader(&mut self, path: &PathBuf, stage: vk::ShaderStageFlags, pipeline: &String) {
        let mut inner = self.inner.write().unwrap();
        info!("Pipeline {pipeline:?} added to watch for shader {path:?}");
        let entry = inner.shaders.entry(fs::canonicalize(path.clone()).unwrap());
        match entry {
            Entry::Occupied(entry) => {
                entry.into_mut().pipelines.push(pipeline.clone());
            }
            Entry::Vacant(entry) => {
                entry.insert(ShaderInfo {
                    stage,
                    pipelines: vec![pipeline.clone()],
                });
            }
        };
        self.reload_pipeline(path.as_path(), pipeline, &mut inner.pipelines, stage)
            .safe_unwrap();
    }

    pub fn handle_file_event(&self, event: notify::Event) {
        let notify::Event {
            kind,
            paths,
            ..
        } = event;
        if let EventKind::Modify(_) = kind {
            for path in paths {
                if path.extension().unwrap_or(OsStr::new("")) == "hlsl" {
                    self.reload_file(path).safe_unwrap();
                }
            }
        }
    }

    fn get_dxc_path() -> Result<PathBuf> {
        if cfg!(target_os = "linux") {
            Ok(PathBuf::from("/usr/bin/dxc"))
        } else {
            Ok(env::var("VULKAN_SDK").map(|sdk| PathBuf::from(&sdk).join("Bin/dxc"))?)
        }
    }

    fn get_output_path(path: &Path) -> Result<PathBuf> {
        let prefix = path.parent().unwrap();
        fs::create_dir_all(prefix)?;
        Ok(prefix
            .join("out/")
            .join(path.file_name().unwrap().to_str().unwrap().to_owned() + ".spv"))
    }

    fn hlsl_profile(stage: vk::ShaderStageFlags) -> Result<String> {
        Ok(match stage {
            vk::ShaderStageFlags::VERTEX => "vs",
            vk::ShaderStageFlags::FRAGMENT => "ps",
            vk::ShaderStageFlags::COMPUTE => "cs",
            // Tessellation control in HLSL is a Hull Shader
            vk::ShaderStageFlags::TESSELLATION_CONTROL => "hs",
            // Tessellation evaluation in HLSL is a Domain Shader
            vk::ShaderStageFlags::TESSELLATION_EVALUATION => "ds",
            _ => todo!(),
        }
        .to_owned()
            + "_6_7")
    }

    fn load_spirv_file(path: &Path) -> Result<Vec<u32>> {
        let mut f = File::open(path)?;
        let metadata = fs::metadata(path)?;
        let mut buffer = vec![0; metadata.len() as usize];
        f.read_exact(&mut buffer)?;
        // SAFETY: A valid SPIR-V module is made out of 32-bit words.
        let (_, binary, _) = unsafe { buffer.align_to::<u32>() };
        Ok(Vec::from(binary))
    }

    #[allow(clippy::suspicious_command_arg_space)]
    fn compile_hlsl(path: &Path, stage: vk::ShaderStageFlags) -> Result<Vec<u32>> {
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
            // Add include path
            .arg("-I shaders/include")
            // Actually generate SPIR-V
            .arg("-spirv")
            // Our input file
            .arg(path)
            .output()?;

        ensure!(
            output.status.success(),
            "Error compiling shader {path:?}: {}",
            String::from_utf8(output.stderr).unwrap()
        );
        Self::load_spirv_file(&out)
    }

    fn reload_pipeline(
        &self,
        shader: &Path,
        pipeline: &str,
        pipelines: &mut ph::PipelineCache,
        stage: vk::ShaderStageFlags,
    ) -> Result<()> {
        info!("Reloading pipeline {pipeline:?}");
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
        let binary = Self::compile_hlsl(shader, stage)?;
        match pipelines.pipeline_type(pipeline) {
            None => {}
            Some(PipelineType::Graphics) => {
                let mut pci = pipelines.pipeline_info(pipeline).unwrap();
                // Update the used shader. We do this by first removing the shader with the reloaded stage, then pushing the new shader
                pci.shaders.retain(|shader| shader.stage() != stage);
                pci.shaders
                    .push(ph::ShaderCreateInfo::from_spirv(stage, binary));
                // This fixes a validation layer message, but I have no idea why
                pci.build_inner();
                // Register as new pipeline, this will update the PCI
                pipelines.create_named_pipeline(pci)?;
            }
            Some(PipelineType::Compute) => {
                let mut pci = pipelines.compute_pipeline_info(pipeline).unwrap();
                // Replace shader, compute shaders only have one shader so this is easy
                pci.shader =
                    Some(ph::ShaderCreateInfo::from_spirv(vk::ShaderStageFlags::COMPUTE, binary));
                // Register as new pipeline, this will update the PCI
                pipelines.create_named_compute_pipeline(pci)?;
            }
            Some(PipelineType::RayTracing) => {
                todo!()
            }
        }

        Ok(())
    }

    fn reload_file(&self, path: PathBuf) -> Result<()> {
        // If our shader was an included shader, we naively reload all pipelines
        let inner = self.inner.write().unwrap();
        let mut pipelines = inner.pipelines.clone();
        if path.to_str().unwrap().contains("shaders\\include\\") {
            info!(
                "Included shader {:?} changed. Reloading all pipelines.",
                path.file_name().unwrap()
            );
            for (path, info) in &inner.shaders {
                for pipeline in &info.pipelines {
                    self.reload_pipeline(path, pipeline, &mut pipelines, info.stage)?;
                }
            }
            return Ok(());
        }

        // CLion always saves quickly files with a ~ suffix first for some reason, so we add a quick hack to ignore this temporary file
        if path.file_name().unwrap().to_str().unwrap().ends_with('~') {
            return Ok(());
        }
        info!("Reloading shader file {:?}", path.file_name().unwrap());
        // Get all involved pipelines
        let info = inner
            .shaders
            .get(path.as_path())
            .ok_or_else(|| {
                anyhow::anyhow!("Shader path not in watchlist: {:?}", path.file_name().unwrap())
            })
            .cloned()?;
        for pipeline in &info.pipelines {
            self.reload_pipeline(&path, pipeline, &mut pipelines, info.stage)?;
        }
        Ok(())
    }
}

impl System<DI> for ShaderReload {
    fn initialize(event_bus: &EventBus<DI>, system: &StoredSystem<Self>)
    where
        Self: Sized, {
        event_bus.subscribe(system, handle_add_shader);
    }
}

fn handle_add_shader(
    state: &mut ShaderReload,
    event: &AddShaderEvent,
    _ctx: &mut EventContext<DI>,
) -> Result<()> {
    state.add_shader(&event.path, event.stage, &event.pipeline);
    Ok(())
}

pub fn initialize(
    pipelines: PipelineCache,
    path: impl Into<PathBuf>,
    recursive: bool,
    bus: &mut EventBus<DI>,
) -> Result<()> {
    let state = ShaderReload::new(pipelines, path, recursive)?;
    bus.add_system(state.clone());
    let mut di = bus.data().write().unwrap();
    di.put(state);
    Ok(())
}
