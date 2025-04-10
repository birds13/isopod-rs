
use std::fmt::format;
use std::slice::from_ref;

use ash::*;
use strum::IntoEnumIterator;
use crate::gfx::*;

pub fn framebuffer_format_to_pipeline_format(format: FramebufferFormat) -> vk::Format {
	match format {
		FramebufferFormat::Rgba8Srgb => vk::Format::R8G8B8A8_SRGB,
	}
}

pub struct VKGfxPipeline {
	vertex_module: vk::ShaderModule,
	fragment_module: vk::ShaderModule,
	pub layout: vk::PipelineLayout,
	pub material_layouts: Vec<vk::DescriptorSetLayout>,
	pub variants: Vec<vk::Pipeline>,
}

#[derive(Default)]
struct VKVertexInputDesc {
	bindings: Vec<vk::VertexInputBindingDescription>,
	attributes: Vec<vk::VertexInputAttributeDescription>,
}

fn add_vertex_layout_data(
	desc: &mut VKVertexInputDesc,
	code: &mut String,
	layout: &StructLayout<VertexAttributeID>,
	input_rate: vk::VertexInputRate,
) {
	if layout.is_empty() {
		return;
	}
	let binding = desc.bindings.len() as u32;
	desc.bindings.push(vk::VertexInputBindingDescription {
		binding, stride: layout.size as u32, input_rate,
	});
	for attr in layout.attributes.iter() {
		let location = desc.attributes.len() as u32;
		let (format, type_name) = match attr.attribute {
			VertexAttributeID::F32 => (vk::Format::R32_SFLOAT, "float"),
			VertexAttributeID::Vec2 => (vk::Format::R32G32_SFLOAT, "vec2"),
			VertexAttributeID::Vec3 => (vk::Format::R32G32B32_SFLOAT, "vec3"),
			VertexAttributeID::Vec4 => (vk::Format::R32G32B32A32_SFLOAT, "vec4"),
			VertexAttributeID::U8 => (vk::Format::R8_UINT, "uint"),
			VertexAttributeID::U8Vec2 => (vk::Format::R8G8_UINT, "uvec2"),
			VertexAttributeID::U8Vec4 => (vk::Format::R8G8B8A8_UINT, "uvec4"),
			VertexAttributeID::U8UNorm => (vk::Format::R8_UNORM, "float"),
			VertexAttributeID::U8Vec2UNorm => (vk::Format::R8G8_UNORM, "vec2"),
			VertexAttributeID::U8Vec4UNorm => (vk::Format::R8G8B8A8_UNORM, "vec4"),
			VertexAttributeID::U16 => (vk::Format::R16_UINT, "uint"),
			VertexAttributeID::U16Vec2 => (vk::Format::R16G16_UINT, "uvec2"),
			VertexAttributeID::U16Vec4 => (vk::Format::R16G16B16A16_UINT, "uvec4"),
			VertexAttributeID::U16UNorm => (vk::Format::R16_UNORM, "float"),
			VertexAttributeID::U16Vec2UNorm => (vk::Format::R16G16_UNORM, "vec2"),
			VertexAttributeID::U16Vec4UNorm => (vk::Format::R16G16B16A16_UNORM, "vec4"),
			VertexAttributeID::U32 => (vk::Format::R32_UINT, "uint"),
			VertexAttributeID::U32Vec2 => (vk::Format::R32G32_UINT, "uvec2"),
			VertexAttributeID::U32Vec4 => (vk::Format::R32G32B32A32_UINT, "uvec4"),
		};
		desc.attributes.push(vk::VertexInputAttributeDescription {
			location, binding, offset: attr.offset as u32, format
		});
		code.push_str(&format!("\nlayout(location = {}) in {} {};", location, type_name, attr.name));
	}
}

fn add_uniform_layout_data(code: &mut String, n_structs: &mut usize, prefix: &str, postfix: &str, layout: &StructLayout<UniformAttributeID>) {
	*n_structs += 1;
	let mut struct_code = format!("\n{} _I_{} {{", prefix, n_structs);
	for attr in &layout.attributes {
		match attr.attribute {
			UniformAttributeID::Padding => {},
			simple => {
				let type_name = match simple {
					UniformAttributeID::F32 => "float",
					UniformAttributeID::Vec2 => "vec2",
					UniformAttributeID::Vec3 => "vec3",
					UniformAttributeID::Vec4 => "vec4",
					UniformAttributeID::I32 => "int",
					UniformAttributeID::IVec2 => "ivec2",
					UniformAttributeID::IVec3 => "ivec3",
					UniformAttributeID::IVec4 => "ivec4",
					UniformAttributeID::Mat2 => "mat2",
					UniformAttributeID::Mat3 => "mat3",
					UniformAttributeID::Mat4 => "mat4",
					UniformAttributeID::U32 => "uint",
					UniformAttributeID::UVec2 => "uvec2",
					UniformAttributeID::UVec3 => "uvec3",
					UniformAttributeID::UVec4 => "uvec4",
					_ => panic!("UniformAttributeID not handled properly")
				};
				struct_code.push_str(&format!("\n{} {};", type_name, attr.name));
			}
		}
	}
	struct_code.push_str(&format!("\n}} {};", postfix));
	code.push_str(&struct_code);
}

impl VKGfxPipeline {
	pub fn new(
		device: &Device,
		def: &shader::ShaderFullDefinition,
		surface_format: vk::Format,
		depth_stencil_format: vk::Format,
	) -> anyhow::Result<Self> {

		let mut shared_header = String::from("#version 450\n");
		let mut vertex_header = String::new();
		let mut fragment_header = String::from("\nlayout(location = 0) out vec4 out_color;");		
		let mut n_structs = 0;
		let mut n_varyings = 0;

		let (_, varying_code) = def.partial.code.split_at(def.partial.code.find("[varying]").unwrap());
		let (mut varying_code, vertex_code) = varying_code.split_at(varying_code.find("[vertex]").unwrap());
		let (mut vertex_code, mut fragment_code) = vertex_code.split_at(vertex_code.find("[fragment]").unwrap());

		varying_code = varying_code.strip_prefix("[varying]").unwrap();
		vertex_code = vertex_code.strip_prefix("[vertex]").unwrap();
		fragment_code = fragment_code.strip_prefix("[fragment]").unwrap();

		for v in varying_code.split(';') {
			let no_whitespace = v.trim_start();
			if no_whitespace.len() > 0 {
				vertex_header.push_str(&format!("\nlayout(location = {}) out {};", n_varyings, v.trim_start()));
				fragment_header.push_str(&format!("\nlayout(location = {}) in {};", n_varyings, v.trim_start()));
				n_varyings += 1;
			}
		}

		// vertex data
		let mut vertex_desc = VKVertexInputDesc::default();
		add_vertex_layout_data(&mut vertex_desc, &mut vertex_header, &def.vertex_layout, vk::VertexInputRate::VERTEX);
		add_vertex_layout_data(&mut vertex_desc, &mut vertex_header, &def.instance_layout, vk::VertexInputRate::INSTANCE);
		let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
			.vertex_binding_descriptions(&vertex_desc.bindings)
			.vertex_attribute_descriptions(&vertex_desc.attributes);

		// material layouts
		let material_layouts = def.material_layouts.iter().enumerate().map(|(set, layout)| {
			let bindings = layout.attributes.iter().enumerate().map(|(binding, attr)| {
				// shader code
				match &attr.attribute {
					MaterialAttributeID::Texture2D => {
						shared_header.push_str(&format!("\nlayout(set = {}, binding = {}) uniform texture2D {};", set, binding, attr.name));
					},
					MaterialAttributeID::Sampler => {
						shared_header.push_str(&format!("\nlayout(set = {}, binding = {}) uniform sampler {};", set, binding, attr.name));
					},
					MaterialAttributeID::Uniform(uniform_layout) => {
						add_uniform_layout_data(
							&mut shared_header, &mut n_structs,
							&format!("layout(set = {}, binding = {}) uniform", set, binding),
							attr.name,
							&uniform_layout
						);
					},
				}

				// binding
				vk::DescriptorSetLayoutBinding {
					binding: binding as u32,
					descriptor_type: match attr.attribute {
						MaterialAttributeID::Texture2D => vk::DescriptorType::SAMPLED_IMAGE,
						MaterialAttributeID::Sampler => vk::DescriptorType::SAMPLER,
						MaterialAttributeID::Uniform(_) => vk::DescriptorType::UNIFORM_BUFFER,
					},
					descriptor_count: 1,
					stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
					..Default::default()
				}
			}).collect::<Vec<_>>();
			
			let material_layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
			unsafe{device.create_descriptor_set_layout(&material_layout_info, None)}.unwrap()
		}).collect::<Vec<_>>();

		// push constants
		let push_constant_range = vk::PushConstantRange {
			stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT, offset: 0, size: 128
		};
		if !def.push_constant_layout.is_empty() {
			add_uniform_layout_data(&mut shared_header, &mut n_structs, "layout(push_constant) uniform", "push", &def.push_constant_layout);
		}

		// compile shaders
		let mut new_compiler = naga::front::glsl::Frontend::default();
		let mut validator = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::PUSH_CONSTANT);

		let mut vs_code = shared_header.clone();
		vs_code.push_str(&vertex_header);
		vs_code.push_str(&vertex_code);

		let vs_naga_module = new_compiler.parse(&naga::front::glsl::Options::from(naga::ShaderStage::Vertex), &vs_code).map_err(|e| e.emit_to_string("")).unwrap();
		let vs_naga_info = validator.validate(&vs_naga_module).map_err(|e| e.emit_to_string("")).unwrap();
		let vs_spv = naga::back::spv::write_vec(&vs_naga_module, &vs_naga_info, &naga::back::spv::Options {
			..Default::default()
		}, Some(&naga::back::spv::PipelineOptions {
			shader_stage: naga::ShaderStage::Vertex,
			entry_point: String::from("main"),
		})).unwrap();

		let mut fs_code = shared_header.clone();
		fs_code.push_str(&fragment_header);
		fs_code.push_str(&fragment_code);

		let fs_naga_module = new_compiler.parse(&naga::front::glsl::Options::from(naga::ShaderStage::Fragment), &fs_code).map_err(|e| e.emit_to_string("")).unwrap();
		let fs_naga_info = validator.validate(&fs_naga_module).map_err(|e| e.emit_to_string("")).unwrap();
		let fs_spv = naga::back::spv::write_vec(&fs_naga_module, &fs_naga_info, &naga::back::spv::Options {
			..Default::default()
		}, Some(&naga::back::spv::PipelineOptions {
			shader_stage: naga::ShaderStage::Fragment,
			entry_point: String::from("main"),
		})).unwrap();

		// setup modules and stage info
		let vertex_shader_info = vk::ShaderModuleCreateInfo::default().code(&vs_spv);
		let vertex_module = unsafe{device.create_shader_module(&vertex_shader_info, None)}?;
		let fragment_shader_info = vk::ShaderModuleCreateInfo::default().code(&fs_spv);
		let fragment_module = unsafe{device.create_shader_module(&fragment_shader_info, None)}?;
		let stages_info = [
			vk::PipelineShaderStageCreateInfo {
				stage: vk::ShaderStageFlags::VERTEX,
				module: vertex_module,
				p_name: c"main".as_ptr(),
				..Default::default()
			},
			vk::PipelineShaderStageCreateInfo {
				stage: vk::ShaderStageFlags::FRAGMENT,
				module: fragment_module,
				p_name: c"main".as_ptr(),
				..Default::default()
			},
		];
	
		// input assembly
		let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo {
			topology: match def.partial.topology {
				MeshTopology::Triangles => vk::PrimitiveTopology::TRIANGLE_LIST,
			},
			primitive_restart_enable: match def.partial.primitive_restart {
				true => vk::TRUE,
				false => vk::FALSE,
			},
			..Default::default()
		};
	
		// viewport
		let n_viewports = 1;
		let viewport_info = vk::PipelineViewportStateCreateInfo {
			viewport_count: n_viewports,
			scissor_count: n_viewports,
			..Default::default()
		};
	
		// rasterization
		let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
			polygon_mode: vk::PolygonMode::FILL,
			line_width: 1.,
			front_face: vk::FrontFace::COUNTER_CLOCKWISE,
			cull_mode: match def.partial.cull_backfaces {
				true => vk::CullModeFlags::BACK,
				false => vk::CullModeFlags::NONE,
			},
			..Default::default()
		};
	
		// multisampling
		let multisampling_info = vk::PipelineMultisampleStateCreateInfo {
			rasterization_samples: vk::SampleCountFlags::TYPE_1,
			..Default::default()
		};
	
		// depth stencil
		let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo {
			depth_test_enable: match def.partial.depth_test {
				true => vk::TRUE,
				false => vk::FALSE,
			},
			depth_write_enable: match def.partial.depth_write {
				true => vk::TRUE,
				false => vk::FALSE,
			},
			depth_compare_op: match def.partial.depth_always {
				false => vk::CompareOp::LESS,
				true => vk::CompareOp::ALWAYS,
			},
			depth_bounds_test_enable: vk::FALSE,
			stencil_test_enable: vk::FALSE,
			..Default::default()
		};
	
		// color blend
		let color_blend_attachments = [vk::PipelineColorBlendAttachmentState {
			color_write_mask: vk::ColorComponentFlags::RGBA,
			blend_enable: vk::FALSE,
			..Default::default()
		}];
		let color_blend_info = vk::PipelineColorBlendStateCreateInfo::default().attachments(&color_blend_attachments);
	
		// dynamic state
		let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
		let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

		// pipeline layout
		let layout_info = vk::PipelineLayoutCreateInfo::default()
			.set_layouts(&material_layouts)
			.push_constant_ranges(from_ref(&push_constant_range));
		let layout = unsafe{device.create_pipeline_layout(&layout_info, None)}.unwrap();
		
		// final info
		let mut info = vk::GraphicsPipelineCreateInfo {
			p_vertex_input_state: &vertex_input_info,
			p_input_assembly_state: &input_assembly_info,
			p_viewport_state: &viewport_info,
			p_rasterization_state: &rasterization_info,
			p_multisample_state: &multisampling_info,
			p_depth_stencil_state: &depth_stencil_info,
			p_color_blend_state: &color_blend_info,
			p_dynamic_state: &dynamic_state_info,
			layout,
			subpass: 0,
			..Default::default()
		}.stages(&stages_info);

		// variants
		let variants = FramebufferFormat::iter()
			.map(|format| framebuffer_format_to_pipeline_format(format))
			.chain(std::iter::once(surface_format)).map(|format| {
				let rendering_info = vk::PipelineRenderingCreateInfoKHR {
					depth_attachment_format: depth_stencil_format,
					stencil_attachment_format: depth_stencil_format,
					..Default::default()
				}.color_attachment_formats(from_ref(&format));
				info.p_next = (&rendering_info as *const vk::PipelineRenderingCreateInfoKHR) as *const std::ffi::c_void;
				unsafe{device.create_graphics_pipelines(
					vk::PipelineCache::null(), from_ref(&info), None
				)}.unwrap()[0]
			}
		).collect::<Vec<_>>();

		Ok(Self {
			layout, vertex_module, fragment_module, material_layouts, variants,
		})
	}

	pub unsafe fn destroy(&self, device: &Device) {
		for layout in &self.material_layouts {
			device.destroy_descriptor_set_layout(*layout, None);
		}

		for variant in &self.variants {
			device.destroy_pipeline(*variant, None);
		}

		device.destroy_shader_module(self.vertex_module, None);
		device.destroy_shader_module(self.fragment_module, None);
		device.destroy_pipeline_layout(self.layout, None);
	}
}