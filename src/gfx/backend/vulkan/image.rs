
use ash::*;
use super::*;
use vk_mem::Alloc;
use crate::gfx::*;

pub enum VKImageCreationMethod {
	New(vk::ImageUsageFlags),
	FromExisting(vk::Image),
}

pub enum VKImageFormat {
	Format(vk::Format),
	DepthStencil,
}

impl VKImageFormat {
	fn is_depth_stencil(&self) -> bool {
		match self {
			Self::DepthStencil => true,
			_ => false,
		}
	}
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
	ctx: Arc<VKCtx>,
}

impl VKImage {
	pub fn new(
		ctx: &Arc<VKCtx>,
		creation_method: VKImageCreationMethod,
		extent: vk::Extent3D,
		format: VKImageFormat,
	) -> Self {
		let (image, allocation) = match creation_method {
			VKImageCreationMethod::New(usage) => {
				let image_info = vk::ImageCreateInfo {
					image_type: if extent.depth <= 1 {
						vk::ImageType::TYPE_2D
					} else {
						vk::ImageType::TYPE_3D
					},
					extent,
					mip_levels: 1,
					array_layers: 1,
					format: match format {
						VKImageFormat::Format(f) => f,
						VKImageFormat::DepthStencil => ctx.depth_stencil_format,
					},
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
				let (image, allocation) = unsafe{ctx.allocator.create_image(&image_info, &allocation_info)}.unwrap();
				(image, Some(allocation))
			},
			VKImageCreationMethod::FromExisting(image) => (image, None),
		};
		let subresource_range = vk::ImageSubresourceRange {
			aspect_mask: if format.is_depth_stencil() {
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
			format: match format {
				VKImageFormat::Format(f) => f,
				VKImageFormat::DepthStencil => ctx.depth_stencil_format,
			},
			subresource_range: subresource_range.clone(),
			..Default::default()
		};
		let view = unsafe{ctx.device.create_image_view(&view_info, None)}.unwrap();
		Self {
			extent, image, allocation, view, subresource_range, is_depth_stencil: format.is_depth_stencil(), ctx: ctx.clone(),
			access_mask: vk::AccessFlags::empty(), layout: vk::ImageLayout::UNDEFINED,
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

impl Drop for VKImage {
	fn drop(&mut self) {
		unsafe {
			self.ctx.device.destroy_image_view(self.view, None);
			if let Some(allocation) = &mut self.allocation {
				self.ctx.allocator.destroy_image(self.image, allocation);
			}
		}
	}
}

pub struct VKFrameBuffer {
	pub color: VKImage,
	pub depth: VKImage,
	pub extent: vk::Extent2D,
	pub color_format_i: usize,
}

pub struct VKSampler {
	pub sampler: vk::Sampler,
	pub ctx: Arc<VKCtx>,
}

impl Drop for VKSampler {
	fn drop(&mut self) {
		unsafe {
			self.ctx.device.destroy_sampler(self.sampler, None);
		}
	}
}