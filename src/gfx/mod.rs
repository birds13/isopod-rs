use std::{cell::Cell, marker::PhantomData, sync::Arc};

pub mod util;
mod attribute;
mod backend;
mod shader;
mod mesh;
mod gpu_texture;
mod uniform;
mod resource;
mod draw;
mod material;
mod texture;

use crate::util::*;

use glam::UVec2;


pub use shader::*;
pub use draw::*;
pub use gpu_texture::*;
pub use material::*;
pub use uniform::*;
pub use attribute::*;
pub use mesh::*;
pub use texture::*;
use resource::*;

#[derive(Default)]
pub(crate) struct GfxFrameData {
	pub resource_update_queue: BufferCell<resource::ResourceUpdate>,
	pub draw_cmd_queue: BufferCell<draw::DrawCmd>,
	pub vertices: ByteBufferCell,
	pub indices: ByteBufferCell,
	pub uniforms: ByteBufferCell,
	pub current_material_ids: [Cell<usize>; MAX_MATERIALS],
	pub current_pipeline: Cell<usize>,
	pub next_id: Cell<usize>,
}

impl GfxFrameData {
	fn reset(&mut self) {
		self.resource_update_queue.get_mut().clear();
		self.draw_cmd_queue.get_mut().clear();
		self.vertices.get_mut().clear();
		self.indices.get_mut().clear();
		self.uniforms.get_mut().clear();
		self.next_id.set(0);
		for v in &self.current_material_ids {
			v.set(ID_NONE);
		}
		self.current_pipeline.set(ID_NONE);
	}
}

#[derive(Default)]
struct GfxResources {
	shaders: IDArenaCell<ShaderFullDefinition>,
	texture2ds: IDArenaCell<Texture2DMeta>,
	samplers: IDArenaCell<SamplerDefinition>,
	meshes: IDArenaCell<()>,
	instances: IDArenaCell<()>,
	uniforms: IDArenaCell<()>,
	framebuffers: IDArenaCell<Texture2DMeta>,
}

/// Provides access to graphics features of the engine.
/// 
/// Many functions have two versions starting with `register_` and `imm_`.
/// Functions starting with `imm_` are designed to support immediate-mode rendering, in which data for a frame is created from scratch each time.
/// Use the normal `register_` versions instead if you arn't doing this with the particular resource.
pub struct GfxCtx {
	pub window_canvas: Canvas,
	pub(crate) frame_data: GfxFrameData,
	resources: GfxResources,
}

const IMMEDIATE_ALIGN: usize = 64;

impl GfxCtx {
	pub(crate) fn new() -> Self {
		Self {
			window_canvas: Canvas { id: CanvasID::Screen, size: UVec2::ONE },
			resources: GfxResources::default(),
			frame_data: GfxFrameData::default(),
		}
	}

	// Gets a unique id for this frame.
	fn unique_id(&self) -> usize {
		let id = self.frame_data.next_id.get();
		self.frame_data.next_id.set(id+1);
		id
	}
	
	/// Creates a [`Shader`] on the GPU.
	pub fn register_shader<
		Vertex: VertexTy, Instance: VertexTy, Materials: MaterialSet, Push: UniformTy,
	>(&self, def: ShaderDefinition) -> Shader<Vertex, Instance, Materials, Push> {
		let def = shader::ShaderFullDefinition::from_partial::<Vertex, Instance, Materials, Push>(def);
		let rc = Arc::new(def.clone());
		let id = self.resources.shaders.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateShader { id, def });
		Shader { id, _rc: rc, _data: PhantomData }
	}

	fn register_texture2d_internal(&self, bytes: Vec<u8>, size: UVec2, format: TextureFormatID) -> GPUTexture2D {
		let meta = Texture2DMeta { size, format };
		let rc = Arc::new(meta.clone());
		let id = self.resources.texture2ds.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateTexture2D { id, meta, bytes });
		GPUTexture2D { ty: Texture2DTy::Texture2D, id, rc }
	}

	/// Creates a [`GPUTexture2D`] on the GPU.
	/// 
	/// If the input data is 3D, anything past the first layer will be ignored.
	pub fn register_texture2d<T: TextureFormat>(&self, mut data: Texture<T>) -> GPUTexture2D {
		data.pixels.truncate(data.area() as usize);
		self.register_texture2d_internal(bytemuck::cast_slice(&data.pixels).to_vec(), data.size_2d(), T::TEXTURE_ID)
	}

	/// Creates a [`GPUTexture2D`] in SRGB format on the GPU.
	/// 
	/// SRGB textures will have their data converted from SRGB colorspace to linear when read in a shader.
	/// If the input data is 3D, anything past the first layer will be ignored.
	pub fn register_texture2d_srgb<T: SrgbTextureFormat>(&self, mut data: Texture<T>) -> GPUTexture2D {
		data.pixels.truncate(data.area() as usize);
		self.register_texture2d_internal(bytemuck::cast_slice(&data.pixels).to_vec(), data.size_2d(), T::SRGB_ID)
	}

	/// Makes a [`Mesh`] accessible for drawing.
	/// 
	/// Consider using [`imm_mesh`](GfxCtx::imm_mesh) instead if the mesh will only be used this frame.
	pub fn register_mesh<T: VertexTy + 'static>(&self, mesh: Mesh<T>) -> GPUMesh<'static, T> {
		let rc = Arc::new(());
		let id = self.resources.meshes.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateMesh { id, data: MeshBytes::from_mesh(mesh) });
		GPUMesh { inner: GPUMeshInner::Resource { id, _rc: rc }, _data: PhantomData }
	}

	/// Registers a [`Mesh`] for drawing for this frame only.
	/// 
	/// This is more effiencent for single use meshes but use [`register_mesh`](GfxCtx::register_mesh) instead for meshes that will persist across multiple frames.
	pub fn imm_mesh<'frame, T: VertexTy>(&'frame self, mesh: Mesh<T>) -> GPUMesh<'frame, T> {
		let start = self.frame_data.vertices.len();
		let vertices = match &mesh {
			Mesh::U32(data) => &data.vertices,
			Mesh::U16(data) => &data.vertices,
			Mesh::NoIndices(items) => &items,
		};
		self.frame_data.vertices.push_bytes(T::into_bytes(vertices));
		self.frame_data.vertices.align_to(IMMEDIATE_ALIGN);
		GPUMesh { inner: GPUMeshInner::Immediate { draw: ImmediateMeshDraw {
			start, n: vertices.len() as u32,
			indices: match &mesh {
				Mesh::U32(data) => {
					let start = self.frame_data.indices.len();
					self.frame_data.indices.push_bytes(bytemuck::cast_slice(&data.indices));
					self.frame_data.indices.align_to(IMMEDIATE_ALIGN);
					Some(mesh::ImmediateIndicesDraw { start, n: data.indices.len() as u32, is_u32: true })
				},
				Mesh::U16(data) => {
					let start = self.frame_data.indices.len();
					self.frame_data.indices.push_bytes(bytemuck::cast_slice(&data.indices));
					self.frame_data.indices.align_to(IMMEDIATE_ALIGN);
					Some(mesh::ImmediateIndicesDraw { start, n: data.indices.len() as u32, is_u32: false })
				},
				Mesh::NoIndices(_) => None,
			},
		}, _lifetime: PhantomData }, _data: PhantomData }
	}

	/// Makes a set of instances accesible for drawing with.
	/// 
	/// Consider using [`imm_instances`](GfxCtx::imm_instances) instead if the instances will only be used this frame.
	pub fn register_instances<T: VertexTy + 'static>(&self, instances: Vec<T>) -> GPUInstances<'static, T> {
		let rc = Arc::new(());
		let id = self.resources.instances.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateInstances { id, data: InstanceBytes::from_vec(instances) });
		GPUInstances { inner: GPUInstancesInner::Resource { id, _rc: rc }, _data: PhantomData }
	}

	/// Registers a set of instances for drawing with for this frame only.
	/// 
	/// This is more effiencent for instances that will be used only once but use [`register_instances`](GfxCtx::register_instances) instead for instances that will be used across multiple frames.
	pub fn imm_instances<'frame, T: VertexTy>(&'frame self, instances: Vec<T>) -> GPUInstances<'frame, T> {
		let start = self.frame_data.vertices.len();
		self.frame_data.vertices.push_bytes(T::into_bytes(&instances));
		self.frame_data.vertices.align_to(IMMEDIATE_ALIGN);
		GPUInstances {
			inner: GPUInstancesInner::Immediate { draw: mesh::ImmediateInstancesDraw { start, n: instances.len() as u32 }, _lifetime: PhantomData },
			_data: PhantomData,
		}
	}

	/// Creates a uniform buffer for use in shaders on the GPU.
	/// 
	/// Consider using [`imm_uniform_buffer`](GfxCtx::imm_uniform_buffer) if the data will only be used this frame.
	pub fn register_uniform_buffer<T: UniformTy>(&self, content: T) -> UniformBuffer<'static, T> {
		let rc = Arc::new(());
		let id = self.resources.uniforms.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateUniform { id, data: content.into_bytes().to_vec() });
		UniformBuffer { inner: UniformBufferInner::Resource { id, _rc: rc }, _data: PhantomData }
	}

	/// Creates a uniform buffer for shader use for this frame only.
	/// 
	/// This is more effiencent if it will only be used once but use [`register_uniform_buffer`](GfxCtx::register_uniform_buffer) instead if the data will be used across multiple frames.
	pub fn imm_uniform_buffer<'frame, T: UniformTy>(&'frame self, uniform: T) -> UniformBuffer<'frame, T> {
		let start = self.frame_data.uniforms.len();
		self.frame_data.uniforms.push_bytes(uniform.into_bytes());
		self.frame_data.uniforms.align_to(IMMEDIATE_ALIGN);
		UniformBuffer { inner: UniformBufferInner::Immediate { start, len: std::mem::size_of::<T>(), _lifetime: PhantomData }, _data: PhantomData }
	}

	/// Creates a [`MaterialCfg`] that can be used with a [`ShaderCfg`] to draw this frame.
	pub fn material_cfg<'frame, T: MaterialTy>(&'frame self, refs: T::Refs<'frame>) -> MaterialCfg<'frame, T> {
		let ref_ids = refs.into_refs();
		let id = self.unique_id();
		MaterialCfg { raw: MaterialRaw { inner: MaterialInner { id, attributes: ref_ids, _data: PhantomData }}, _data: PhantomData }
	}

	/// Creates a [`Framebuffer`] on the GPU.
	pub fn register_framebuffer(&self, size: UVec2, format: TextureFormatID) -> Framebuffer {
		let meta = Texture2DMeta { size, format };
		let rc = Arc::new(meta.clone());
		let id = self.resources.framebuffers.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateFramebuffer { id, meta });
		Framebuffer::new(id, rc)
	}

	/// Creates a [`Sampler`] on the GPU.
	pub fn register_sampler(&self, def: SamplerDefinition) -> Sampler {
		let rc = Arc::new(def.clone());
		let id = self.resources.samplers.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateSampler { id, def });
		Sampler { id, _rc: rc }
	}

	/// Creates a [`ShaderCfg`] that can be used for drawing this frame.
	pub fn shader_cfg<'frame, Vertex: VertexTy, Instance: VertexTy, Materials: MaterialSet, Push: UniformTy>(
		&'frame self, shader: &Shader<Vertex, Instance, Materials, Push>, material_cfgs: Materials::Materials<'frame>,
	) -> ShaderCfg<'frame, Vertex, Instance, Materials, Push> {
		ShaderCfg { ctx: self, shader: shader.id, materials: material_cfgs, _data: PhantomData }
	}

	/// Sets the current target that will be rendered to.
	pub fn set_canvas(&self, canvas: &Canvas, clear_color: Option<glam::Vec4>) {
		self.frame_data.draw_cmd_queue.push(draw::DrawCmd::SetCanvas { id: canvas.id, clear_color });
		self.frame_data.current_pipeline.set(ID_NONE);
	}
}

pub(crate) struct GfxSys {
	window: sdl2::video::Window,
	backend: Box<dyn backend::GfxBackend>,
}

impl GfxSys {
	pub fn start_update(&mut self, c: &mut GfxCtx, reset_frame_data: bool) {
		c.window_canvas.size = self.window.size().into();
		if reset_frame_data {
			c.frame_data.reset();
		}

		for id in c.resources.framebuffers.remove_unused() {
			c.frame_data.resource_update_queue.push(ResourceUpdate::Free{id, ty: ResourceFreeType::Framebuffer });
		}
		for id in c.resources.instances.remove_unused() {
			c.frame_data.resource_update_queue.push(ResourceUpdate::Free{id, ty: ResourceFreeType::Instances });
		}
		for id in c.resources.meshes.remove_unused() {
			c.frame_data.resource_update_queue.push(ResourceUpdate::Free{id, ty: ResourceFreeType::Mesh });
		}
		for id in c.resources.samplers.remove_unused() {
			c.frame_data.resource_update_queue.push(ResourceUpdate::Free{id, ty: ResourceFreeType::Sampler });
		}
		for id in c.resources.shaders.remove_unused() {
			c.frame_data.resource_update_queue.push(ResourceUpdate::Free{id, ty: ResourceFreeType::Shader });
		}
		for id in c.resources.texture2ds.remove_unused() {
			c.frame_data.resource_update_queue.push(ResourceUpdate::Free{id, ty: ResourceFreeType::Texture2D });
		}
		for id in c.resources.uniforms.remove_unused() {
			c.frame_data.resource_update_queue.push(ResourceUpdate::Free{id, ty: ResourceFreeType::Uniform });
		}
	}

	pub fn render(&mut self, c: &mut GfxCtx) {
		self.backend.render(c);
	}

	pub fn new(sdl_video: &sdl2::VideoSubsystem) -> Self {
		let mut window_builder = sdl2::video::WindowBuilder::new(&sdl_video, "", 800, 800);
		window_builder.position_centered();
		window_builder.resizable();
		let (backend, window) = backend::VulkanGfxBackend::load(sdl_video, window_builder).unwrap();
		Self {
			backend: Box::new(backend), window
		}
	}
}