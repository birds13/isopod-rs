use crate::util::*;

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
}

impl Console {
	pub(crate) fn update(gfx: &super::gfx::GfxCtx, input: &super::input::InputCtx) {
		
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