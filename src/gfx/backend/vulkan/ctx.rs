
use std::{ffi::CString, sync::Mutex};

use anyhow::Context;
use ash::*;
use super::*;

pub struct VKCtx {
	pub device: Device,
	pub swapchain_device: ash::khr::swapchain::Device,
	pub inst: Instance,
	pub surface_inst: ash::khr::surface::Instance,
	pub queue: vk::Queue,
	pub physical_device: vk::PhysicalDevice,
	pub depth_stencil_format: vk::Format,
	pub allocator: UnsafeDestroyable<vk_mem::Allocator>,
}

impl Drop for VKCtx {
	fn drop(&mut self) {
		unsafe {
			self.allocator.destroy();
			self.device.destroy_device(None);
			self.inst.destroy_instance(None);
		}
	}
}

impl VKCtx {
	pub fn new(extensions: &[CString]) -> anyhow::Result<Self> {
		// setup entry
		let entry = Entry::linked();
		let app_info = vk::ApplicationInfo {
			api_version: vk::make_api_version(0, 1, 3, 0),
			..Default::default()
		};

		// add extensions
		let inst_extension_ptrs = extensions.iter().map(|ext| ext.as_ptr()).collect::<Vec<_>>();

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

		{
			// get physical device
			let physical_devices = unsafe{inst.enumerate_physical_devices()}.context("couldn't enumerate physical devices")?;
			let physical_device =physical_devices.into_iter().next().context("no supported devices")?;

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
			let device = unsafe{inst.create_device(physical_device, &device_create_info, None)}.context("device not supported")?;
			let main_queue = unsafe{device.get_device_queue(0, 0)};

			{
				// allocator
				let allocator_create_info = vk_mem::AllocatorCreateInfo::new(&inst, &device, physical_device);
				let allocator = UnsafeDestroyable::new(unsafe{vk_mem::Allocator::new(allocator_create_info)}?);

				// context creation was successful
				Ok(VKCtx {
					swapchain_device: ash::khr::swapchain::Device::new(&inst, &device),
					surface_inst: ash::khr::surface::Instance::new(&entry, &inst),
					physical_device,
					depth_stencil_format,
					device: device.clone(),
					queue: main_queue,
					inst: inst.clone(),
					allocator,
				})
			}.map_err(|e| {
				unsafe { device.destroy_device(None); }
				e
			})
		}.map_err(|e| {
			unsafe { inst.destroy_instance(None) };
			e
		})
	}
}