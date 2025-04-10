
use ash::*;
use super::*;

pub struct VKIndices {
	pub buffer: VKBuffer,
	pub n: u32,
	pub is_u32: bool,
}

pub struct VKMesh {
	pub vertex_buffer: VKBuffer,
	pub n_vertices: u32,
	pub indices: Option<VKIndices>,
}

impl VKMesh {
	pub fn new(allocator: &mut vk_mem::Allocator, data: &crate::gfx::mesh::MeshDataBytes) -> Self {
		Self {
			vertex_buffer: VKBuffer::new(
				allocator, data.vertex_bytes.len(),
				vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST, vk_mem::MemoryUsage::AutoPreferDevice
			),
			n_vertices: data.n_vertices as u32,
			indices: data.indices.as_ref().map(|index_data| VKIndices {
				buffer: VKBuffer::new(
					allocator, index_data.bytes.len(),
					vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST, vk_mem::MemoryUsage::AutoPreferDevice
				),
				n: index_data.n as u32,
				is_u32: index_data.is_u32,
			}),
		}
	}

	pub unsafe fn destroy(&mut self, allocator: &mut vk_mem::Allocator) {
		self.vertex_buffer.destroy(allocator);
		self.indices.as_mut().map(|index_data| index_data.buffer.destroy(allocator));
	}
}

pub struct VKInstances {
	pub buffer: VKBuffer,
	pub n: u32,
}

impl VKInstances {
	pub fn new(allocator: &mut vk_mem::Allocator, data: &crate::gfx::mesh::InstanceDataBytes) -> Self {
		Self {
			buffer: VKBuffer::new(
				allocator, data.bytes.len(), vk::BufferUsageFlags::VERTEX_BUFFER, vk_mem::MemoryUsage::AutoPreferDevice
			),
			n: data.n as u32,
		}
	}

	pub unsafe fn destroy(&mut self, allocator: &mut vk_mem::Allocator) {
		self.buffer.destroy(allocator);
	}
}