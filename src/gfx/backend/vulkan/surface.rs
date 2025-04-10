use ash::{vk::Handle, *};
use super::*;

fn inside(window: (u32, u32), min: vk::Extent2D, max: vk::Extent2D) -> vk::Extent2D {
	vk::Extent2D {
		width: window.0.max(min.width).min(max.width),
		height: window.1.max(min.height).min(max.height),
	}
}

pub struct VKSwapchain {
	ctx: Arc<VKCtx>,
	pub color_images: Vec<VKImage>,
	pub depth_image: VKImage,
	pub extent: vk::Extent2D,
	pub swapchain: vk::SwapchainKHR,
}

impl VKSwapchain {
	fn new(
		ctx: &Arc<VKCtx>,
		surface: &vk::SurfaceKHR,
		surface_format: vk::SurfaceFormatKHR,
		surface_capabilities: &vk::SurfaceCapabilitiesKHR,
		window_size: (u32, u32),
	) -> Self {

		// swapchain
		const DESIRED_IMAGE_COUNT: u32 = 3;
		let extent = inside(window_size, surface_capabilities.min_image_extent, surface_capabilities.max_image_extent);
		let swapchain_info = vk::SwapchainCreateInfoKHR {
			surface: *surface,
			min_image_count: 
				if surface_capabilities.max_image_count > 0 {
					surface_capabilities.max_image_count.min(surface_capabilities.min_image_count.max(DESIRED_IMAGE_COUNT))
				} else {
					surface_capabilities.min_image_count.max(DESIRED_IMAGE_COUNT)
				},
			image_format: surface_format.format,
			image_color_space: surface_format.color_space,
			image_extent: extent,
			image_array_layers: 1,
			present_mode: vk::PresentModeKHR::FIFO,
			image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
			pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
			composite_alpha: vk::CompositeAlphaFlagsKHR::INHERIT,
			..Default::default()
		};
		let swapchain = unsafe{ctx.swapchain_device.create_swapchain(&swapchain_info, None)}.unwrap();

		// depth image
		let depth_image = VKImage::new(
			ctx, VKImageCreationMethod::New(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST),
			extent.into(), VKImageFormat::DepthStencil,
		);

		// swapchain images + framebuffers
		let color_images = unsafe{ctx.swapchain_device.get_swapchain_images(swapchain)}.unwrap().into_iter().map(|image| {
			VKImage::new(ctx, VKImageCreationMethod::FromExisting(image), extent.into(), VKImageFormat::Format(surface_format.format))
		}).collect::<Vec<_>>();

		Self { swapchain, color_images, extent, depth_image, ctx: ctx.clone() }
	}
}

impl Drop for VKSwapchain {
	fn drop(&mut self) {
		unsafe {
			self.ctx.swapchain_device.destroy_swapchain(self.swapchain, None);
		}
	}
}

// need to do this to enforce drop order
pub struct VKSurfaceInner {
	ctx: Arc<VKCtx>,
	surface: vk::SurfaceKHR,
}

impl Drop for VKSurfaceInner {
	fn drop(&mut self) {
		unsafe {
			self.ctx.surface_inst.destroy_surface(self.surface, None);
		}
	}
}

pub struct VKSurface {
	pub swapchain: VKSwapchain,
	inner: VKSurfaceInner,
	pub format: vk::SurfaceFormatKHR,
	pub capabilities: vk::SurfaceCapabilitiesKHR,
	pub swapchain_is_bad: bool,
}

impl VKSurface {
	pub fn new(ctx: &Arc<VKCtx>,window: &sdl2::video::Window) -> anyhow::Result<Self> {
		let window_size = window.size();

		// create surface
		let surface = vk::SurfaceKHR::from_raw(window.vulkan_create_surface(ctx.inst.handle().as_raw() as usize).unwrap());
		{
			// get surface data
			let surface_formats = unsafe{ctx.surface_inst.get_physical_device_surface_formats(ctx.physical_device, surface)}?;
			let format = surface_formats[0];
			let capabilities = unsafe{ctx.surface_inst.get_physical_device_surface_capabilities(ctx.physical_device, surface)}?;

			// swapchain
			let swapchain = VKSwapchain::new(ctx, &surface, format, &capabilities, window_size.into());

			let inner = VKSurfaceInner { surface, ctx: ctx.clone() };

			Ok(Self {
				inner, capabilities, format, swapchain, swapchain_is_bad: false,
			})
		}.map_err(|e| {
			unsafe { ctx.surface_inst.destroy_surface(surface, None) };
			e
		})
	}

	pub fn reconstruct_swapchain(&mut self, window_size: (u32, u32)) {
		// todo: something something swachain format could change (also need to update renderpass?)
		unsafe{self.inner.ctx.device.device_wait_idle()}.unwrap();
		self.capabilities = unsafe{self.inner.ctx.surface_inst.get_physical_device_surface_capabilities(self.inner.ctx.physical_device, self.inner.surface)}.unwrap();

		// recreate
		self.swapchain = VKSwapchain::new(
			&self.inner.ctx, &self.inner.surface, self.format, &self.capabilities, window_size
		);
	}
}