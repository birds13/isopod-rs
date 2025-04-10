use std::marker::PhantomData;

use super::*;

pub(crate) struct MeshDrawCmdDesc {
	pub mesh: mesh::MeshDraw,
	pub instances: mesh::InstancesDraw,
	pub push: [u8; 128]
}

pub(crate) enum DrawCmd {
	SetCanvas { id: texture::CanvasID, clear_color: Option<glam::Vec4> },
	SetShader { id: usize },
	SetMaterial { attributes: Vec<MaterialAttributeRefID>, slot: usize },
	DrawMesh(MeshDrawCmdDesc),
}

pub struct ShaderCfg<'frame, Vertex: VertexTy, Instance: VertexTy, Materials: MaterialSet, Push: UniformTy> {
	pub materials: Materials::Cfgs<'frame>,
	pub(crate) ctx: &'frame GfxCtx,
	pub(crate) shader: usize,
	pub(crate) _data: PhantomData<(Vertex, Instance, Materials, Push)>,
}

impl<'frame, Vertex: VertexTy, Instance: VertexTy, Materials: MaterialSet, Push: UniformTy> ShaderCfg<
	'frame, Vertex, Instance, Materials, Push
> {
	pub fn draw<Mesh: MeshAny<Vertex>, Instances: InstancesAny<Instance>>(&'frame self, mesh: &Mesh, instances: &Instances, push: Push) {
		// set shader
		if self.ctx.frame_data.current_pipeline.get() != self.shader {
			self.ctx.frame_data.current_pipeline.set(self.shader);
			self.ctx.frame_data.draw_cmd_queue.push(DrawCmd::SetShader { id: self.shader });
		}
		// set materials
		for (slot, material) in Materials::iter(&self.materials).enumerate() {
			if self.ctx.frame_data.current_material_ids[slot].get() != material.id {
				self.ctx.frame_data.draw_cmd_queue.push(DrawCmd::SetMaterial { attributes: material.attributes.clone(), slot });
				self.ctx.frame_data.current_material_ids[slot].set(material.id);
			}
		}
		// push constants and command
		if std::mem::size_of::<Push>() > 128 {
			panic!("push constants must have a size less than or equal to 128 bytes");
		}
		let mut cmd = MeshDrawCmdDesc { mesh: mesh.draw(), instances: instances.draw(), push: [0;128] };
		let push_bytes = bytemuck::bytes_of(&push);
		cmd.push[..push_bytes.len()].copy_from_slice(push_bytes);
		self.ctx.frame_data.draw_cmd_queue.push(DrawCmd::DrawMesh(cmd));
	}
}