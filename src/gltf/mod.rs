use std::{cell::{OnceCell, RefCell}, collections::HashMap, io::Read, marker::PhantomData, ops::{Deref, Sub}, path::PathBuf, rc::{Rc, Weak}, u16, usize};

mod json;
use json::*;

trait IntoF32Norm: Sized {
	fn into_norm<const N: usize>(v: [Self; N]) -> [f32; N]; 
}
impl IntoF32Norm for u8 {
	fn into_norm<const N: usize>(v: [Self; N]) -> [f32; N] {
		v.map(|e| e as f32 / u8::MAX as f32)
	}
}
impl IntoF32Norm for u16 {
	fn into_norm<const N: usize>(v: [Self; N]) -> [f32; N] {
		v.map(|e| e as f32 / u16::MAX as f32)
	}
}
impl IntoF32Norm for i8 {
	fn into_norm<const N: usize>(v: [Self; N]) -> [f32; N] {
		v.map(|e| (e as f32 / i8::MAX as f32).max(-1.0))
	}
}
impl IntoF32Norm for i16 {
	fn into_norm<const N: usize>(v: [Self; N]) -> [f32; N] {
		v.map(|e| (e as f32 / i16::MAX as f32).max(-1.0))
	}
}
impl IntoF32Norm for u32 {
	fn into_norm<const N: usize>(v: [Self; N]) -> [f32; N] {
		v.map(|e| e as f32 / u32::MAX as f32)
	}
}

fn extend<T: Copy, const N: usize, const N2: usize>(v: [T;N], default: T) -> [T;N2] {
	let mut out = [default; N2];
	for i in 0..N.min(N2) {
		out[i] = v[i];
	}
	out
}

#[derive(Debug)]
pub struct Primitive {
	pub indices: Option<Vec<u32>>,
	pub n_vertices: usize,
	pub positions: Vec<[f32;3]>,
	pub normals: Vec<[f32;3]>,
	pub tangents: Vec<[f32;4]>,
	pub tex_coords: Vec<Vec<[f32;2]>>,
	pub colors: Vec<Vec<[f32;4]>>,
	pub joints: Vec<Vec<[u16;4]>>,
	pub weights: Vec<Vec<[f32;4]>>,
}

#[derive(Debug)]
pub struct Mesh {
	pub primitives: Vec<Primitive>,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeID(usize);

#[derive(Debug)]
pub struct Node {
	pub name: Option<String>,
	pub parent: Option<NodeID>,
	pub children: Vec<NodeID>,
	pub mesh: Option<Rc<Mesh>>,
	pub local_transform: glam::Mat4,
	pub global_transform: glam::Mat4,
}

#[derive(Debug)]
pub struct Scene {
	pub name: Option<String>,
	pub root_nodes: Vec<NodeID>,
}

#[derive(Debug)]
pub struct NodeGraph {
	nodes: Vec<Node>,
}

impl NodeGraph {
	pub fn get(&self, id: NodeID) -> &Node {
		self.nodes.get(id.0).unwrap()
	}

	pub fn iter(&self) -> impl std::iter::Iterator<Item = &Node> {
		self.nodes.iter()
	}
}

#[derive(Debug)]
pub struct GltfDecoded {
	pub nodes: NodeGraph,
	pub scene: Rc<Scene>,
	pub scenes: Vec<Rc<Scene>>,
}

#[derive(Debug, thiserror::Error)]
pub enum GltfError {
	#[error("invalid path")]
	InvalidPath,
	#[error("io error")]
	IoError(#[from] std::io::Error),
	#[error("uri \"{0}\" must not be absolute")]
	AbsoluteURI(PathBuf),
	#[error("uri \"{0}\" must not refernce parent directories")]
	ParentDirURI(PathBuf),
	#[error("invalid uri \"{0}\"")]
	InvalidURI(String),
	#[error("invalid data")]
	InvalidData,
	#[error("invalid json")]
	InvalidJSON(#[from] serde_json::error::Error),
	#[error("importer is missing feature {0}")]
	MissingFeature(String),
	#[error("unsupported version \"{0}\", only version 2 is supported")]
	UnsupportedVersion(u32),
}

const COMPONENT_I8: u32 = 5120;
const COMPONENT_U8: u32 = 5121;
const COMPONENT_I16: u32 = 5122;
const COMPONENT_U16: u32 = 5123;
const COMPONENT_U32: u32 = 5125;
const COMPONENT_F32: u32 = 5126;

struct Accessors {
	buffers: Vec<Vec<u8>>,
	views: Vec<JSONBufferView>,
	accessors: Vec<JSONAccessor>,
}

impl Accessors {
	fn convert<
		Input: bytemuck::Pod,
		Output: std::fmt::Debug,
		F: Fn(Input) -> Output
	>(
		&self,
		i: usize,
		convert_fn: F,
	) -> Result<Vec<Output>, GltfError> {
		let accessor = self.accessors.get(i).ok_or(GltfError::InvalidData)?;
		if let Some(sparse) = &accessor.sparse {
			// idk what to do for now
			Err(GltfError::MissingFeature(String::from("sparse accessors")))
		} else {
			let view = self.views.get(accessor.buffer_view).ok_or(GltfError::InvalidData)?;
			let offset = view.byte_offset.unwrap_or(0) + accessor.byte_offset.unwrap_or(0);
			let size = std::mem::size_of::<Input>();
			
			let stride = view.byte_stride.unwrap_or(size);
			let mut buffer = self.buffers.get(view.buffer).ok_or(GltfError::InvalidData)?.as_slice();
			(_, buffer) = buffer.split_at_checked(offset).ok_or(GltfError::InvalidData)?;
			(buffer, _) = buffer.split_at_checked(view.byte_length).ok_or(GltfError::InvalidData)?;
			Ok(buffer.chunks(stride).map(
				|chunk| convert_fn(*bytemuck::from_bytes(&chunk[..size]))
			).collect::<Vec<_>>())
		}
	}

	fn get_component_and_type(&self, i: usize) -> Result<(u32, &str), GltfError> {
		self.accessors.get(i).map(|v| (v.component_type, v.ty.as_str())).ok_or(GltfError::InvalidData)
	}
}

fn str_n(prefix: &str, n: usize) -> String {
	let mut s = String::from(prefix);
	s.push_str(&n.to_string());
	s
}

fn validate_index<T>(data: &[T], i: usize) -> Result<NodeID, GltfError> {
	if i >= data.len() {
		Err(GltfError::InvalidData)
	} else {
		Ok(NodeID(i))
	}
}

pub fn decode_gltf(path: std::path::PathBuf) -> Result<GltfDecoded, GltfError> {
	let path_parent = std::path::PathBuf::from(path.parent().ok_or(GltfError::InvalidPath)?);
	let file = std::fs::read(path).map_err(|e| GltfError::IoError(e))?;

	// figure out if file is binary
	let is_binary = if let Some((header, _)) = file.split_first_chunk::<12>() {
		let (magic, rest) = header.split_first_chunk::<4>().unwrap();
		let (version, _) = rest.split_first_chunk::<4>().unwrap();
		let has_magic_num = u32::from_le_bytes(*magic) == 0x46546C67;
		if has_magic_num {
			let version = u32::from_le_bytes(*version);
			if version != 2 {
				return Err(GltfError::UnsupportedVersion(version));
			}
		}
		has_magic_num
	} else {
		false
	};

	// handle binary/json parsing stuff
	let (json, mut internal_buffer) = if is_binary {
		// skip header
		let (_, data) = file.split_first_chunk::<12>().ok_or(GltfError::InvalidData)?;
		let (json_length, data) = data.split_first_chunk::<4>().ok_or(GltfError::InvalidData)?;
		let (json_type, data) = data.split_first_chunk::<4>().ok_or(GltfError::InvalidData)?;
		if u32::from_le_bytes(*json_type) != 0x4E4F534A {
			return Err(GltfError::InvalidData)?;
		}
		let (json_bytes, data) = data.split_at_checked(u32::from_le_bytes(*json_length) as usize).ok_or(GltfError::InvalidData)?;
		let json = serde_json::from_str::<JSONGltf>(
			std::str::from_utf8(&json_bytes).map_err(|_| GltfError::InvalidData)?
		).map_err(|e| GltfError::InvalidJSON(e))?;
		let internal_buffer = if data.len() > 0 {
			let (binary_length_maybe, data) = data.split_first_chunk::<4>().ok_or(GltfError::InvalidData)?;
			let (binary_chunk_type_maybe, data) = data.split_first_chunk::<4>().ok_or(GltfError::InvalidData)?;
			if u32::from_le_bytes(*binary_chunk_type_maybe) == 0x004E4942 {
				let (binary_data, _) = data.split_at_checked(u32::from_le_bytes(*binary_length_maybe) as usize).ok_or(GltfError::InvalidData)?;
				Some(binary_data)
			} else {
				None
			}
		} else {
			None
		};
		(json, internal_buffer)
	} else {(
		serde_json::from_str::<JSONGltf>(
			std::str::from_utf8(&file).map_err(|_| GltfError::InvalidData)?
		).map_err(|e| GltfError::InvalidJSON(e))?,
		None,
	)};
	
	// accessors
	let accessors = Accessors {
		buffers: json.buffers.unwrap_or_default().into_iter().map(|buffer| {
			if let Some(uri) = &buffer.uri {
				let path = std::path::PathBuf::try_from(uri).map_err(|_| GltfError::InvalidURI(uri.clone()))?;
				if path.is_absolute() {
					return Err(GltfError::AbsoluteURI(path));
				}
				if path.components().find(|c| *c == std::path::Component::ParentDir).is_some() {
					return Err(GltfError::ParentDirURI(path));
				}
				let mut full_path = path_parent.clone();
				full_path.push(path);
				Ok(std::fs::read(full_path)?)
			} else if let Some(buffer) = internal_buffer.take() {
				Ok(buffer.to_vec())
			} else {
				Err(GltfError::InvalidData)
			}
		}).collect::<Result<Vec<_>, _>>()?,
		views: json.buffer_views.unwrap_or_default(),
		accessors: json.accessors.unwrap_or_default(),
	};
	
	// meshes
	let meshes = json.meshes.unwrap_or_default().into_iter().map(|mesh| {
		Ok(Rc::new(Mesh {
			primitives: mesh.primitives.into_iter().map(|primitive| {
				let positions = primitive.attributes.get("POSITION").map(|i|
					accessors.convert(*i, |v| v)
				).transpose()?.ok_or(GltfError::InvalidData)?;
				let normals = primitive.attributes.get("NORMAL").map(|i|
					accessors.convert(*i, |v| v)
				).transpose()?.unwrap_or_default();
				let tangents = primitive.attributes.get("TANGENT").map(|i|
					accessors.convert(*i, |v| v)
				).transpose()?.unwrap_or_default();
				let tex_coords = (0..).map_while(|n| primitive.attributes.get(&str_n("TEXCOORD_", n)).map(|i| {
					match accessors.get_component_and_type(*i)? {
						(COMPONENT_F32, "VEC2") => accessors.convert(*i, |v| v),
						(COMPONENT_U8, "VEC2") => accessors.convert::<[u8;2],_,_>(*i, |v| u8::into_norm(v)),
						(COMPONENT_U16, "VEC2") => accessors.convert::<[u16;2],_,_>(*i, |v| u16::into_norm(v)),
						_ => Err(GltfError::InvalidData),
					}
				})).collect::<Result<Vec<_>,GltfError>>()?;
				let colors = (0..).map_while(|n| primitive.attributes.get(&str_n("COLOR_", n)).map(|i| {
					match accessors.get_component_and_type(*i)? {
						(COMPONENT_F32, "VEC3") => accessors.convert::<[f32;3],_,_>(*i, |v| extend(v, 1.0)),
						(COMPONENT_U8, "VEC3") => accessors.convert::<[u8;3],_,_>(*i,  |v| extend(u8::into_norm(v), 1.0)),
						(COMPONENT_U16, "VEC3") => accessors.convert::<[u16;3],_,_>(*i,  |v| extend(u16::into_norm(v), 1.0)),
						(COMPONENT_F32, "VEC4") => accessors.convert(*i, |v| v),
						(COMPONENT_U8, "VEC4") => accessors.convert::<[u8;4],_,_>(*i,  |v| u8::into_norm(v)),
						(COMPONENT_U16, "VEC4") => accessors.convert::<[u16;4],_,_>(*i,  |v| u16::into_norm(v)),
						_ => Err(GltfError::InvalidData),
					}
				})).collect::<Result<Vec<_>,GltfError>>()?;
				let joints = (0..).map_while(|n| primitive.attributes.get(&str_n("JOINTS_", n)).map(|i| {
					match accessors.get_component_and_type(*i)? {
						(COMPONENT_U8, "VEC4") => accessors.convert::<[u8;4],_,_>(*i, |v| v.map(|c| c as u16)),
						(COMPONENT_U16, "VEC4") => accessors.convert(*i, |v| v),
						_ => Err(GltfError::InvalidData),
					}
				})).collect::<Result<Vec<_>,GltfError>>()?;
				let weights = (0..).map_while(|n| primitive.attributes.get(&str_n("WEIGHTS_", n)).map(|i| {
					match accessors.get_component_and_type(*i)? {
						(COMPONENT_F32, "VEC4") => accessors.convert(*i, |v| v),
						(COMPONENT_U8, "VEC4") => accessors.convert::<[u8;4],_,_>(*i,  |v| u8::into_norm(v)),
						(COMPONENT_U16, "VEC4") => accessors.convert::<[u16;4],_,_>(*i,  |v| u16::into_norm(v)),
						_ => Err(GltfError::InvalidData),
					}
				})).collect::<Result<Vec<_>,GltfError>>()?;
				let indices = primitive.indices.map(|i| {
					match accessors.get_component_and_type(i)? {
						(COMPONENT_U16, _) => accessors.convert::<u16,_,_>(i, |v| v as u32),
						(COMPONENT_U32, _) => accessors.convert(i, |v| v),
						_ => Err(GltfError::InvalidData),
					}
				}).transpose()?;
				let n_vertices = positions.len();
				Ok(Primitive {
					positions, normals, tangents, tex_coords, indices, n_vertices, colors, joints, weights,
				})
			}).collect::<Result<Vec<_>, GltfError>>()?,
		}))
	}).collect::<Result<Vec<_>, GltfError>>()?;

	// nodes
	let nodes_json = json.nodes.unwrap_or_default();
	let mut nodes = nodes_json.iter().map(|node| {
		Ok(Node {
			name: node.name.clone(),
			parent: None,
			mesh: node.mesh.map(|i| meshes.get(i).map(|rc| rc.clone()).ok_or(GltfError::InvalidData)).transpose()?,
			local_transform: if let Some(matrix) = node.matrix {
				glam::Mat4::from_cols_slice(&matrix)
			} else {
				glam::Mat4::from_scale_rotation_translation(
					node.scale.map(|s| s.into()).unwrap_or(glam::Vec3::ONE),
					node.rotation.map(|r| glam::Quat::from_array(r)).unwrap_or_default(), 
					node.translation.map(|t| t.into()).unwrap_or(glam::Vec3::ZERO),
				)
			},
			global_transform: glam::Mat4::IDENTITY,
			children: node.children.iter().flatten().map(|i|
				validate_index(&nodes_json, *i)
			).collect::<Result<Vec<_>, GltfError>>()?,
		})
	}).collect::<Result<Vec<_>, GltfError>>()?;

	// set node parents
	for parent in 0..nodes.len() {
		for i in 0..nodes[parent].children.len() {
			let child = nodes[parent].children[i];
			nodes[child.0].parent = Some(NodeID(parent));
		}
	}
	// node global transforms
	for i in 0..nodes.len() {
		let mut transform = nodes[i].local_transform;
		let mut node = i;
		while let Some(parent) = nodes[node].parent {
			node = parent.0;
			transform *= nodes[parent.0].local_transform;
		}
		nodes[i].global_transform = transform;
	}

	let scenes = json.scenes.iter().flatten().map(|scene| {
		Ok(Rc::new(Scene {
			name: scene.name.clone(),
			root_nodes: scene.nodes.iter().flatten().map(|i|
				validate_index(&nodes_json, *i)
			).collect::<Result<Vec<_>, GltfError>>()?,
		}))
	}).collect::<Result<Vec<_>, GltfError>>()?;

	Ok(GltfDecoded {
		scene: scenes.get(json.scene).map(|rc| rc.clone()).ok_or(GltfError::InvalidData)?,
		nodes: NodeGraph { nodes }, scenes
	})
}