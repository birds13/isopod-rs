use std::{cell::Cell, marker::PhantomData, sync::Arc};

pub mod util;
mod attribute;
mod backend;
mod shader;
mod mesh;
mod texture;
mod uniform;
mod resource;
mod draw;
mod material;
mod texture_data;

use crate::util::*;

use glam::UVec2;


pub use shader::*;
pub use draw::*;
pub use texture::*;
pub use material::*;
pub use uniform::*;
pub use attribute::*;
pub use mesh::*;
pub use texture_data::*;
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
	shaders: IDArenaCell<shader::ShaderFullDefinition>,
	texture2ds: IDArenaCell<()>,
	samplers: IDArenaCell<SamplerDefinition>,
	meshes: IDArenaCell<()>,
	instances: IDArenaCell<()>,
	uniforms: IDArenaCell<()>,
	framebuffers: IDArenaCell<FramebufferDefinition>,
}

#[derive(Default)]
pub struct GfxCtx {
	pub window_size: glam::Vec2,
	pub backend_debug_info: String,
	pub(crate) frame_data: GfxFrameData,
	resources: GfxResources,
}

const IMMEDIATE_ALIGN: usize = 64;

impl GfxCtx {
	pub(crate) fn new() -> Self {
		Self::default()
	}
	
	/// Creates a [Shader] on the GPU.
	pub fn create_shader<
		Vertex: VertexTy, Instance: VertexTy, Materials: MaterialSet, Push: UniformTy,
	>(&self, def: ShaderDefinition) -> Shader<Vertex, Instance, Materials, Push> {
		let def = shader::ShaderFullDefinition::from_partial::<Vertex, Instance, Materials, Push>(def);
		let rc = Arc::new(def.clone());
		let id = self.resources.shaders.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateShader { id, def });
		Shader { id, _rc: rc, _data: PhantomData }
	}

	/// Creates a [Texture2D] on the GPU.
	/// 
	/// If the input data is 3D, anything past the first layer will be ignored.
	pub fn create_texture2d<T: TextureAttribute>(&self, data: TextureData<T>) -> Texture2D {
		let data = data.into_bytes();
		let size = UVec2::new(data.size.x, data.size.y);
		let attribute = data.attribute;
		let rc = Arc::new(());
		let id = self.resources.texture2ds.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateTexture2D { id, data });
		Texture2D { id, size, attribute, _rc: rc }
	}

	/// Creates a [Mesh] on the GPU.
	pub fn create_mesh<T: VertexTy + 'static>(&self, mesh: MeshData<T>) -> Mesh<T> {
		let rc = Arc::new(());
		let id = self.resources.meshes.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateMesh { id, data: MeshDataBytes::from_data(mesh) });
		Mesh { id, _rc: rc, _data: PhantomData }
	}

	/// Creates a buffer of [Instances] on the GPU.
	pub fn create_instances<T: VertexTy + 'static>(&self, instances: Vec<T>) -> Instances<T> {
		let rc = Arc::new(());
		let id = self.resources.instances.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateInstances { id, data: InstanceDataBytes::from_vec(instances) });
		Instances { id, _rc: rc, _data: PhantomData }
	}

	/// Creates a [UniformBuffer] on the GPU.
	pub fn create_uniform_buffer<T: UniformTy>(&self, uniform: T) -> UniformBuffer<T> {
		let rc = Arc::new(());
		let id = self.resources.uniforms.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateUniform { id, data: uniform.into_bytes().to_vec() });
		UniformBuffer { id, _rc: rc, _data: PhantomData }
	}

	/// Creates a [Framebuffer] on the GPU.
	pub fn create_framebuffer(&self, def: FramebufferDefinition) -> Framebuffer {
		let size = def.size;
		let format = def.format;
		let rc = Arc::new(def.clone());
		let id = self.resources.framebuffers.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateFramebuffer { id, def });
		Framebuffer { id, size, format, _rc: rc }
	}

	/// Creates a [Sampler] on the GPU.
	pub fn create_sampler(&self, def: SamplerDefinition) -> Sampler {
		let rc = Arc::new(def.clone());
		let id = self.resources.samplers.insert(&rc);
		self.frame_data.resource_update_queue.push(ResourceUpdate::CreateSampler { id, def });
		Sampler { id, _rc: rc }
	}

	/// Creates an [ImmediateMesh] suitable for per-frame meshes.
	pub fn imm_mesh<'frame, T: VertexTy>(&'frame self, mesh: MeshData<T>) -> ImmediateMesh<'frame, T> {
		let start = self.frame_data.vertices.len();
		self.frame_data.vertices.push_bytes(T::into_bytes(&mesh.vertices));
		self.frame_data.vertices.align_to(IMMEDIATE_ALIGN);
		ImmediateMesh { _data: PhantomData, draw: mesh::ImmediateMeshDraw {
			start, n: mesh.vertices.len() as u32,
			indices: mesh.indices.map(|data| match data {
				MeshIndexData::U16(items) => {
					let start = self.frame_data.indices.len();
					self.frame_data.indices.push_bytes(bytemuck::cast_slice(&items));
					self.frame_data.indices.align_to(IMMEDIATE_ALIGN);
					mesh::ImmediateIndicesDraw { start, n: items.len() as u32, is_u32: false }
				},
				MeshIndexData::U32(items) => {
					let start = self.frame_data.indices.len();
					self.frame_data.indices.push_bytes(bytemuck::cast_slice(&items));
					self.frame_data.indices.align_to(IMMEDIATE_ALIGN);
					mesh::ImmediateIndicesDraw { start, n: items.len() as u32, is_u32: true }
				},
			})
		}}
	}

	/// Creates [ImmediateInstances] suitable for per-frame instances.
	pub fn imm_instances<'frame, T: VertexTy>(&'frame self, instances: Vec<T>) -> ImmediateInstances<'frame, T> {
		let start = self.frame_data.vertices.len();
		self.frame_data.vertices.push_bytes(T::into_bytes(&instances));
		self.frame_data.vertices.align_to(IMMEDIATE_ALIGN);
		ImmediateInstances { draw: mesh::ImmediateInstancesDraw { start, n: instances.len() as u32 }, _data: PhantomData }
	}

	/// Creates an [ImmediateUniformBuffer] suitable for per-frame uniforms.
	pub fn imm_uniform_buffer<'frame, T: UniformTy>(&'frame self, uniform: T) -> ImmediateUniformBuffer<'frame, T> {
		let start = self.frame_data.uniforms.len();
		self.frame_data.uniforms.push_bytes(uniform.into_bytes());
		self.frame_data.uniforms.align_to(IMMEDIATE_ALIGN);
		ImmediateUniformBuffer { start, len: std::mem::size_of::<T>(), _data: PhantomData }
	}

	/// Creates a [ShaderCfg] that can be used for drawing this frame.
	pub fn shader_cfg<'frame, Vertex: VertexTy, Instance: VertexTy, Materials: MaterialSet, Push: UniformTy>(
		&'frame self, shader: &Shader<Vertex, Instance, Materials, Push>, materials: Materials::Cfgs<'frame>,
	) -> ShaderCfg<'frame, Vertex, Instance, Materials, Push> {
		ShaderCfg { ctx: self, shader: shader.id, materials, _data: PhantomData }
	}

	/// Sets the current target for rendering.
	pub fn set_canvas<C: Canvas>(&self, canvas: &C, clear_color: Option<glam::Vec4>) {
		self.frame_data.draw_cmd_queue.push(draw::DrawCmd::SetCanvas { id: canvas.id(), clear_color });
		self.frame_data.current_pipeline.set(ID_NONE);
	}

	/// Gets a unique id for the current frame.
	pub fn unique_id(&self) -> usize {
		let id = self.frame_data.next_id.get();
		self.frame_data.next_id.set(id+1);
		id
	}
}

pub(crate) struct GfxSys {
	window: sdl2::video::Window,
	backend: Box<dyn backend::GfxBackend>,
}

impl GfxSys {
	pub fn start_update(&mut self, c: &mut GfxCtx, reset_frame_data: bool) {
		let window_size = self.window.size();
		c.window_size = glam::Vec2::new(window_size.0 as f32, window_size.1 as f32);
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