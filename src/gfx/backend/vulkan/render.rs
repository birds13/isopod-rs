use strum::EnumCount;

use crate::gfx::*;
use super::*;
use std::slice::from_ref;

enum StagingBufferCopy {
	Image { image: vk::Image, copy: vk::BufferImageCopy },
	Buffer { buffer: vk::Buffer, copy: vk::BufferCopy },
}

enum DescWriteInfo {
	Image(vk::DescriptorImageInfo),
	Sampler(vk::DescriptorImageInfo),
	Buffer(vk::DescriptorBufferInfo),
}

enum DrawType {
	NonIndexed { range: std::ops::Range<u32> },
	Indexed { n_indices: u32, buffer: vk::Buffer, offset: u64, is_u32: bool },
}

struct SwapchainDrawData {
	image_index: u32,
}

const STAGING_BUFFER_ALIGN: usize = 64;

fn staging_buffer_buffer_copy_and_align(
	staging_buffer_size: &mut usize,
	staging_buffer_copies: &mut Vec<(Vec<u8>,usize,StagingBufferCopy)>,
	buffer: vk::Buffer,
	data: Vec<u8>
) {
	let size = data.len();
	staging_buffer_copies.push((data, *staging_buffer_size, StagingBufferCopy::Buffer {
		buffer: buffer,
		copy: vk::BufferCopy {
			src_offset: *staging_buffer_size as u64,
			dst_offset: 0,
			size: size as u64,
		},
	}));
	*staging_buffer_size = align_up(*staging_buffer_size + size, STAGING_BUFFER_ALIGN);
}

impl GfxBackend for VulkanGfxBackend {
	fn render(&mut self, c: &mut GfxCtx) {
		//////////////////////////////////////////////////////////////////////////////////////////
		// STEP 0:
		// do setup for this frame (waiting for fence, starting command buffer, cleanup, etc.)
		//////////////////////////////////////////////////////////////////////////////////////////

		let device = self.ctx.device.clone();

		// get current frame resources for this frame
		let cfr = if self.even_frame {
			&mut self.frame_resources.0
		} else {
			&mut self.frame_resources.1
		};

		// wait for resources to be usable (then reset fence)
		unsafe{device.wait_for_fences(&[cfr.resources_usable], true, u64::MAX)}.unwrap();
		//c.backend_debug_info = format!("gpu wait time: {} micros", gpu_wait_start.elapsed().subsec_micros());
		unsafe{device.reset_fences(&[cfr.resources_usable])}.unwrap();

		// destroy resources that are queued for destruction
		self.destroy_queue.clear();

		// reset command pool
		unsafe{device.reset_command_pool(cfr.cmd_pool, vk::CommandPoolResetFlags::empty())}.unwrap();
		let cmd_begin_info = vk::CommandBufferBeginInfo {
			flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
			..Default::default()
		};

		// start command buffer
		let cmd = cfr.cmd_buffer;
		unsafe{device.begin_command_buffer(cfr.cmd_buffer, &cmd_begin_info)}.unwrap();

		// setup queues for differnt things
		let mut staging_buffer_size = 0;
		let mut staging_buffer_copies = vec![];
		let mut transfer_dst_layout_barriers = vec![];
		let mut shader_read_layout_barriers = vec![];
		let mut depth_attachment_layout_barriers = vec![];
		let mut color_image_clears = vec![];
		let mut depth_image_clears = vec![];
		
		//////////////////////////////////////////////////////////////////////////////////////////
		// STEP 1:
		// handle resource updates
		//////////////////////////////////////////////////////////////////////////////////////////
		
		for resource_update in c.frame_data.resource_update_queue.get_mut().drain(..) {
			use crate::gfx::resource::ResourceUpdate;
			match resource_update {
				ResourceUpdate::CreateShader { id, def } => {
					let pipeline = VKGfxPipeline::new(&self.ctx, &def, self.surface.format.format).unwrap();
					self.pipelines.insert(id, pipeline);
				},
				ResourceUpdate::CreateTexture2D { id, bytes, meta } => {
					let extent = uvec2_to_extent2d(meta.size);
					let mut texture = VKImage::new(
						&self.ctx, VKImageCreationMethod::New(
							vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC,
						), extent2d_to_extent3d(extent),
						VKImageFormat::Format(VKImage::texture_format_to_vk_format(meta.format))
					);
					transfer_dst_layout_barriers.push(texture.change_layout_mem_barrier(vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::AccessFlags::TRANSFER_WRITE));
					shader_read_layout_barriers.push(texture.change_layout_mem_barrier(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::AccessFlags::SHADER_READ));
					let len = bytes.len();
					staging_buffer_copies.push((bytes, staging_buffer_size, StagingBufferCopy::Image {
						image: texture.image,
						copy: texture.get_buffer_copy_to(staging_buffer_size),
					}));
					staging_buffer_size = align_up(staging_buffer_size + len, STAGING_BUFFER_ALIGN);
					self.texture_2ds.insert(id, texture);
				},
				ResourceUpdate::CreateSampler { id, def } => {
					let address_mode = match def.wrap_mode {
						SamplerWrapMode::Extend => vk::SamplerAddressMode::CLAMP_TO_BORDER,
						SamplerWrapMode::Mirror => vk::SamplerAddressMode::MIRRORED_REPEAT,
						SamplerWrapMode::Repeat => vk::SamplerAddressMode::REPEAT,
					};
					let create_info = vk::SamplerCreateInfo {
						address_mode_u: address_mode,
						address_mode_v: address_mode,
						address_mode_w: address_mode,
						min_filter: if def.min_linear { vk::Filter::LINEAR } else { vk::Filter::NEAREST },
						mag_filter: if def.mag_linear { vk::Filter::LINEAR } else { vk::Filter::NEAREST },
						..Default::default()
					};
					let sampler = unsafe{device.create_sampler(&create_info, None)}.unwrap();
					self.samplers.insert(id, VKSampler { sampler, ctx: self.ctx.clone() });
				},
				ResourceUpdate::CreateMesh { id, data } => {
					let mesh = VKMesh::new(&self.ctx, &data);
					staging_buffer_buffer_copy_and_align(&mut staging_buffer_size, &mut staging_buffer_copies, mesh.vertex_buffer.buffer, data.vertex_bytes);
					if let Some(index_data) = data.indices {
						staging_buffer_buffer_copy_and_align(&mut staging_buffer_size, &mut staging_buffer_copies, mesh.indices.as_ref().unwrap().buffer.buffer, index_data.bytes);
					}
					self.meshes.insert(id, mesh);
				},
				ResourceUpdate::CreateInstances { id, data } => {
					let instances = VKInstances::new(&self.ctx, &data);
					staging_buffer_buffer_copy_and_align(&mut staging_buffer_size, &mut staging_buffer_copies, instances.buffer.buffer, data.bytes);
					self.instances.insert(id, instances);
				},
				ResourceUpdate::CreateUniform { id, data } => {
					let buffer = VKBuffer::new(
						&self.ctx, data.len(),
						vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
						vk_mem::MemoryUsage::AutoPreferDevice,
					);
					self.uniforms.insert(id, buffer);
				},
				ResourceUpdate::CreateFramebuffer { id, meta } => {
					let extent = uvec2_to_extent2d(meta.size);
					let mut color = VKImage::new(
						&self.ctx, VKImageCreationMethod::New(
							vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC
						), extent.into(), VKImageFormat::Format(VKImage::texture_format_to_vk_format(meta.format)),
					);
					transfer_dst_layout_barriers.push(color.change_layout_mem_barrier(
						vk::ImageLayout::TRANSFER_DST_OPTIMAL,
						vk::AccessFlags::TRANSFER_WRITE,
					));
					color_image_clears.push((color.image, color.subresource_range.clone()));
					shader_read_layout_barriers.push(color.change_layout_mem_barrier(
						vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
						vk::AccessFlags::SHADER_READ,
					));
					let mut depth = VKImage::new(
						&self.ctx, VKImageCreationMethod::New(
							vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST
						), extent.into(), VKImageFormat::DepthStencil,
					);
					transfer_dst_layout_barriers.push(depth.change_layout_mem_barrier(
						vk::ImageLayout::TRANSFER_DST_OPTIMAL,
						vk::AccessFlags::TRANSFER_WRITE,
					));
					depth_image_clears.push((depth.image, depth.subresource_range.clone()));
					depth_attachment_layout_barriers.push(depth.change_layout_mem_barrier(
						vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
						vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
					));
					self.framebuffers.insert(id, VKFrameBuffer { color, depth, extent, color_format_index: VKImage::texture_format_to_index(meta.format) });
				},
    			ResourceUpdate::Free { id, ty } => {
					match ty {
        				ResourceFreeType::Shader => { self.destroy_queue.push(Destroyable::GfxPipeline(self.pipelines.remove(id).unwrap())); },
						ResourceFreeType::Texture2D => { self.destroy_queue.push(Destroyable::Texture2D(self.texture_2ds.remove(id).unwrap())); },
						ResourceFreeType::Mesh => { self.destroy_queue.push(Destroyable::Mesh(self.meshes.remove(id).unwrap())); },
						ResourceFreeType::Instances => { self.destroy_queue.push(Destroyable::Instances(self.instances.remove(id).unwrap())); },
						ResourceFreeType::Uniform => { self.destroy_queue.push(Destroyable::Uniform(self.uniforms.remove(id).unwrap())); },
						ResourceFreeType::Framebuffer => { self.destroy_queue.push(Destroyable::FrameBuffer(self.framebuffers.remove(id).unwrap())); },
						ResourceFreeType::Sampler => { self.destroy_queue.push(Destroyable::Sampler(self.samplers.remove(id).unwrap())); },
					}
				},
			}
		}

		//////////////////////////////////////////////////////////////////////////////////////////
		// STEP 2:
		// prepare for swapchain rendering (if able)
		//////////////////////////////////////////////////////////////////////////////////////////

		let window_size = c.window_canvas.size;

		// dont do anything if the window size doesnt make sense
		let mut swapchain_draw_data = if window_size.x > 0 && window_size.y > 0 {
			// fix swapchain if it is bad
			if self.surface.swapchain_is_bad {
				self.surface.reconstruct_swapchain(window_size);
			}
			// try get next image and do setup if success
			match unsafe{self.ctx.swapchain_device.acquire_next_image(
				self.surface.swapchain.swapchain, std::u64::MAX, cfr.framebuffer_ready, vk::Fence::null()
			)} {
				Ok((image_index, suboptimal)) => {
					if suboptimal {
						self.surface.swapchain_is_bad = true;
					}
					let color_image = &mut self.surface.swapchain.color_images[image_index as usize];
					let depth_image = &mut self.surface.swapchain.depth_image;
					transfer_dst_layout_barriers.push(color_image.change_layout_mem_barrier(
						vk::ImageLayout::TRANSFER_DST_OPTIMAL,
						vk::AccessFlags::TRANSFER_WRITE,
					));
					transfer_dst_layout_barriers.push(depth_image.change_layout_mem_barrier(
						vk::ImageLayout::TRANSFER_DST_OPTIMAL,
						vk::AccessFlags::TRANSFER_WRITE,
					));
					depth_attachment_layout_barriers.push(depth_image.change_layout_mem_barrier(
						vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
						vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
					));
					color_image_clears.push((color_image.image, color_image.subresource_range.clone()));
					depth_image_clears.push((depth_image.image, depth_image.subresource_range.clone()));
					Some(SwapchainDrawData {
						image_index,
					})
				},
				Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
					self.surface.swapchain_is_bad = true;
					None
				},
				Err(e) => {panic!("{}",e)},
			}
		} else {
			None
		};

		//////////////////////////////////////////////////////////////////////////////////////////
		// STEP 3:
		// handle large data transfers to GPU and image clears
		//////////////////////////////////////////////////////////////////////////////////////////

		// transition images to correct format for recieving transfers
		if !transfer_dst_layout_barriers.is_empty() {
			unsafe{device.cmd_pipeline_barrier(
				cmd, vk::PipelineStageFlags::ALL_COMMANDS, vk::PipelineStageFlags::TRANSFER,
				vk::DependencyFlags::empty(), &[], &[], &transfer_dst_layout_barriers,
			);}
		}

		// clear any images that need it
		for (image, range) in color_image_clears {
			unsafe{device.cmd_clear_color_image(
				cmd, image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL, 
				&vk::ClearColorValue{uint32: [0,0,0,1]},
				from_ref(&range)
			);}
		}
		for (image, range) in depth_image_clears {
			unsafe{device.cmd_clear_depth_stencil_image(
				cmd, image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL, 
				&vk::ClearDepthStencilValue{depth: 1.0,stencil:0},
				from_ref(&range)
			);}
		}

		// build staging buffer and add copy commands to command buffer
		cfr.staging_buffer.expand_to_fit( staging_buffer_size);
		let staging_buffer_buffer = cfr.staging_buffer.buffer;
		cfr.staging_buffer.map(|staging_buffer_mem| {
			for (bytes, start, staging_buffer_copy) in staging_buffer_copies.iter() {
				staging_buffer_mem[*start..*start+bytes.len()].copy_from_slice(&bytes);
				match staging_buffer_copy {
					StagingBufferCopy::Image { image, copy } => {
						unsafe{device.cmd_copy_buffer_to_image(
							cmd, staging_buffer_buffer, 
							*image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, from_ref(copy),
						)};
					},
					StagingBufferCopy::Buffer { buffer, copy } => {
						unsafe{device.cmd_copy_buffer(cmd, staging_buffer_buffer,
							*buffer, from_ref(copy),
						);}
					}
				}
			}
		});

		// handle immediate mode rendering transfers for meshes/instances/uniforms
		let imm_vertices = c.frame_data.vertices.get_mut();
		cfr.vertex_buffer.expand_to_fit(imm_vertices.len());
		cfr.vertex_buffer.map(|mem| {
			mem[0..imm_vertices.len()].copy_from_slice(&imm_vertices);
		});
		let imm_indices = c.frame_data.indices.get_mut();
		cfr.index_buffer.expand_to_fit(imm_indices.len());
		cfr.index_buffer.map(|mem| {
			mem[0..imm_indices.len()].copy_from_slice(&imm_indices);
		});
		let imm_uniforms = c.frame_data.uniforms.get_mut();
		cfr.uniform_buffer.expand_to_fit(imm_uniforms.len());
		cfr.uniform_buffer.map(|mem| {
			mem[0..imm_uniforms.len()].copy_from_slice(&imm_uniforms);
		});

		//////////////////////////////////////////////////////////////////////////////////////////
		// STEP 4:
		// prepare for general rendering
		//////////////////////////////////////////////////////////////////////////////////////////

		// transition images that will be used in shader reads
		if !shader_read_layout_barriers.is_empty() {
			unsafe{device.cmd_pipeline_barrier(
				cmd, vk::PipelineStageFlags::TRANSFER,
				vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
				vk::DependencyFlags::empty(), &[], &[], &shader_read_layout_barriers,
			);}
		}

		// transition depth images to correct format
		if !depth_attachment_layout_barriers.is_empty() {
			unsafe{device.cmd_pipeline_barrier(
				cmd, vk::PipelineStageFlags::TRANSFER,
				vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
				vk::DependencyFlags::empty(), &[], &[], &depth_attachment_layout_barriers,
			);}
		}

		// transition swapchain image to correct format if able
		if let Some(data) = &mut swapchain_draw_data {
			let swapchain_attachment_barrier = self.surface.swapchain.color_images[data.image_index as usize].change_layout_mem_barrier(
				vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
				vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
			);
			unsafe{device.cmd_pipeline_barrier(
				cmd, vk::PipelineStageFlags::TRANSFER,
				vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
				vk::DependencyFlags::empty(), &[], &[], from_ref(&swapchain_attachment_barrier),
			);}
		}

		//////////////////////////////////////////////////////////////////////////////////////////
		// STEP 5:
		// setup material descriptor sets
		//////////////////////////////////////////////////////////////////////////////////////////

		unsafe{device.reset_descriptor_pool(cfr.material_desc_pool, vk::DescriptorPoolResetFlags::empty())}.unwrap();
		let mut desc_set_layouts = vec![];
		let mut current_pipeline = None;
		let mut desc_write_info = vec![];
		for cmd in c.frame_data.draw_cmd_queue.get_mut().iter() {match cmd {
			DrawCmd::SetShader { id } => {
				current_pipeline = Some(self.pipelines.get_mut(*id).unwrap());
			},
			DrawCmd::SetMaterial { attributes, slot } => {
				let pipeline = current_pipeline.as_mut().unwrap();
				desc_set_layouts.push(pipeline.material_layouts[*slot]);
				desc_write_info.push(attributes.iter().map(|desc| {match desc.inner {
					MaterialAttributeRefIDInner::Texture2D { id } => DescWriteInfo::Image(vk::DescriptorImageInfo {
						image_view: self.texture_2ds.get(id).unwrap().view,
						image_layout: self.texture_2ds.get(id).unwrap().layout,
						..Default::default()
					}),
					MaterialAttributeRefIDInner::Sampler { id } => DescWriteInfo::Sampler(vk::DescriptorImageInfo {
						sampler: self.samplers.get(id).unwrap().sampler,
						..Default::default()
					}),
					MaterialAttributeRefIDInner::FramebufferColor { id } => DescWriteInfo::Image(vk::DescriptorImageInfo {
						image_view: self.framebuffers.get(id).unwrap().color.view,
						image_layout: self.framebuffers.get(id).unwrap().color.layout,
						..Default::default()
					}),
					MaterialAttributeRefIDInner::Uniform { id } => DescWriteInfo::Buffer(self.uniforms.get(id).unwrap().desc_whole_buffer_info()),
					MaterialAttributeRefIDInner::ImmediateUniform { start, len } => DescWriteInfo::Buffer(cfr.uniform_buffer.desc_partial_buffer_info(start, len)),
					_ => panic!("not implemented"),
				}}).collect::<Vec<_>>());
			},
			_ => {},
		}}
		let desc_set_alloc_info = vk::DescriptorSetAllocateInfo {
			descriptor_pool: cfr.material_desc_pool,
			..Default::default()
		}.set_layouts(&desc_set_layouts);
		let desc_sets = if desc_set_alloc_info.descriptor_set_count > 0 {
			unsafe{device.allocate_descriptor_sets(&desc_set_alloc_info)}.unwrap()
		} else {
			vec![]
		};

		// determine resource descriptor set writes and then submit them
		let mut desc_set_writes = vec![];
		for (set, info) in desc_sets.iter().zip(desc_write_info.iter()) {
			for (binding, info) in info.iter().enumerate() {
				let write = vk::WriteDescriptorSet {
					dst_set: *set,
					dst_binding: binding as u32,
					descriptor_type: match info {
						DescWriteInfo::Image(_) => vk::DescriptorType::SAMPLED_IMAGE,
						DescWriteInfo::Sampler(_) => vk::DescriptorType::SAMPLER,
						DescWriteInfo::Buffer(_) => vk::DescriptorType::UNIFORM_BUFFER,
					},
					..Default::default()
				};
				desc_set_writes.push(match info {
					DescWriteInfo::Image(info) => write.image_info(from_ref(info)),
					DescWriteInfo::Sampler(info) => write.image_info(from_ref(info)),
					DescWriteInfo::Buffer(info) => write.buffer_info(from_ref(info)),
				});
			}
		}
		if desc_set_writes.len() > 0 {
			unsafe{device.update_descriptor_sets(&desc_set_writes, &[]);}
		}

		// material descriptor sets are now ready to use
		let mut desc_sets_iter = desc_sets.iter();

		//////////////////////////////////////////////////////////////////////////////////////////
		// STEP 6:
		// actually do the draw commands
		//////////////////////////////////////////////////////////////////////////////////////////
		
		let mut current_target = None;
		let mut current_target_format_i = VKImage::N_TEXTURE_FORMATS;
		let mut current_pipeline = self.pipelines.iter().next().unwrap();
		let mut vertex_buffers = vec![];
		let mut vertex_buffer_offsets = vec![];
		for draw_cmd in c.frame_data.draw_cmd_queue.get_mut().iter() { match draw_cmd {
			DrawCmd::SetCanvas { id, clear_color } => if current_target != Some(*id) {

				// end previous pass
				if current_target.is_some() {
					unsafe{device.cmd_end_rendering(cmd);};
				}

				// swap framebuffer back to shader read if needed
				match current_target {
					Some(CanvasID::Framebuffer(id)) => {
						let framebuffer = self.framebuffers.get_mut(id).unwrap();
						let barrier = framebuffer.color.change_layout_mem_barrier(
							vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::AccessFlags::SHADER_READ
						);
						unsafe{device.cmd_pipeline_barrier(
							cmd, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
							vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::VERTEX_SHADER,
							vk::DependencyFlags::empty(), &[], &[], from_ref(&barrier),
						);}
					},
					_ => {},
				}

				if let Some((extent, color, depth, format_i)) = match id {
					CanvasID::Screen => {
						swapchain_draw_data.as_ref().map(|data| {
							(
								self.surface.swapchain.extent, &mut self.surface.swapchain.color_images[data.image_index as usize],
								&mut self.surface.swapchain.depth_image, VKImage::N_TEXTURE_FORMATS
							)
						})
					},
					CanvasID::Framebuffer(id) => {
						let framebuffer = self.framebuffers.get_mut(*id).unwrap();
						let mem_barrier = framebuffer.color.change_layout_mem_barrier(
							vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::COLOR_ATTACHMENT_READ
						);
						unsafe{device.cmd_pipeline_barrier(
							cmd, vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::VERTEX_SHADER,
							vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
							vk::DependencyFlags::empty(), &[], &[], from_ref(&mem_barrier),
						);}
						Some((framebuffer.extent, &mut framebuffer.color, &mut framebuffer.depth, framebuffer.color_format_index))
					},
				} {
					// start new pass
					let color_attachment = vk::RenderingAttachmentInfoKHR {
						image_view: color.view,
						image_layout: color.layout,
						load_op: if clear_color.is_some() { vk::AttachmentLoadOp::CLEAR } else { vk::AttachmentLoadOp::LOAD },
						store_op: vk::AttachmentStoreOp::STORE,
						clear_value: vk::ClearValue {color: vk::ClearColorValue {float32: clear_color.unwrap_or_default().into() }},
						..Default::default()
					};
					let depth_attachment = vk::RenderingAttachmentInfoKHR {
						image_view: depth.view,
						image_layout: depth.layout,
						load_op: if clear_color.is_some() { vk::AttachmentLoadOp::CLEAR } else { vk::AttachmentLoadOp::LOAD },
						store_op: vk::AttachmentStoreOp::STORE,
						clear_value: vk::ClearValue{depth_stencil: vk::ClearDepthStencilValue{ depth: 1.0, stencil: 0}},
						..Default::default()
					};
					let rendering_info = vk::RenderingInfoKHR {
						render_area: vk::Rect2D::default().extent(extent),
						layer_count: 1,
						..Default::default()
					}.color_attachments(from_ref(&color_attachment))
					.depth_attachment(&depth_attachment)
					.stencil_attachment(&depth_attachment);
					unsafe{device.cmd_begin_rendering(cmd, &rendering_info)};

					// viewport/scissor
					let viewport = vk::Viewport {
						width: extent.width as f32,
						height: extent.height as f32,
						max_depth: 1.,
						..Default::default()
					};
					unsafe{device.cmd_set_viewport(cmd, 0, from_ref(&viewport));}
					let scissor = vk::Rect2D { extent, ..Default::default() };
					unsafe{device.cmd_set_scissor(cmd, 0, from_ref(&scissor));}

					// mark as new target
					current_target = Some(*id);
					current_target_format_i = format_i;
				}
			}
			DrawCmd::SetShader { id } => if current_target.is_some() {
				current_pipeline = self.pipelines.get(*id).unwrap();
				unsafe{device.cmd_bind_pipeline(
					cmd, vk::PipelineBindPoint::GRAPHICS, 
					current_pipeline.variants[current_target_format_i]
				);}
			},
			DrawCmd::SetMaterial { slot, .. } => if current_target.is_some()  {
				unsafe{device.cmd_bind_descriptor_sets(
					cmd, vk::PipelineBindPoint::GRAPHICS,
					current_pipeline.layout, *slot as u32, from_ref(desc_sets_iter.next().unwrap()), &[]
				);}
			},
			DrawCmd::DrawMesh(MeshDrawCmdDesc{ mesh, instances, push })=> if current_target.is_some() {
				// push constants
				unsafe{device.cmd_push_constants(cmd, current_pipeline.layout, 
					vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT, 0, push
				)};

				// mesh
				vertex_buffers.clear();
				vertex_buffer_offsets.clear();
				let draw_type = match mesh {
					MeshDraw::Immediate(mesh) => {
						vertex_buffers.push(cfr.vertex_buffer.buffer);
						vertex_buffer_offsets.push(mesh.start as u64);
						if let Some(indices) = &mesh.indices {
							DrawType::Indexed { n_indices: indices.n, buffer: cfr.index_buffer.buffer, offset: indices.start as u64, is_u32: indices.is_u32 }
						} else {
							DrawType::NonIndexed { range: 0..mesh.n }
						}
					},
					MeshDraw::Resource(id) => {
						let mesh = self.meshes.get(*id).unwrap();
						vertex_buffers.push(mesh.vertex_buffer.buffer);
						vertex_buffer_offsets.push(0);
						if let Some(indices) = &mesh.indices {
							DrawType::Indexed { n_indices: indices.n, buffer: indices.buffer.buffer, offset: 0, is_u32: indices.is_u32 }
						} else {
							DrawType::NonIndexed { range: 0..mesh.n_vertices }
						}
					},
					MeshDraw::Range(range) => DrawType::NonIndexed { range: range.clone() },
				};

				// instances
				let instance_range = match instances {
					InstancesDraw::Immediate(instances) => {
						vertex_buffers.push(cfr.vertex_buffer.buffer);
						vertex_buffer_offsets.push(instances.start as u64);
						0..instances.n
					},
					InstancesDraw::Resource(id) => {
						let instances = self.instances.get(*id).unwrap();
						vertex_buffers.push(instances.buffer.buffer);
						vertex_buffer_offsets.push(0);
						0..instances.n
					},
					InstancesDraw::Range(range) => range.clone(),
				};

				// vertex buffers
				unsafe{device.cmd_bind_vertex_buffers(cmd, 0, &vertex_buffers, &vertex_buffer_offsets);}

				// draw
				match draw_type {
					DrawType::NonIndexed { range } => {
						unsafe{device.cmd_draw(
							cmd, range.end-range.start,
							instance_range.end-instance_range.start, range.start, instance_range.start
						);}
					},
					DrawType::Indexed { n_indices, buffer, offset, is_u32 } => {
						unsafe{device.cmd_bind_index_buffer(cmd, buffer, offset, if is_u32 {
							vk::IndexType::UINT32
						} else {
							vk::IndexType::UINT16
						});}
						unsafe{device.cmd_draw_indexed(
							cmd, n_indices, instance_range.end-instance_range.start, 0, 0, instance_range.start
						);}
					},
				}
			},
		}}

		// end pass if needed
		if current_target.is_some() {
			unsafe{device.cmd_end_rendering(cmd);};
		}

		// swap framebuffer back to shader read if needed
		match current_target {
			Some(CanvasID::Framebuffer(id)) => {
				let framebuffer = self.framebuffers.get_mut(id).unwrap();
				let barrier = framebuffer.color.change_layout_mem_barrier(
					vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::AccessFlags::SHADER_READ
				);
				unsafe{device.cmd_pipeline_barrier(
					cmd, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
					vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::VERTEX_SHADER,
					vk::DependencyFlags::empty(), &[], &[], from_ref(&barrier),
				);}
			},
			_ => {},
		}

		//////////////////////////////////////////////////////////////////////////////////////////
		// STEP 6:
		// finalize render
		//////////////////////////////////////////////////////////////////////////////////////////

		// get swapchain image ready for present
		if let Some(data) = &mut swapchain_draw_data {
			let swapchain_present_mem_barrier = self.surface.swapchain.color_images[data.image_index as usize].change_layout_mem_barrier(
				vk::ImageLayout::PRESENT_SRC_KHR, vk::AccessFlags::NONE
			);
			unsafe{device.cmd_pipeline_barrier(
				cmd, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::FRAGMENT_SHADER,
				vk::PipelineStageFlags::BOTTOM_OF_PIPE,
				vk::DependencyFlags::empty(), &[], &[], from_ref(&swapchain_present_mem_barrier),
			);}
		}

		// end buffer
		unsafe{device.end_command_buffer(cmd)}.unwrap();

		// submit
		let submit_info = vk::SubmitInfo::default()
			.wait_semaphores(from_ref(&cfr.framebuffer_ready))
			.signal_semaphores(from_ref(&cfr.render_finished))
			.wait_dst_stage_mask(&[vk::PipelineStageFlags::BOTTOM_OF_PIPE])
			.command_buffers(from_ref(&cmd));
		unsafe{device.queue_submit(self.ctx.queue, from_ref(&submit_info), cfr.resources_usable)}.unwrap();

		// present if able
		if let Some(data) = &swapchain_draw_data {
			let present_info = vk::PresentInfoKHR::default()
			.image_indices(from_ref(&data.image_index))
			.swapchains(from_ref(&self.surface.swapchain.swapchain))
			.wait_semaphores(from_ref(&cfr.render_finished));
			unsafe{self.ctx.swapchain_device.queue_present(self.ctx.queue, &present_info)}.unwrap();
		}
		
		// change even/odd frame
		self.even_frame = !self.even_frame;
	}
}