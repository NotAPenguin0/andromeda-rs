use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

use anyhow::Result;

use egui_winit_ash_integration::{AllocationCreateInfoTrait, AllocationTrait, AllocatorTrait, Integration, MemoryLocation};

use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use phobos as ph;
use phobos::WindowSize;
use phobos::vk;
use phobos::vk::MemoryRequirements;

use crate::gfx;
use crate::gui::{Image, USize};

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
    // Deletion queue, but needs access to self so we cant put it in an actual deletion queue.
    to_unregister: Vec<(Image, u32)>,
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
            to_unregister: vec![],
        })
    }

    pub fn new_frame(&mut self, window: &Window) {
        self.integration.begin_frame(window);
        self.to_unregister.iter_mut().for_each(|(image, ttl)| {
            *ttl = *ttl - 1;
            if *ttl == 0 {
                self.integration.unregister_user_texture(image.id);
            }
        });
        self.to_unregister.retain(|(_, ttl)| *ttl != 0);
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

    pub fn register_texture(&mut self, image: &ph::ImageView) -> Image {
        let id = self.integration.register_user_texture(image.handle, self.sampler.handle);
        Image {
            id,
            size: USize::new(image.size.width, image.size.height),
        }
    }

    pub fn unregister_texture(&mut self, image: Image) {
        self.to_unregister.push((image, 4));
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