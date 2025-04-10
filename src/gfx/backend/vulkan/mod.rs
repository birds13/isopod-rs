use crate::util::*;

use super::*;
use anyhow::Context;
use ash::*;

mod frame_resources;
mod pipeline;
mod surface;
mod image;
mod render;
mod mesh;
mod buffer;
mod util;

use frame_resources::*;
use pipeline::*;
use surface::*;
use image::*;
use mesh::*;
use buffer::*;
use util::*;

pub struct VulkanGfxBackend {
	inst: Instance,
	device: Device,
	physical_device: vk::PhysicalDevice,
	main_queue: vk::Queue,
	allocator: UnsafeDestroyable<vk_mem::Allocator>,
	surface: VKSurface,

	depth_stencil_format: vk::Format,

	pipelines: SparseVec<VKGfxPipeline>,
	texture_2ds: SparseVec<VKImage>,
	samplers: SparseVec<vk::Sampler>,
	meshes: SparseVec<VKMesh>,
	instances: SparseVec<VKInstances>,
	uniforms: SparseVec<VKBuffer>,
	framebuffers: SparseVec<VKFrameBuffer>,

	frame_resources: (FrameResources, FrameResources),
	even_frame: bool,
}

impl VulkanGfxBackend {
	pub fn load(sdl_video: &sdl2::VideoSubsystem, mut window_builder: sdl2::video::WindowBuilder) -> anyhow::Result<(Self, sdl2::video::Window)> {

		// do sdl setup
		sdl_video.vulkan_load_library_default().unwrap();
		let window = window_builder.vulkan().build().unwrap();

		// setup entry
		let entry = Entry::linked();
		let app_info = vk::ApplicationInfo {
			api_version: vk::make_api_version(0, 1, 3, 0),
			..Default::default()
		};

		// determine and add extensions
		let inst_extension_names = window.vulkan_instance_extensions().unwrap().into_iter().map(|s| 
			std::ffi::CString::new(s.as_bytes()).unwrap()
		).collect::<Vec<_>>();
		let inst_extension_ptrs = inst_extension_names.iter().map(|ext| ext.as_ptr()).collect::<Vec<_>>();
		println!("{:?}", inst_extension_names);
		assert!(inst_extension_names[0].as_c_str() == c"VK_KHR_surface");

		// validation
		let inst_validation_names = if true {
			vec![c"VK_LAYER_KHRONOS_validation".as_ptr()]
		} else {vec![]};

		// create instance
		let inst_create_info = vk::InstanceCreateInfo {
			p_application_info: &app_info,
			..Default::default()
		}.enabled_extension_names(&inst_extension_ptrs).enabled_layer_names(&inst_validation_names);
		let inst = unsafe{entry.create_instance(&inst_create_info, None)}?;

		// get physical device
		let physical_devices = unsafe{inst.enumerate_physical_devices()}?;
		let physical_device = physical_devices.into_iter().next().context("no supported devices")?;

		// get depth/stencil format
		let depth_stencil_formats = [
			(vk::Format::D24_UNORM_S8_UINT, unsafe{inst.get_physical_device_format_properties(physical_device, vk::Format::D24_UNORM_S8_UINT)}),
			(vk::Format::D32_SFLOAT_S8_UINT, unsafe{inst.get_physical_device_format_properties(physical_device, vk::Format::D32_SFLOAT_S8_UINT)}),
		];
		let depth_stencil_format = depth_stencil_formats.into_iter().filter(|(_, props)| {
			props.optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
		}).next().context("no supported depth/stencil format")?.0;

		// deterime and add device extensions
		let device_extension_names = [
			khr::swapchain::NAME.as_ptr(),
			khr::dynamic_rendering::NAME.as_ptr(),
		];

		// create logical device + queues (one for now)
		let queue_create_infos = vec![vk::DeviceQueueCreateInfo {
			queue_count: 1,
			..Default::default()
		}.queue_priorities(&[1.])];
		let mut dynamic_rendering_feature = vk::PhysicalDeviceDynamicRenderingFeaturesKHR {
			dynamic_rendering: vk::TRUE,
			..Default::default()
		};
		let device_create_info = vk::DeviceCreateInfo::default()
			.enabled_extension_names(&device_extension_names)
			.push_next(&mut dynamic_rendering_feature)
			.queue_create_infos(&queue_create_infos);
		let device = unsafe{inst.create_device(physical_device, &device_create_info, None)}?;
		let main_queue = unsafe{device.get_device_queue(0, 0)};

		// setup vulkan allocator lib
		let allocator_create_info = vk_mem::AllocatorCreateInfo::new(&inst, &device, physical_device);
		let mut allocator = UnsafeDestroyable::new(unsafe{vk_mem::Allocator::new(allocator_create_info)}?);

		// create surface
		let surface = VKSurface::new(&entry, &inst, &device, physical_device, &mut allocator, &window, depth_stencil_format)?;

		// create per-frame resources
		let frame_resources = (
			FrameResources::new(&device, &mut allocator),
			FrameResources::new(&device, &mut allocator),
		);

		Ok((Self {
			inst, device, physical_device, main_queue, surface, allocator, frame_resources, depth_stencil_format,
			even_frame: false,
			pipelines: SparseVec::new(), texture_2ds: SparseVec::new(), meshes: SparseVec::new(), instances: SparseVec::new(), uniforms: SparseVec::new(),
			framebuffers: SparseVec::new(), samplers: SparseVec::new(),
		}, window))
	}
}

impl Drop for VulkanGfxBackend {
	fn drop(&mut self) {
		unsafe {
			self.device.device_wait_idle().unwrap();

			self.frame_resources.0.destroy(&self.device, &mut self.allocator);
			self.frame_resources.1.destroy(&self.device, &mut self.allocator);

			self.surface.destroy(&self.device, &mut self.allocator);

			for mesh in self.meshes.iter_mut() {
				mesh.destroy(&mut self.allocator);
			}
			for instances in self.instances.iter_mut() {
				instances.destroy(&mut self.allocator);
			}
			for framebuffer in self.framebuffers.iter_mut() {
				framebuffer.destroy(&self.device, &mut self.allocator);
			}
			for texture in self.texture_2ds.iter_mut() {
				texture.destroy(&self.device, &mut self.allocator);
			}
			for sampler in self.samplers.iter() {
				self.device.destroy_sampler(*sampler, None);
			}
			for pipeline in self.pipelines.iter_mut() {
				pipeline.destroy(&self.device);
			}
			for uniform in self.uniforms.iter_mut() {
				uniform.destroy(&mut self.allocator);
			}

			self.allocator.destroy();

			self.device.destroy_device(None);
			self.inst.destroy_instance(None);
		}
	}
}