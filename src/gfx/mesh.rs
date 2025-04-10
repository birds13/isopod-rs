#![allow(private_interfaces)]

use std::{marker::PhantomData, num::NonZero, sync::Arc};
use super::*;
use crate::math::*;

// SAFETY: it must be safe to convert &[T] to &[u8]
pub unsafe trait VertexTy: Default + Clone + Copy {
	#[doc(hidden)]
	fn layout() -> StructLayout<VertexAttributeID>;
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

#[derive(Default,Clone)]
pub struct MeshDataU32<T: VertexTy> {
	pub vertices: Vec<T>,
	pub indices: Vec<u32>,
}

impl<T: VertexTy> MeshDataU32<T> {
	pub fn new() -> Self {
		Self { vertices: vec![], indices: vec![] }
	}
}

#[derive(Default,Clone)]
pub struct MeshDataU16<T: VertexTy> {
	pub vertices: Vec<T>,
	pub indices: Vec<u16>,
}

impl<T: VertexTy> MeshDataU16<T> {
	pub fn new() -> Self {
		Self { vertices: vec![], indices: vec![] }
	}
}

#[derive(Clone)]
pub enum MeshData<T: VertexTy> {
	U32(MeshDataU32<T>),
	U16(MeshDataU16<T>),
	NoIndices(Vec<T>),
}

pub(crate) struct MeshIndexDataBytes {
	pub is_u32: bool,
	pub n: usize,
	pub bytes: Vec<u8>,
}

pub(crate) struct MeshDataBytes {
	pub vertex_bytes: Vec<u8>,
	pub n_vertices: usize,
	pub indices: Option<MeshIndexDataBytes>,
}

impl MeshDataBytes {
	pub fn from_data<T: VertexTy>(data: MeshData<T>) -> Self {
		match data {
			MeshData::U32(data) => Self {
				n_vertices: data.vertices.len(),
				vertex_bytes: T::into_bytes(&data.vertices).to_vec(),
				indices: Some(MeshIndexDataBytes {
					n: data.indices.len(),
					bytes: bytemuck::cast_slice(&data.indices).to_vec(),
					is_u32: true,
				})
			},
			MeshData::U16(data) => Self {
				n_vertices: data.vertices.len(),
				vertex_bytes: T::into_bytes(&data.vertices).to_vec(),
				indices: Some(MeshIndexDataBytes {
					n: data.indices.len(),
					bytes: bytemuck::cast_slice(&data.indices).to_vec(),
					is_u32: false,
				})
			},
			MeshData::NoIndices(items) => Self {
				n_vertices: items.len(),
				vertex_bytes: T::into_bytes(&items).to_vec(),
				indices: None
			},
		}
	}
}

pub(crate) struct InstanceDataBytes {
	pub n: usize,
	pub bytes: Vec<u8>,
}

impl InstanceDataBytes {
	pub fn from_vec<T: VertexTy>(vec: Vec<T>) -> Self {
		Self {
			n: vec.len(),
			bytes: T::into_bytes(&vec).to_vec(),
		}
	}
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub(crate) struct ImmediateIndicesDraw {
	pub start: usize,
	pub n: u32,
	pub is_u32: bool,
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ImmediateMeshDraw {
	pub(crate) start: usize,
	pub(crate) n: u32,
	pub(crate) indices: Option<ImmediateIndicesDraw>,
}

#[doc(hidden)]
#[derive(Debug)]
pub enum MeshDraw {
	Range(std::ops::Range<u32>),
	Immediate(ImmediateMeshDraw),
	Resource(usize),
}

pub trait MeshAny<T: VertexTy> {
	#[doc(hidden)]
	fn draw(&self) -> MeshDraw;
}

impl MeshAny<()> for () {
	fn draw(&self) -> MeshDraw {
		MeshDraw::Range(0..1)
	}
}

impl MeshAny<()> for std::ops::Range<u32> {
	fn draw(&self) -> MeshDraw {
		MeshDraw::Range(self.clone())
	}
}

pub struct ImmediateMesh<'frame, T: VertexTy> {
	pub(crate) draw: ImmediateMeshDraw,
	pub(crate) _data: PhantomData<(&'frame (), T)>,
}

impl<'frame, T: VertexTy> MeshAny<T> for ImmediateMesh<'frame, T> {
	#[doc(hidden)]
	fn draw(&self) -> MeshDraw {
		MeshDraw::Immediate(self.draw.clone())
	}
}

pub struct Mesh<T: VertexTy> {
	pub(crate) id: usize,
	pub(crate) _rc: Arc<()>,
	pub(crate) _data: PhantomData<T>,
}

impl<T: VertexTy> MeshAny<T> for Mesh<T> {
	fn draw(&self) -> MeshDraw {
		MeshDraw::Resource(self.id)
	}
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ImmediateInstancesDraw {
	pub(crate) start: usize,
	pub(crate) n: u32,
}

#[doc(hidden)]
#[derive(Debug)]
pub enum InstancesDraw {
	Range(std::ops::Range<u32>),
	Immediate(ImmediateInstancesDraw),
	Resource(usize),
}

pub trait InstancesAny<T: VertexTy> {
	#[doc(hidden)]
	fn draw(&self) -> InstancesDraw;
}

impl InstancesAny<()> for () {
	fn draw(&self) -> InstancesDraw {
		InstancesDraw::Range(0..1)
	}
}

impl InstancesAny<()> for std::ops::Range<u32> {
	fn draw(&self) -> InstancesDraw {
		InstancesDraw::Range(self.clone())
	}
}

pub struct ImmediateInstances<'frame, T: VertexTy> {
	pub(crate) draw: ImmediateInstancesDraw,
	pub(crate) _data: PhantomData<(&'frame (), T)>,
}

impl<'frame, T: VertexTy> InstancesAny<T> for ImmediateInstances<'frame, T> {
	fn draw(&self) -> InstancesDraw {
		InstancesDraw::Immediate(self.draw.clone())
	}
}

pub struct Instances<T: VertexTy> {
	pub(crate) id: usize,
	pub(crate) _rc: Arc<()>,
	pub(crate) _data: PhantomData<T>,
}

impl<T: VertexTy> InstancesAny<T> for Instances<T> {
	fn draw(&self) -> InstancesDraw {
		InstancesDraw::Resource(self.id)
	}
}