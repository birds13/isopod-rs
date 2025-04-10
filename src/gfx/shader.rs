use std::{marker::PhantomData, sync::Arc};

use super::*;

#[derive(Default, Clone, Copy)]
pub enum MeshTopology {
	#[default]
	Triangles,
}

#[derive(Clone, Copy)]
pub enum ColorBlend {
	Alpha,
}

#[derive(Default, Clone)]
pub struct ShaderDefinition {
	pub code: String,
	pub topology: MeshTopology,
	pub cull_backfaces: bool,
	pub primitive_restart: bool,
	pub depth_test: bool,
	pub depth_write: bool,
	pub depth_always: bool,
	pub color_blend: Option<ColorBlend>,
}

#[derive(Clone)]
pub(crate) struct ShaderFullDefinition {
	pub vertex_layout: StructLayout<VertexAttributeID>,
	pub instance_layout: StructLayout<VertexAttributeID>,
	pub push_constant_layout: StructLayout<UniformAttributeID>,
	pub material_layouts: Vec<StructLayout<MaterialAttributeID>>,
	pub partial: ShaderDefinition,
}

impl ShaderFullDefinition {
	pub fn from_partial<
		Vertex: VertexTy, Instance: VertexTy, Materials: MaterialSet, Push: UniformTy,
	>(def: ShaderDefinition) -> Self {
		Self {
			vertex_layout: Vertex::layout(),
			instance_layout: Instance::layout(),
			material_layouts: Materials::layouts(),
			push_constant_layout: Push::layout(),
			partial: def,
		}
	}
}

#[derive(Clone)]
pub struct Shader<
	Vertex: VertexTy, Instance: VertexTy, Materials: MaterialSet, Push: UniformTy,
> {
	pub(crate) id: usize,
	pub(crate) _rc: Arc<ShaderFullDefinition>,
	pub(crate) _data: PhantomData<(Vertex, Instance, Materials, Push)>,
}