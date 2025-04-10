
use ash::*;
use vk_mem::Alloc;

use super::*;

pub struct VKBuffer {
	pub size: usize,
	pub buffer: vk::Buffer,
	pub allocation: vk_mem::Allocation,
	usage: vk::BufferUsageFlags,
	mem_usage: vk_mem::MemoryUsage,
	ctx: Arc<VKCtx>,
}

impl VKBuffer {
	pub fn new(ctx: &Arc<VKCtx>, size: usize, usage: vk::BufferUsageFlags, mem_usage: vk_mem::MemoryUsage) -> Self {
		let buffer_info = vk::BufferCreateInfo {
			size: size as u64,
			usage,
			sharing_mode: vk::SharingMode::EXCLUSIVE,
			..Default::default()
		};
		let allocation_info = vk_mem::AllocationCreateInfo {
			flags: if mem_usage == vk_mem::MemoryUsage::AutoPreferHost {
				vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM
			} else {
				vk_mem::AllocationCreateFlags::empty()
			},
			usage: mem_usage,
			..Default::default()
		};
		let (buffer, allocation) = unsafe{ctx.allocator.create_buffer(&buffer_info, &allocation_info)}.unwrap();
		Self {
			buffer, size, allocation, usage, mem_usage, ctx: ctx.clone(),
		}
	}

	pub fn map<F: FnOnce(&mut [u8])>(&mut self, f: F) {
		let ptr = unsafe{self.ctx.allocator.map_memory(&mut self.allocation)}.unwrap();
		f(unsafe{std::slice::from_raw_parts_mut(ptr, self.size)});
		unsafe{self.ctx.allocator.unmap_memory(&mut self.allocation);}
	}

	pub fn expand_to_fit(&mut self, size: usize) {
		if self.size < size {
			*self = VKBuffer::new(&self.ctx, size * 2, self.usage, self.mem_usage);
		}
	}

	pub fn desc_whole_buffer_info(&self) -> vk::DescriptorBufferInfo {
		vk::DescriptorBufferInfo {
			buffer: self.buffer,
			offset: 0,
			range: self.size as u64,
		}
	}

	pub fn desc_partial_buffer_info(&self, start: usize, len: usize) -> vk::DescriptorBufferInfo {
		vk::DescriptorBufferInfo {
			buffer: self.buffer,
			offset: start as u64,
			range: len as u64,
		}
	}
}

impl Drop for VKBuffer {
	fn drop(&mut self) {
		unsafe {
			self.ctx.allocator.destroy_buffer(self.buffer, &mut self.allocation);
		}
	}
}