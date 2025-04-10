use std::collections::HashMap;


#[derive(Debug, serde::Deserialize)]
pub struct JSONNode {
	pub name: Option<String>,
	pub matrix: Option<[f32;16]>,
	pub rotation: Option<[f32;4]>,
	pub scale: Option<[f32;3]>,
	pub translation: Option<[f32;3]>,
	pub mesh: Option<usize>,
	pub children: Option<Vec<usize>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONPrimitive {
	pub attributes: HashMap<String, usize>,
	pub indices: Option<usize>,
	pub material: Option<usize>,
	pub mode: Option<usize>,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONMesh {
	pub name: Option<String>,
	pub primitives: Vec<JSONPrimitive>,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONScene {
	pub name: Option<String>,
	pub nodes: Option<Vec<usize>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONSparseIndices {
	#[serde(alias = "bufferView")]
	pub buffer_view: usize,
	#[serde(alias = "byteOffset")]
	pub byte_offset: Option<usize>,
	#[serde(alias = "componentType")]
	pub component_type: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONSparseValues {
	#[serde(alias = "bufferView")]
	pub buffer_view: usize,
	#[serde(alias = "byteOffset")]
	pub byte_offset: Option<usize>,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONSparse {
	pub indices: JSONSparseIndices,
	pub values: JSONSparseValues,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONAccessor {
	#[serde(alias = "bufferView")]
	pub buffer_view: usize,
	#[serde(alias = "byteOffset")]
	pub byte_offset: Option<usize>,
	#[serde(alias = "componentType")]
	pub component_type: u32,
	#[serde(alias = "type")]
	pub ty: String,
	pub sparse: Option<JSONSparse>,
	pub count: Option<usize>,
	// todo: target?
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONBufferView {
	pub buffer: usize,
	#[serde(alias = "byteLength")]
	pub byte_length: usize,
	#[serde(alias = "byteOffset")]
	pub byte_offset: Option<usize>,
	#[serde(alias = "byteStride")]
	pub byte_stride: Option<usize>,
	// todo: target?
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONBuffer {
	#[serde(alias = "byteLength")]
	pub byte_length: usize,
	pub uri: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONMaterial {
	pub name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct JSONGltf {
	pub scenes: Option<Vec<JSONScene>>,
	pub scene: usize,
	pub nodes: Option<Vec<JSONNode>>,
	pub buffers: Option<Vec<JSONBuffer>>,
	pub meshes: Option<Vec<JSONMesh>>,
	pub materials: Option<Vec<JSONMaterial>>,
	#[serde(alias = "bufferViews")]
	pub buffer_views: Option<Vec<JSONBufferView>>,
	pub accessors: Option<Vec<JSONAccessor>>,
}