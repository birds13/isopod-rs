use ash::{vk::Handle, *};
use super::*;

fn inside(window: (u32, u32), min: vk::Extent2D, max: vk::Extent2D) -> vk::Extent2D {
	vk::Extent2D {
		width: window.0.max(min.width).min(max.width),
		height: window.1.max(min.height).min(max.height),
	}
}

pub struct VKSwapchain {
	pub swapchain: vk::SwapchainKHR,
	pub color_images: Vec<VKImage>,
	pub depth_image: VKImage,
	pub extent: vk::Extent2D,
}

impl VKSwapchain {
	fn new(
		device: &Device,
		allocator: &mut vk_mem::Allocator,
		surface: &vk::SurfaceKHR,
		swapchain_device: &ash::khr::swapchain::Device,
		surface_format: vk::SurfaceFormatKHR,
		surface_capabilities: &vk::SurfaceCapabilitiesKHR,
		depth_stencil_format: vk::Format,
		window_size: (u32, u32),
	) -> anyhow::Result<Self> {

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
		let swapchain = unsafe{swapchain_device.create_swapchain(&swapchain_info, None)}?;

		// depth image
		let depth_image = VKImage::new(
			device, VKImageCreationMethod::Allocator {
				allocator: allocator, usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST
			}, extent.into(), depth_stencil_format, true
		);

		// swapchain images + framebuffers
		let color_images = unsafe{swapchain_device.get_swapchain_images(swapchain)}.unwrap().into_iter().map(|image| {
			VKImage::new(
				device, VKImageCreationMethod::Image(image), extent.into(), surface_format.format, false,
			)
		}).collect::<Vec<_>>();

		Ok(Self {
			swapchain, color_images, extent, depth_image,
		})
	}

	unsafe fn destroy(&mut self, device: &Device, allocator: &mut vk_mem::Allocator, swapchain_device: &ash::khr::swapchain::Device) {
		for image in &mut self.color_images {
			image.destroy(device, allocator);
		}
		self.depth_image.destroy(device, allocator);
		swapchain_device.destroy_swapchain(self.swapchain, None);
	}
}

pub struct VKSurface {
	surface_inst: ash::khr::surface::Instance,
	surface: vk::SurfaceKHR,
	pub format: vk::SurfaceFormatKHR,
	pub capabilities: vk::SurfaceCapabilitiesKHR,

	pub swapchain_device: ash::khr::swapchain::Device,
	pub swapchain: VKSwapchain,
	pub swapchain_is_bad: bool,
}

impl VKSurface {
	pub fn new(
		entry: &Entry,
		inst: &Instance,
		device: &Device,
		phys_device: vk::PhysicalDevice,
		allocator: &mut vk_mem::Allocator,
		window: &sdl2::video::Window,
		depth_stencil_format: vk::Format,
	) -> anyhow::Result<Self> {
		let window_size = window.size();

		// create surface
		let surface_inst = ash::khr::surface::Instance::new(entry, inst);
		let surface = vk::SurfaceKHR::from_raw(window.vulkan_create_surface(inst.handle().as_raw() as usize).unwrap());

		// get surface data
		let surface_formats = unsafe{surface_inst.get_physical_device_surface_formats(phys_device, surface)}?;
		let format = surface_formats[0];
		let capabilities = unsafe{surface_inst.get_physical_device_surface_capabilities(phys_device, surface)}?;

		// swapchain
		let swapchain_device = ash::khr::swapchain::Device::new(inst, device);
		let swapchain = VKSwapchain::new(
			device, allocator, &surface, &swapchain_device,
			format, &capabilities,
			depth_stencil_format, window_size.into(),
		)?;

		Ok(Self {
			surface_inst, surface, format, capabilities, swapchain_device, swapchain,
			swapchain_is_bad: false,
		})
	}

	pub fn reconstruct_swapchain(
		&mut self,
		device: &Device,
		phys_device: &vk::PhysicalDevice,
		allocator: &mut vk_mem::Allocator,
		window_size: (u32, u32),
		depth_stencil_format: vk::Format
	) {
		// todo: something something swachain format could change (also need to update renderpass?)
		unsafe{device.device_wait_idle()}.unwrap();
		self.capabilities = unsafe{self.surface_inst.get_physical_device_surface_capabilities(*phys_device, self.surface)}.unwrap();

		// destory and recreate
		unsafe{self.swapchain.destroy(device, allocator, &self.swapchain_device)};
		self.swapchain = VKSwapchain::new(
			device, allocator, &self.surface, &self.swapchain_device,
			self.format, &self.capabilities,
			depth_stencil_format, window_size,
		).unwrap();
	}

	pub unsafe fn destroy(&mut self, device: &Device, allocator: &mut vk_mem::Allocator) {
		unsafe{self.swapchain.destroy(device, allocator, &self.swapchain_device)};
		self.surface_inst.destroy_surface(self.surface, None);
	}
}