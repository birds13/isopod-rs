use std::collections::HashMap;
use std::error::Error;

use rustc_hash::FxBuildHasher;

use crate::material_ty;
use crate::util::*;
use crate::gfx::*;
use crate::math::*;

#[repr(C)]
#[derive(Copy, Clone, Default, isopod_derive::VertexTy)]
#[isopod_crate(crate)]
struct Vertex {
	color: Vec4,
	#[position] position: Vec3,
	#[tex_coord] uv: Vec2,
	_p: Padding<12>,
}

material_ty!(crate | FontMaterial {
	tex: Texture2D,
	sp: Sampler,
});

impl Vertex {
	fn color(c: Vec4) -> Self {
		Self { color: c, ..Default::default() }
	}
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")] 
enum Cmd {
	Msg(String),
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
	shader: Shader<Vertex, (), FontMaterial, Mat4>,
	font_map: HashMap<Option<char>, Rect2D, FxBuildHasher>,
	font_texture: Texture2D,
	font_sampler: Sampler,
}

impl Console {
	pub(crate) fn update(&mut self, gfx: &GfxCtx, input: &super::input::InputCtx) {
		gfx.set_canvas(&ScreenCanvas, None);
		// update text input
		for _ in 0..input.text_input.n_backspaces {
			self.text_input.pop();
		}
		self.text_input.push_str(&input.text_input.text);
		if input.text_input.enter {
			match ron::de::from_str::<Cmd>(&self.text_input) {
				Ok(cmd) => match cmd {
					Cmd::Msg(s) => self.log(s),
				},
				Err(e) => {
					self.error(format!("invalid command: {}", e));
				},
			};
			self.text_input.clear();
		}

		// render
		let mat = Mat4::from_translation(Vec3::new(-1., 1., 0.)) * Mat4::from_scale(Vec3::new(2./gfx.window_size.x, -2./gfx.window_size.y, 1.));
		let mut mesh = MeshU16::new();
		let char_size = Vec2::new(7.*3., 8.*3.);
		let shadow_offset = Vec2::new(3., 3.);
		let padding = 4.*3.;
		let mut cursor = Vec2::ZERO;
		let black = Vertex::color(vec4(0., 0., 0., 1.));
		
		// input
		mesh.uv_rect(
			Rect2D::new(Vec2::ZERO, vec2(gfx.window_size.x, char_size.y + padding * 2.)),
			self.font_map[&None], 0.75, Vertex::color(vec4(0., 0., 0.1, 0.75))
		);
		cursor += padding;
		for char in self.text_input.chars() {
			if char != '\n' {
				let rect = self.font_map[&Some(char)];
				mesh.uv_rect(Rect2D::with_extent(cursor + shadow_offset, char_size), rect, 0.5, black);
				mesh.uv_rect(Rect2D::with_extent(cursor, char_size), rect, 0.5, Vertex::color(vec4(0.9, 0.9, 0.9, 1.0)));
				cursor.x += char_size.x;
			}
		}
		cursor.y += padding + char_size.y;
		cursor.x = 0.;

		// messages
		for msg in self.messages.get_mut().iter().rev() {
			let (bg_color, fg_color) = match msg.ty {
				MsgType::Log => (Vertex::color(vec4(0.1, 0.1, 0.3, 0.5)), Vertex::color(vec4(0.6, 0.6, 0.8, 1.0))),
				MsgType::Warning => (Vertex::color(vec4(0.2, 0.2, 0.0, 0.5)), Vertex::color(vec4(0.8, 0.8, 0.5, 1.0))),
				MsgType::Error => (Vertex::color(vec4(0.3, 0.1, 0.1, 0.5)), Vertex::color(vec4(0.8, 0.6, 0.6, 1.0))),
			};
			let mut bcursor = cursor;
			let mut bcursor_max = 0.;
			bcursor += padding * Vec2::ONE;
			for line in msg.content.split('\n') {
				bcursor.x = padding;
				for word in line.split(' ') {
					if bcursor.x + word.len() as f32 * char_size.x> gfx.window_size.x - padding {
						bcursor.y += char_size.y;
						bcursor.x = padding;
						bcursor_max = gfx.window_size.x;
					} else {
						bcursor.x += char_size.x;
					}
					bcursor.x += word.len() as f32 * char_size.x;
					bcursor_max = bcursor.x.max(bcursor_max);
				}
				bcursor.y += char_size.y;
			}
			bcursor.x = bcursor_max;
			bcursor.y += padding;
			mesh.uv_rect(Rect2D::with_extent(cursor, bcursor - cursor), self.font_map[&None], 0.75, bg_color);
			cursor += padding * Vec2::ONE;
			for line in msg.content.split('\n') {
				cursor.x = padding - char_size.x;
				for word in line.split(' ') {
					if cursor.x + word.len() as f32 * char_size.x> gfx.window_size.x - padding {
						cursor.y += char_size.y;
						cursor.x = padding;
					} else {
						cursor.x += char_size.x;
					}
					for char in word.chars() {
						let rect = self.font_map[&Some(char)];
						mesh.uv_rect(Rect2D::with_extent(cursor + shadow_offset, char_size), rect, 0.5, black);
						mesh.uv_rect(Rect2D::with_extent(cursor, char_size), rect, 0.5, fg_color);
						cursor.x += char_size.x;
					}
				}
				cursor.y += char_size.y;
			}
			cursor.x = 0.;
			cursor.y += padding;
			if cursor.y > gfx.window_size.y {
				break;
			}
		}
		let mesh = gfx.imm_mesh(Mesh::U16(mesh));
		gfx.shader_cfg(&self.shader, &FontMaterial::cfg(&gfx, &self.font_texture, &self.font_sampler)).draw(&mesh, &GPUInstances::one(), mat);
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

	pub(crate) fn new(gfx: &GfxCtx) -> Self {
		let font_texture_data =  texture_from_png_srgb(include_bytes!("font.png").as_slice()).unwrap();

		let mut sprite_offset = UVec2::ZERO;
		let mut sprites = (32 as u8..127 as u8).map(|c| {
			let s = font_texture_data.sprite_slice(
				Some(c as char),
				URect2D::with_start_and_size(sprite_offset, UVec2::new(7,8)),
				0
			);
			sprite_offset.x += 7;
			if sprite_offset.x >= 7*32 {
				sprite_offset.x = 0;
				sprite_offset.y += 8;
			}
			s
		}).collect::<Vec<_>>();
		sprites.push(font_texture_data.sprite_slice(None, URect2D::new(font_texture_data.size_2d() - UVec2::ONE, font_texture_data.size_2d()), 0));
		let (font_atlas, font_map) = pack_sprite_atlas(sprites, UVec2::new(256,256)).unwrap();

		Self {
			text_input: String::new(),
			messages: BufferDequeCell::new(),
			font_texture: gfx.create_texture2d(font_atlas),
			font_map,
			font_sampler: gfx.register_sampler(SamplerDefinition::default()),
			shader: gfx.create_shader(ShaderDefinition {
				code: r#"
					[varying]
					vec2 vuv;
					vec4 vcolor;

					[vertex]
					void main() {
						gl_Position = push.value * vec4(position, 1.0);
						vuv = uv;
						vcolor = color;
					}

					[fragment]
					void main() {
						vec4 c = texture(sampler2D(tex, sp), vuv) * vcolor;
						if (c.a < 0.5) {
							discard;
						}
						out_color = c;
					}
				"#.into(),
				color_blend: Some(ColorBlend::Alpha),
				..Default::default()
			})
		}
	}
}