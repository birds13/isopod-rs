#![allow(private_interfaces)]

use std::{marker::PhantomData, sync::Arc};
use super::*;
use crate::math::*;


/// Indicates that a type is suitable for use as a vertex type in a [`GPUMesh`] or in [`GPUInstances`].
/// 
/// Do not implement this trait manually, instead use the [`VertexTy`](crate::VertexTy) derive macro.

// SAFETY: it must be safe to convert &[T] to &[u8]
pub unsafe trait VertexTy: Default + Clone + Copy {
	#[doc(hidden)]
	fn layout() -> StructLayout<VertexAttributeID>;
	#[doc(hidden)]
	fn into_bytes(slice: &[Self]) -> &[u8] {
		unsafe {
			std::slice::from_raw_parts(std::mem::transmute(slice.as_ptr()), std::mem::size_of::<Self>() * slice.len())
		}
	}
}

unsafe impl VertexTy for () {
	fn layout() -> StructLayout<VertexAttributeID> { StructLayout::unit() }
}

pub trait VertexTyWithPosition {
	fn set_position(&mut self, v: Vec3);
	fn get_position(&self) -> Vec3;
}

pub trait VertexTyWithTexCoord {
	fn set_tex_coord(&mut self, v: Vec2);
	fn get_tex_coord(&self) -> Vec2;
}

pub trait MeshIndexTy: Sized {
	fn extend_u32(vec: &mut Vec<Self>, indices: &[u32]);
}

impl MeshIndexTy for u16 {
	fn extend_u32(vec: &mut Vec<Self>, indices: &[u32]) {
		vec.extend(indices.iter().map(|i| *i as u16));
	}
}

impl MeshIndexTy for u32 {
	fn extend_u32(vec: &mut Vec<Self>, indices: &[u32]) {
		vec.extend_from_slice(indices);
	}
}

/// Represents a mesh with indices on the CPU.
#[derive(Default,Clone)]
pub struct MeshIndexed<T: VertexTy, I: MeshIndexTy> {
	pub vertices: Vec<T>,
	pub indices: Vec<I>,
}

impl<T: VertexTy, I: MeshIndexTy> MeshIndexed<T, I> {
	pub fn new() -> Self {
		Self { vertices: vec![], indices: vec![] }
	}
}

pub type MeshU32<T> = MeshIndexed<T, u32>;
pub type MeshU16<T> = MeshIndexed<T, u16>;

/// CPU representation of a mesh.
/// 
/// Use [`register_mesh`](GfxCtx::register_mesh) or [`imm_mesh`](GfxCtx::imm_mesh) to create a [`GPUMesh`] which can be used for drawing.
#[derive(Clone)]
pub enum Mesh<T: VertexTy> {
	U32(MeshU32<T>),
	U16(MeshU16<T>),
	NoIndices(Vec<T>),
}

pub(crate) struct MeshIndicesBytes {
	pub is_u32: bool,
	pub n: usize,
	pub bytes: Vec<u8>,
}

pub(crate) struct MeshBytes {
	pub vertex_bytes: Vec<u8>,
	pub n_vertices: usize,
	pub indices: Option<MeshIndicesBytes>,
}

impl MeshBytes {
	pub fn from_mesh<T: VertexTy>(data: Mesh<T>) -> Self {
		match data {
			Mesh::U32(data) => Self {
				n_vertices: data.vertices.len(),
				vertex_bytes: T::into_bytes(&data.vertices).to_vec(),
				indices: Some(MeshIndicesBytes {
					n: data.indices.len(),
					bytes: bytemuck::cast_slice(&data.indices).to_vec(),
					is_u32: true,
				})
			},
			Mesh::U16(data) => Self {
				n_vertices: data.vertices.len(),
				vertex_bytes: T::into_bytes(&data.vertices).to_vec(),
				indices: Some(MeshIndicesBytes {
					n: data.indices.len(),
					bytes: bytemuck::cast_slice(&data.indices).to_vec(),
					is_u32: false,
				})
			},
			Mesh::NoIndices(items) => Self {
				n_vertices: items.len(),
				vertex_bytes: T::into_bytes(&items).to_vec(),
				indices: None
			},
		}
	}
}

pub(crate) struct InstanceBytes {
	pub n: usize,
	pub bytes: Vec<u8>,
}

impl InstanceBytes {
	pub fn from_vec<T: VertexTy>(vec: Vec<T>) -> Self {
		Self {
			n: vec.len(),
			bytes: T::into_bytes(&vec).to_vec(),
		}
	}
}

#[derive(Clone, Debug)]
pub(crate) struct ImmediateIndicesDraw {
	pub start: usize,
	pub n: u32,
	pub is_u32: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ImmediateMeshDraw {
	pub start: usize,
	pub n: u32,
	pub indices: Option<ImmediateIndicesDraw>,
}

#[derive(Debug)]
pub(crate) enum MeshDraw {
	Range(std::ops::Range<u32>),
	Immediate(ImmediateMeshDraw),
	Resource(usize),
}

#[derive(Clone)]
pub(crate) enum GPUMeshInner<'a> {
	Range(std::ops::Range<u32>),
	Immediate {
		draw: ImmediateMeshDraw,
		_lifetime: PhantomData<&'a ()>,
	},
	Resource {
		id: usize,
		_rc: Arc<()>,
	},
}

/// Handle to a mesh on the GPU.
/// 
/// Create this using either [`register_mesh`](GfxCtx::register_mesh) or [`imm_mesh`](GfxCtx::imm_mesh).
/// Another handle to this same mesh can created by calling [`clone`](Self::clone) on this.
#[derive(Clone)]
pub struct GPUMesh<'a, T: VertexTy> {
	pub(crate) inner: GPUMeshInner<'a>,
	pub(crate) _data: PhantomData<T>,
}

/// Convenience alias for a [`GPUMesh`] with a static lifetime created using [`register_mesh`](GfxCtx::register_mesh).
pub type GPUMeshRes<T> = GPUMesh<'static, T>;

impl<'a, T: VertexTy> GPUMesh<'a, T> {

	/// Indicates that vertices with no data will be used for drawing.
	/// Their index in a vertex shader can be retrieved using `gl_VertexID`.
	pub fn range(range: std::ops::Range<u32>) -> Self {
		Self { inner: GPUMeshInner::Range(range), _data: PhantomData }
	}

	pub(crate) fn draw(&self) -> MeshDraw {
		match &self.inner {
			GPUMeshInner::Range(range) => MeshDraw::Range(range.clone()),
			GPUMeshInner::Immediate { draw, .. } => MeshDraw::Immediate(draw.clone()),
			GPUMeshInner::Resource { id, .. } => MeshDraw::Resource(*id),
		}
	}
}

#[derive(Clone, Debug)]
pub(crate) struct ImmediateInstancesDraw {
	pub(crate) start: usize,
	pub(crate) n: u32,
}

#[derive(Debug)]
pub(crate) enum InstancesDraw {
	Range(std::ops::Range<u32>),
	Immediate(ImmediateInstancesDraw),
	Resource(usize),
}

#[derive(Clone)]
pub(crate) enum GPUInstancesInner<'a> {
	Range(std::ops::Range<u32>),
	Immediate {
		draw: ImmediateInstancesDraw,
		_lifetime: PhantomData<&'a ()>,
	},
	Resource {
		id: usize,
		_rc: Arc<()>,
	},
}

/// Handle to a set on instances on the GPU.
/// 
/// Create this using either [`register_instances`](GfxCtx::register_instances) or [`imm_instances`](GfxCtx::imm_instances).
/// Another handle to the same instances can created by calling [`clone`](Self::clone) on this.
#[derive(Clone)]
pub struct GPUInstances<'a, T: VertexTy> {
	pub(crate) inner: GPUInstancesInner<'a>,
	pub(crate) _data: PhantomData<T>,
}

/// Convenience alias for [`GPUInstances`] with a static lifetime created using [`register_instances`](GfxCtx::register_instances).
pub type GPUInstancesRes<T> = GPUInstances<'static, T>;

impl<'a, T: VertexTy> GPUInstances<'a, T> {

	/// Used for indicating that only one instance of the given mesh will drawn.
	pub fn one() -> Self {
		Self { inner: GPUInstancesInner::Range(0..1), _data: PhantomData }
	}

	/// Indicates that instances with no additional data will be used for drawing.
	/// Their index in a vertex shader can be retrieved using `gl_InstanceID`.
	pub fn range(range: std::ops::Range<u32>) -> Self {
		Self { inner: GPUInstancesInner::Range(range), _data: PhantomData }
	}

	pub(crate) fn draw(&self) -> InstancesDraw {
		match &self.inner {
			GPUInstancesInner::Range(range) => InstancesDraw::Range(range.clone()),
			GPUInstancesInner::Immediate { draw, .. } =>  InstancesDraw::Immediate(draw.clone()),
			GPUInstancesInner::Resource { id, .. } =>  InstancesDraw::Resource(*id),
		}
	}
}