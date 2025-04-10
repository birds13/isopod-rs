
use ash::*;
use vk_mem::Alloc;
use crate::gfx::*;

pub enum VKImageCreationMethod<'a> {
	Allocator { allocator: &'a mut vk_mem::Allocator, usage: vk::ImageUsageFlags },
	Image(vk::Image),
}

pub struct VKImage {
	pub extent: vk::Extent3D,
	pub image: vk::Image,
	pub allocation: Option<vk_mem::Allocation>,
	pub view: vk::ImageView,
	pub access_mask: vk::AccessFlags,
	pub layout: vk::ImageLayout,
	is_depth_stencil: bool,
	pub subresource_range: vk::ImageSubresourceRange,
}

impl VKImage {
	pub fn new(
		device: &Device,
		creation_method: VKImageCreationMethod,
		extent: vk::Extent3D,
		format: vk::Format,
		is_depth_stencil: bool,
	) -> Self {
		let (image, allocation) = match creation_method {
			VKImageCreationMethod::Allocator { allocator, usage } => {
				let image_info = vk::ImageCreateInfo {
					image_type: if extent.depth <= 1 {
						vk::ImageType::TYPE_2D
					} else {
						vk::ImageType::TYPE_3D
					},
					extent,
					mip_levels: 1,
					array_layers: 1,
					format,
					tiling: vk::ImageTiling::OPTIMAL,
					initial_layout: vk::ImageLayout::UNDEFINED,
					usage,
					sharing_mode: vk::SharingMode::EXCLUSIVE,
					samples: vk::SampleCountFlags::TYPE_1,
					..Default::default()
				};
				let allocation_info = vk_mem::AllocationCreateInfo {
					usage: vk_mem::MemoryUsage::AutoPreferDevice,
					..Default::default()
				};
				let (image, allocation) = unsafe{allocator.create_image(&image_info, &allocation_info)}.unwrap();
				(image, Some(allocation))
			},
			VKImageCreationMethod::Image(image) => (image, None),
		};
		let subresource_range = vk::ImageSubresourceRange {
			aspect_mask: if is_depth_stencil {
				vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
			} else {
				vk::ImageAspectFlags::COLOR
			},
			base_mip_level: 0,
			level_count: 1,
			base_array_layer: 0,
			layer_count: 1,
		};
		let view_info = vk::ImageViewCreateInfo {
			image: image,
			view_type: if extent.depth <= 1 {
				vk::ImageViewType::TYPE_2D
			} else {
				vk::ImageViewType::TYPE_3D
			},
			format,
			subresource_range: subresource_range.clone(),
			..Default::default()
		};
		let view = unsafe{device.create_image_view(&view_info, None)}.unwrap();
		Self {
			extent, image, allocation, view, subresource_range, is_depth_stencil,
			access_mask: vk::AccessFlags::empty(), layout: vk::ImageLayout::UNDEFINED,
		}
	}

	pub unsafe fn destroy(&mut self, device: &Device, allocator: &mut vk_mem::Allocator) {
		device.destroy_image_view(self.view, None);
		if let Some(allocation) = &mut self.allocation {
			allocator.destroy_image(self.image, allocation);
		}
	}

	pub fn change_layout_mem_barrier(&mut self, layout: vk::ImageLayout, access_mask: vk::AccessFlags) -> vk::ImageMemoryBarrier<'static> {
		let barrier = vk::ImageMemoryBarrier {
			image: self.image,
			old_layout: self.layout,
			new_layout: layout,
			src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
			dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
			subresource_range: self.subresource_range,
			src_access_mask: self.access_mask,
			dst_access_mask: access_mask,
			..Default::default()
		};
		self.layout = layout;
		self.access_mask = access_mask;
		barrier
	}

	pub fn get_buffer_copy_to(&self, offset: usize) -> vk::BufferImageCopy {
		vk::BufferImageCopy {
			buffer_offset: offset as u64,
			image_subresource: vk::ImageSubresourceLayers {
				aspect_mask: if self.is_depth_stencil {
					vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
				} else {
					vk::ImageAspectFlags::COLOR
				},
				mip_level: 0,
				base_array_layer: 0,
				layer_count: 1,
			},
			image_extent: self.extent,
			..Default::default()
		}
	}

	pub fn texture_attribute_to_vk_format(attribute: TextureAttributeID, normilzation: NormalizationID) -> vk::Format {
		match attribute {
			TextureAttributeID::F32 => vk::Format::R32_SFLOAT,
			TextureAttributeID::Vec2 => vk::Format::R32G32_SFLOAT,
			TextureAttributeID::Vec4 => vk::Format::R32G32B32A32_SFLOAT,
			TextureAttributeID::U8 => match normilzation {
				NormalizationID::None => vk::Format::R8_UINT,
				NormalizationID::Srgb => vk::Format::R8_SRGB,
				NormalizationID::MinusOneToOne => vk::Format::R8_SNORM,
				NormalizationID::ZeroToOne => vk::Format::R8_UNORM,
			},
			TextureAttributeID::U8Vec2 => match normilzation {
				NormalizationID::None => vk::Format::R8G8_UINT,
				NormalizationID::Srgb => vk::Format::R8G8_SRGB,
				NormalizationID::MinusOneToOne => vk::Format::R8G8_SNORM,
				NormalizationID::ZeroToOne => vk::Format::R8G8_UNORM,
			},
			TextureAttributeID::U8Vec4 => match normilzation {
				NormalizationID::None => vk::Format::R8G8B8A8_UINT,
				NormalizationID::Srgb => vk::Format::R8G8B8A8_SRGB,
				NormalizationID::MinusOneToOne => vk::Format::R8G8B8A8_SNORM,
				NormalizationID::ZeroToOne => vk::Format::R8G8B8A8_UNORM,
			},
			TextureAttributeID::U16 => match normilzation {
				NormalizationID::MinusOneToOne => vk::Format::R16_SNORM,
				NormalizationID::ZeroToOne => vk::Format::R16_UNORM,
				_ => vk::Format::R16_UINT,
			},
			TextureAttributeID::U16Vec2 => match normilzation {
				NormalizationID::MinusOneToOne => vk::Format::R16G16_SNORM,
				NormalizationID::ZeroToOne => vk::Format::R16G16_UNORM,
				_ => vk::Format::R16G16_UINT,
			},
			TextureAttributeID::U16Vec4 => match normilzation {
				NormalizationID::MinusOneToOne => vk::Format::R16G16B16A16_SNORM,
				NormalizationID::ZeroToOne => vk::Format::R16G16B16A16_UNORM,
				_ => vk::Format::R16G16B16A16_UINT,
			},
			TextureAttributeID::U32 => vk::Format::R32_UINT,
		}
	}
}

pub struct VKFrameBuffer {
	pub color: VKImage,
	pub depth: VKImage,
	pub extent: vk::Extent2D,
	pub color_format_i: usize,
}

impl VKFrameBuffer {
	pub unsafe fn destroy(&mut self, device: &Device, allocator: &mut vk_mem::Allocator) {
		self.color.destroy(device, allocator);
		self.depth.destroy(device, allocator);
	}
}