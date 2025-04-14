
use super::*;

pub(crate) enum ResourceFreeType {
	Shader,
	Texture2D,
	Mesh,
	Instances,
	Uniform,
	Framebuffer,
	Sampler,
}

pub(crate) enum ResourceUpdate {
	CreateShader { id: usize, def: shader::ShaderFullDefinition },
	CreateTexture2D { id: usize, meta: Texture2DMeta, bytes: Vec<u8> },
	CreateMesh { id: usize, data: mesh::MeshBytes },
	CreateInstances { id: usize, data: mesh::InstanceBytes },
	CreateUniform { id: usize, data: Vec<u8> },
	CreateFramebuffer { id: usize, meta: Texture2DMeta },
	CreateSampler { id: usize, def: SamplerDefinition },
	Free { id: usize, ty: ResourceFreeType },
}