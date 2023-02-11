use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use anyhow::Result;

use phobos as ph;
use phobos::{vk, WindowSize};

use egui;
use egui_winit_ash_integration;
use egui_winit_ash_integration::{Integration, MemoryLocation};
use phobos::vk::{DeviceMemory, MemoryRequirements};
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::gfx;

pub struct Allocation(ph::vk_alloc::Allocation);
pub struct AllocationCreateInfo(ph::vk_alloc::AllocationCreateDesc<'static>);

impl egui_winit_ash_integration::AllocationTrait for Allocation {
    unsafe fn memory(&self) -> DeviceMemory {
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

impl egui_winit_ash_integration::AllocationCreateInfoTrait for AllocationCreateInfo {
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

impl egui_winit_ash_integration::AllocatorTrait for gfx::ThreadSafeAllocator {
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
}

impl UIIntegration {
    pub fn new(event_loop: &EventLoop<()>,
               window: &Window,
               device: Arc<ph::Device>,
               allocator: gfx::ThreadSafeAllocator,
               gfx_queue: &ph::Queue,
               swapchain: &ph::Swapchain) -> Result<Self> {
        Ok(Self {
            integration: ManuallyDrop::new(Integration::new(
                event_loop,
                window.width(),
                window.height(),
                window.scale_factor(),
                egui::FontDefinitions::default(),
                egui::Style::default(),
                unsafe { device.ash_device() },
                allocator.clone(),
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

    pub fn render(&mut self, window: &Window, cmd: vk::CommandBuffer, image_index: usize) {
        let output = self.integration.end_frame(window);
        let clipped_meshes = self.integration.context().tessellate(output.shapes);
        self.integration.paint(cmd, image_index, clipped_meshes, output.textures_delta);
    }

    pub fn context(&self) -> egui::Context {
        self.integration.context().clone()
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