
mod vulkan;

pub use super::GfxCtx;

pub use vulkan::VulkanGfxBackend;

pub trait GfxBackend {
	fn render(&mut self, c: &mut GfxCtx);
}