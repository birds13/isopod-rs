

use super::*;
use crate::math::*;

pub struct MeshBuilder<T: VertexTyWithPosition + VertexTy> {
	vertices: Vec<T>,
	indices: Vec<u16>,
}

impl<T: VertexTyWithPosition + VertexTy> MeshBuilder<T> {
	pub fn new() -> Self {
		Self {
			vertices: vec![],
			indices: vec![],
		}
	}

	pub fn regular_polygon(&mut self, sides: u16, center: Vec3, radius: f32, rotation: f32, outer_data: T, inner_data: T) {
		let v_start = self.vertices.len() as u16;
		let mut inside_vertex = inner_data.clone();
		inside_vertex.set_position(center);
		self.vertices.push(inside_vertex);
		let sides_inv = 1.0 / sides as f32;
		for i in 0..sides {
			let r = (i as f32 * sides_inv + rotation) * std::f32::consts::PI * 2.0;
			let mut vertex = outer_data.clone();
			vertex.set_position(vec3(f32::cos(r) * radius, f32::sin(r) * radius, 0.) + center);
			self.vertices.push(vertex);
		}
		self.indices.extend_from_slice(&[v_start, v_start + 1, v_start + sides]);
		for i in 1..sides {
			self.indices.extend_from_slice(&[v_start, v_start + i, v_start + i + 1]);
		}
	}

	pub fn star(&mut self, points: u16, center: Vec3, inner_radius: f32, outer_radius: f32, rotation: f32, outer_data: T, inner_data: T) {
		let v_start = self.vertices.len() as u16;
		let mut inside_vertex = inner_data.clone();
		inside_vertex.set_position(center);
		self.vertices.push(inside_vertex);
		let sides_inv = 1.0 / (points * 2) as f32;
		for i in 0..points*2 {
			let r = (i as f32 * sides_inv + rotation) * std::f32::consts::PI * 2.0;
			let radius = if i % 2 == 0 { inner_radius } else { outer_radius };
			let mut vertex = outer_data.clone();
			vertex.set_position(vec3(f32::cos(r) * radius, f32::sin(r) * radius, 0.) + center);
			self.vertices.push(vertex);
		}
		self.indices.extend_from_slice(&[v_start, v_start + 1, v_start + points * 2]);
		for i in 1..points * 2 {
			self.indices.extend_from_slice(&[v_start, v_start + i, v_start + i + 1]);
		}
	}

	pub fn rect(&mut self, rect: Rect2D, z: f32, data: T) {
		let v = self.vertices.len() as u16;

		let mut v0 = data.clone();
		v0.set_position(vec3(rect.start.x,rect.start.y,z));
		self.vertices.push(v0);

		let mut v1 = data.clone();
		v1.set_position(vec3(rect.start.x,rect.end.y,z));
		self.vertices.push(v1);

		let mut v2 = data.clone();
		v2.set_position(vec3(rect.end.x,rect.start.y,z));
		self.vertices.push(v2);

		let mut v3 = data.clone();
		v3.set_position(vec3(rect.end.x,rect.end.y,z));
		self.vertices.push(v3);

		self.indices.extend_from_slice(&[v+0,v+1,v+2,v+1,v+2,v+3]);
	}

	pub fn build(self) -> MeshData<T> {
		MeshData { 
			vertices: self.vertices,
			indices: Some(MeshIndexData::U16(self.indices))
		}
	}
}

impl<T: VertexTyWithPosition + VertexTyWithTexCoord + VertexTy> MeshBuilder<T> {
	pub fn uv_rect(&mut self, rect: Rect2D, uv: Rect2D, z: f32, data: T) {
		let v = self.vertices.len() as u16;

		let mut v0 = data.clone();
		v0.set_position(vec3(rect.start.x,rect.start.y,z));
		v0.set_tex_coord(uv.start);
		self.vertices.push(v0);

		let mut v1 = data.clone();
		v1.set_position(vec3(rect.start.x,rect.end.y,z));
		v1.set_tex_coord(vec2(uv.start.x, uv.end.y));
		self.vertices.push(v1);

		let mut v2 = data.clone();
		v2.set_position(vec3(rect.end.x,rect.start.y,z));
		v2.set_tex_coord(vec2(uv.end.x, uv.start.y));
		self.vertices.push(v2);

		let mut v3 = data.clone();
		v3.set_position(vec3(rect.end.x,rect.end.y,z));
		v3.set_tex_coord(uv.end);
		self.vertices.push(v3);

		self.indices.extend_from_slice(&[v+0,v+1,v+2,v+1,v+2,v+3]);
	}
}