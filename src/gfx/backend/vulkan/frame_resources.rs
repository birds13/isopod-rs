

use ash::*;

fn create_semaphore(device: &Device) -> vk::Semaphore {
	unsafe{device.create_semaphore(&Default::default(), None)}.unwrap()
}

fn create_fence(device: &Device, signaled: bool) -> vk::Fence {
	let fence_info = vk::FenceCreateInfo {
		flags: if signaled {
			vk::FenceCreateFlags::SIGNALED
		} else {
			vk::FenceCreateFlags::empty()
		},
		..Default::default()
	};
	unsafe{device.create_fence(&fence_info, None)}.unwrap()
}

fn create_command_pool_and_buffers(device: &Device, n: u32) -> (vk::CommandPool, Vec<vk::CommandBuffer>) {
	let pool_info = vk::CommandPoolCreateInfo {
		flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,// | vk::CommandPoolCreateFlags::TRANSIENT,
		// set this to queue family index
		queue_family_index: 0,
		..Default::default()
	};
	let pool = unsafe{device.create_command_pool(&pool_info, None)}.unwrap();
	let buffer_info = vk::CommandBufferAllocateInfo {
		command_pool: pool,
		level: vk::CommandBufferLevel::PRIMARY,
		command_buffer_count: n,
		..Default::default()
	};
	let buffers = unsafe{device.allocate_command_buffers(&buffer_info)}.unwrap();
	(pool, buffers)
}

pub struct FrameResources {
	pub cmd_pool: vk::CommandPool,
	pub cmd_buffer: vk::CommandBuffer,

	pub material_desc_pool: vk::DescriptorPool,

	pub staging_buffer: super::VKBuffer,

	pub vertex_buffer: super::VKBuffer,
	pub index_buffer: super::VKBuffer,
	pub uniform_buffer: super::VKBuffer,

	pub framebuffer_ready: vk::Semaphore,
	pub render_finished: vk::Semaphore,
	pub resources_usable: vk::Fence,
}

impl FrameResources {
	pub fn new(
		device: &Device,
		allocator: &mut vk_mem::Allocator,
	) -> Self {

		let (cmd_pool, cmd_buffers) = create_command_pool_and_buffers(device, 2);
		let cmd_buffer = cmd_buffers[0];

		// material descriptor pool
		let material_desc_pool_sizes = [
			vk::DescriptorPoolSize {
				ty: vk::DescriptorType::UNIFORM_BUFFER,
				descriptor_count: 4096,
			},
			vk::DescriptorPoolSize {
				ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
				descriptor_count: 4096,
			}
		];
		let material_desc_pool_info = vk::DescriptorPoolCreateInfo {
			max_sets: 4096,
			..Default::default()
		}.pool_sizes(&material_desc_pool_sizes);
		let material_desc_pool = unsafe{device.create_descriptor_pool(&material_desc_pool_info, None)}.unwrap();

		// per frame buffers
		let staging_buffer = super::VKBuffer::new(
			allocator, 1024*1024*4, vk::BufferUsageFlags::TRANSFER_SRC, vk_mem::MemoryUsage::AutoPreferHost
		);
		let vertex_buffer = super::VKBuffer::new(
			allocator, 1024*1024*4, vk::BufferUsageFlags::VERTEX_BUFFER, vk_mem::MemoryUsage::AutoPreferHost
		);
		let index_buffer = super::VKBuffer::new(
			allocator, 1024*1024*2, vk::BufferUsageFlags::INDEX_BUFFER, vk_mem::MemoryUsage::AutoPreferHost
		);
		let uniform_buffer = super::VKBuffer::new(
			allocator, 1024*1024*2, vk::BufferUsageFlags::UNIFORM_BUFFER, vk_mem::MemoryUsage::AutoPreferHost
		);

		Self {
			framebuffer_ready: create_semaphore(device),
			render_finished: create_semaphore(device),
			resources_usable: create_fence(device, true),
			cmd_pool, cmd_buffer, material_desc_pool, staging_buffer, vertex_buffer, index_buffer, uniform_buffer
		}
	}

	pub unsafe fn destroy(&mut self, device: &Device, allocator: &mut vk_mem::Allocator) {

		self.vertex_buffer.destroy(allocator);
		self.index_buffer.destroy(allocator);
		self.uniform_buffer.destroy(allocator);
		self.staging_buffer.destroy(allocator);

		device.destroy_descriptor_pool(self.material_desc_pool, None);

		device.destroy_command_pool(self.cmd_pool, None);

		device.destroy_semaphore(self.framebuffer_ready, None);
		device.destroy_semaphore(self.render_finished, None);
		device.destroy_fence(self.resources_usable, None);
	}
}