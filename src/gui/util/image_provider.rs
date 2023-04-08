use crate::gfx::util::targets::RenderTargets;
use crate::gui::util::image::Image;
use crate::gui::util::integration::UIIntegration;
use crate::gui::util::size::USize;

pub trait ImageProvider {
    fn get_image(&mut self, size: impl Into<USize>) -> Option<Image>;
}

pub struct RenderTargetImageProvider<'a> {
    pub targets: &'a mut RenderTargets,
    pub integration: &'a mut UIIntegration,
    pub name: &'a str,
}

impl ImageProvider for RenderTargetImageProvider<'_> {
    fn get_image(&mut self, size: impl Into<USize>) -> Option<Image> {
        // Make sure next frames output with our requested size
        let size = size.into();
        self.targets
            .set_output_resolution(size.x(), size.y())
            .ok()?;
        // Then grab our color output.
        let image = self.targets.get_target_view(self.name).unwrap();
        // We can re-register the same image, nothing will happen.
        let handle = self.integration.register_texture(&image);
        Some(handle)
    }
}
