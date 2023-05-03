use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use derivative::Derivative;
use egui::TextureId;
use egui_winit_phobos::Integration;
use gui::util::image::Image;
use gui::util::size::USize;
use phobos::prelude::traits::*;
use phobos::{vk, DefaultAllocator, ImageView, VirtualResource};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::util::graph::FrameGraph;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct UIIntegration {
    #[derivative(Debug = "ignore")]
    integration: Integration<DefaultAllocator>,
    // Deletion queue, but needs access to self so we cant put it in an actual deletion queue.
    to_unregister: HashMap<TextureId, u32>,
}

impl UIIntegration {
    pub fn new(
        event_loop: &EventLoop<()>,
        window: &Window,
        ctx: gfx::SharedContext,
    ) -> Result<Self> {
        let mut style = egui::Style::default();

        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;

        Ok(Self {
            integration: Integration::new(
                window.width(),
                window.height(),
                window.scale_factor() as f32,
                event_loop,
                egui::FontDefinitions::default(),
                style,
                ctx.device.clone(),
                ctx.allocator.clone(),
                ctx.exec.clone(),
                ctx.pipelines,
            )?,
            to_unregister: HashMap::new(),
        })
    }

    pub fn new_frame(&mut self, window: &Window) {
        self.integration.begin_frame(window);
        self.to_unregister.iter_mut().for_each(|(id, ttl)| {
            *ttl -= 1;
            if *ttl == 0 {
                self.integration.unregister_user_texture(*id);
            }
        });
        self.to_unregister.retain(|_, ttl| *ttl != 0);
    }

    pub fn render<'cb>(
        &'cb mut self,
        window: &Window,
        swapchain: VirtualResource,
        graph: &mut FrameGraph<'cb>,
    ) -> Result<()> {
        self.integration.resize(window.width(), window.height());

        let output = self.integration.end_frame(window);
        let clipped_meshes = self.integration.context().tessellate(output.shapes);
        let scene_output = graph.latest_version(&graph.aliased_resource("renderer_output")?)?;
        graph.add_pass(self.integration.paint(
            std::slice::from_ref(&scene_output),
            &swapchain,
            vk::AttachmentLoadOp::CLEAR,
            Some(vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            }),
            clipped_meshes,
            output.textures_delta,
        )?);
        Ok(())
    }

    pub fn context(&self) -> egui::Context {
        self.integration.context()
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        let _ = self.integration.handle_event(event);
    }

    pub fn register_texture(&mut self, image: &ImageView) -> Image {
        let id = self.integration.register_user_texture(image);
        // 8 frames to live, then it needs to be registered again (our application always does this every frame anyway)
        self.to_unregister.insert(id, 8);
        Image {
            id,
            size: USize::new(image.width(), image.height()),
        }
    }
}
