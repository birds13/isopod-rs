use crate::gfx::util::MeshBuilder;
use crate::util::*;
use crate::gfx::*;
use crate::math::*;

#[repr(C)]
#[derive(Copy, Clone, Default, isopod_derive::VertexTy, bytemuck::NoUninit)]
#[isopod_crate(crate)]
struct Vertex {
	#[position]
	position: Vec3,
	color: Vec3,
}

enum MsgType {
	Log, Warning, Error,
}

struct Msg {
	ty: MsgType,
	content: String,
	time: f32,
}

pub struct Console {
	text_input: String,
	messages: BufferDequeCell<Msg>,
	shader: Shader<Vertex, (), (), Mat4>,
}

impl Console {
	pub(crate) fn update(&mut self, gfx: &super::gfx::GfxCtx, input: &super::input::InputCtx) {
		// update text input
		for _ in 0..input.text_input.n_backspaces {
			self.text_input.pop();
		}
		self.text_input.push_str(&input.text_input.text);

		// render
		let mat = Mat4::IDENTITY;
		let builder = MeshBuilder::new();
		let mesh = gfx.imm_mesh(builder.build());
		gfx.shader_cfg(&self.shader, ()).draw(&mesh, &(), mat);
	}

	fn msg(&self, msg: impl Into<String>, ty: MsgType) {
		self.messages.push(Msg {
			ty, content: msg.into(), time: 0.,
		});
	}

	pub fn log(&self, msg: impl Into<String>) {
		self.msg(msg, MsgType::Log);
	}

	pub fn warn(&self, msg: impl Into<String>) {
		self.msg(msg, MsgType::Warning);
	}

	pub fn error(&self, msg: impl Into<String>) {
		self.msg(msg, MsgType::Error);
	}
}