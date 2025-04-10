
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
	CreateTexture2D { id: usize, data: TextureDataBytes },
	CreateMesh { id: usize, data: mesh::MeshDataBytes },
	CreateInstances { id: usize, data: mesh::InstanceDataBytes },
	CreateUniform { id: usize, data: Vec<u8> },
	CreateFramebuffer { id: usize, def: FramebufferDefinition },
	CreateSampler { id: usize, def: SamplerDefinition },
	Free { id: usize, ty: ResourceFreeType },
}