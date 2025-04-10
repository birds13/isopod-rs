#![allow(private_interfaces)]

use std::{marker::PhantomData, sync::Arc};
use super::*;
use crate::math::*;

pub trait VertexTy: bytemuck::NoUninit + Default {
	#[doc(hidden)]
	fn layout() -> StructLayout<VertexAttributeID>;
}

impl VertexTy for () {
	fn layout() -> StructLayout<VertexAttributeID> { StructLayout::unit() }
}

pub trait VertexTyWithPosition2D<const SLOT: usize> {
	fn set_position_2d(&mut self, v: Vec2);
	fn get_position_2d(&self) -> Vec2;
}

pub trait VertexTyWithPosition4D<const SLOT: usize> {
	fn set_position_4d(&mut self, v: Vec4);
	fn get_position_4d(&self) -> Vec4;
}

pub trait VertexTyWithTexCoord<const SLOT: usize> {
	fn set_tex_coord(&mut self, v: Vec2);
	fn get_tex_coord(&self) -> Vec2;
}

pub enum MeshIndexData {
	U16(Vec<u16>),
	U32(Vec<u32>),
}

pub struct MeshData<T: VertexTy> {
	pub vertices: Vec<T>,
	pub indices: Option<MeshIndexData>,
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
		Self {
			n_vertices: data.vertices.len(),
			vertex_bytes: bytemuck::cast_slice(&data.vertices).to_vec(),
			indices: data.indices.map(|data| match data {
				MeshIndexData::U16(items) => MeshIndexDataBytes {
					n: items.len(),
					bytes: bytemuck::cast_slice(&items).to_vec(),
					is_u32: false,
				},
				MeshIndexData::U32(items) => MeshIndexDataBytes {
					n: items.len(),
					bytes: bytemuck::cast_slice(&items).to_vec(),
					is_u32: true,
				},
			}),
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
			bytes: bytemuck::cast_slice(&vec).to_vec(),
		}
	}
}

#[doc(hidden)]
#[derive(Clone)]
pub(crate) struct ImmediateIndicesDraw {
	pub start: usize,
	pub n: u32,
	pub is_u32: bool,
}

#[doc(hidden)]
#[derive(Clone)]
pub struct ImmediateMeshDraw {
	pub(crate) start: usize,
	pub(crate) n: u32,
	pub(crate) indices: Option<ImmediateIndicesDraw>,
}

#[doc(hidden)]
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
#[derive(Clone)]
pub struct ImmediateInstancesDraw {
	pub(crate) start: usize,
	pub(crate) n: u32,
}

#[doc(hidden)]
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