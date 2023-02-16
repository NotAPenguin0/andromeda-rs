use std::ffi::c_void;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;

use anyhow::Result;

use egui_winit_ash_integration::{AllocationCreateInfoTrait, AllocationTrait, AllocatorTrait, Integration, MemoryLocation};

use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use phobos as ph;
use phobos::{GraphicsCmdBuffer, WindowSize};
use phobos::vk;
use phobos::vk::MemoryRequirements;

use crate::gfx;

pub struct Allocation(ph::vk_alloc::Allocation);
pub struct AllocationCreateInfo(ph::vk_alloc::AllocationCreateDesc<'static>);

impl AllocationTrait for Allocation {
    unsafe fn memory(&self) -> vk::DeviceMemory {
        self.0.memory()
    }

    fn offset(&self) -> u64 {
        self.0.offset()
    }

    fn size(&self) -> u64 {
        self.0.size()
    }

    fn mapped_ptr(&self) -> Option<NonNull<c_void>> {
        self.0.mapped_ptr()
    }
}

impl AllocationCreateInfoTrait for AllocationCreateInfo {
    fn new(requirements: MemoryRequirements, location: MemoryLocation, linear: bool) -> Self {
        Self {
            0: ph::vk_alloc::AllocationCreateDesc {
                name: "",
                requirements,
                location: match location {
                    MemoryLocation::Unknown => { ph::MemoryLocation::Unknown }
                    MemoryLocation::GpuOnly => { ph::MemoryLocation::GpuOnly }
                    MemoryLocation::CpuToGpu => { ph::MemoryLocation::CpuToGpu }
                    MemoryLocation::GpuToCpu => { ph::MemoryLocation::GpuToCpu }
                },
                linear,
                allocation_scheme: ph::vk_alloc::AllocationScheme::GpuAllocatorManaged,
            }
        }
    }
}

impl AllocatorTrait for gfx::ThreadSafeAllocator {
    type Allocation = Allocation;
    type AllocationCreateInfo = AllocationCreateInfo;

    fn allocate(&self, desc: Self::AllocationCreateInfo) -> Result<Self::Allocation> {
        let mut lock = self.lock().unwrap(); // TODO
        Ok(Allocation{0: lock.allocate(&desc.0)?})
    }

    fn free(&self, allocation: Self::Allocation) -> Result<()> {
        let mut lock = self.lock().unwrap();
        Ok(lock.free(allocation.0)?)
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct UIIntegration {
    #[derivative(Debug="ignore")]
    integration: ManuallyDrop<Integration<gfx::ThreadSafeAllocator>>,
    sampler: ph::Sampler,
}

impl UIIntegration {
    pub fn new(event_loop: &EventLoop<()>,
               window: &Window,
               ctx: gfx::SharedContext,
               gfx_queue: &ph::Queue,
               swapchain: &ph::Swapchain) -> Result<Self> {
        let mut style = egui::Style::default();
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;


        Ok(Self {
            sampler: ph::Sampler::default(ctx.device.clone())?,
            integration: ManuallyDrop::new(Integration::new(
                event_loop,
                window.width(),
                window.height(),
                window.scale_factor(),
                egui::FontDefinitions::default(),
                style,
                unsafe { ctx.device.ash_device() },
                ctx.allocator.clone(),
                gfx_queue.info.family_index,
                gfx_queue.handle(),
                unsafe { swapchain.loader() },
                unsafe { swapchain.handle() },
                swapchain.format
            )),
        })
    }

    pub fn new_frame(&mut self, window: &Window) {
        self.integration.begin_frame(window);
    }

    pub fn render<'w: 's, 's: 'e + 'q, 'e, 'q>(
        &'s mut self,
        window: &'w Window,
        swapchain: ph::VirtualResource,
        graph: ph::PassGraph<'e, 'q, ph::domain::All>)
        -> Result<ph::PassGraph<'e, 'q, ph::domain::All>> {

        graph.add_pass(ph::PassBuilder::render("ui")
            .color_attachment(swapchain.clone(), vk::AttachmentLoadOp::CLEAR, Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0]}))?
            .sample_image(ph::VirtualResource::image("output"), ph::PipelineStage::FRAGMENT_SHADER)
            .execute(|cmd, ifc, _| {
                let output = self.integration.end_frame(window);
                let clipped_meshes = self.integration.context().tessellate(output.shapes);
                self.integration.paint(unsafe { cmd.handle() }, ifc.swapchain_image_index.unwrap(), clipped_meshes, output.textures_delta);
                Ok(cmd)
            })
            .build()
        )
    }

    pub fn context(&self) -> egui::Context {
        self.integration.context().clone()
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        let _ = self.integration.handle_event(event);
    }

    pub fn register_texture(&mut self, image: &ph::ImageView) -> egui::TextureId {
        self.integration.register_user_texture(image.handle, self.sampler.handle)
    }
}

impl Drop for UIIntegration {
    fn drop(&mut self) {
        unsafe {
            self.integration.destroy();
            ManuallyDrop::drop(&mut self.integration);
        }
    }
}