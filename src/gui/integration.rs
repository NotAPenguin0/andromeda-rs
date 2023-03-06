use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::ptr::NonNull;

use anyhow::Result;

use egui_winit_phobos::Integration;

use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use phobos as ph;
use phobos::WindowSize;
use phobos::vk;
use phobos::vk::MemoryRequirements;

use crate::gfx;
use crate::gui::{Image, USize};


#[derive(Derivative)]
#[derivative(Debug)]
pub struct UIIntegration {
    #[derivative(Debug="ignore")]
    integration: ManuallyDrop<Integration>,
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
            integration: ManuallyDrop::new(Integration::new(
                window.width(),
                window.height(),
                window.scale_factor() as f32, event_loop,
                egui::FontDefinitions::default(), style,
                ctx.device.clone(),
                ctx.allocator.deref().clone(),
                ctx.exec.clone(),
                ctx.pipelines.clone(),
                ctx.descriptors.clone()
            )?),
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

    pub async fn render<'s: 'e + 'q, 'e, 'q>(
        &'s mut self,
        window: &Window,
        swapchain: ph::VirtualResource,
        graph: ph::PassGraph<'e, 'q, ph::domain::All>)
        -> Result<ph::PassGraph<'e, 'q, ph::domain::All>> {

        let output = self.integration.end_frame(window);
        let clipped_meshes = self.integration.context().tessellate(output.shapes);
        let scene_output = ph::VirtualResource::image("output").upgrade(); // NOTE THE UPGRADE CALL
        self.integration.paint(
            graph,
            std::slice::from_ref(&scene_output),
            swapchain,
            vk::AttachmentLoadOp::CLEAR,
            Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0]}),
            clipped_meshes,
            output.textures_delta
        ).await
    }

    pub fn context(&self) -> egui::Context {
        self.integration.context().clone()
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        let _ = self.integration.handle_event(event);
    }

    pub fn register_texture(&mut self, image: &ph::ImageView) -> Image {
        let id = self.integration.register_user_texture(image);
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
            ManuallyDrop::drop(&mut self.integration);
        }
    }
}