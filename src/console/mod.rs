use std::collections::HashMap;

use rustc_hash::FxBuildHasher;

use crate::gfx::util::MeshBuilder;
use crate::util::*;
use crate::gfx::*;
use crate::math::*;

#[repr(C)]
#[derive(Copy, Clone, Default, isopod_derive::VertexTy, bytemuck::NoUninit)]
#[isopod_crate(crate)]
struct Vertex {
	color: Vec4,
	#[tex_coord] uv: Vec2,
	_p: Padding<8>,
	#[position] position: Vec3,
	_p2: Padding<4>,
}

#[derive(isopod_derive::MaterialTy)]
#[isopod_crate(crate)]
struct FontMaterial {
	tex: Texture2D,
	sp: Sampler,
}

impl Vertex {
	fn color(c: Vec4) -> Self {
		Self { color: c, ..Default::default() }
	}
}

enum MsgType {
	Log, Warning, Error,
}

struct Msg {
	ty: MsgType,
	size: UVec2,
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
	pub(crate) fn new(gfx: &GfxCtx) -> Self {
		let mut font_reader = png::Decoder::new(include_bytes!("font.png").as_slice()).read_info().unwrap();
		let mut font_bytes = vec![0; font_reader.output_buffer_size()];
		let font_frame_info = font_reader.next_frame(&mut font_bytes).unwrap();
		let font_texture_data = TextureData::<Normalized<U8Vec4, Srgb>>::new_from_bytes(
			font_bytes, UVec3::new(font_frame_info.width, font_frame_info.height, 1)
		).unwrap();

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
			font_sampler: gfx.create_sampler(SamplerDefinition::default()),
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
	
	pub(crate) fn update(&mut self, gfx: &GfxCtx, input: &super::input::InputCtx) {

		unsafe {
			static mut x: usize = 0;
			x += 1;
			self.log(format!("frame: {}", x));
		}

		gfx.set_canvas(&ScreenCanvas, None);
		// update text input
		for _ in 0..input.text_input.n_backspaces {
			self.text_input.pop();
		}
		self.text_input.push_str(&input.text_input.text);

		// render
		let mat = Mat4::from_translation(Vec3::new(-1., 1., 0.)) * Mat4::from_scale(Vec3::new(2./gfx.window_size.x, -2./gfx.window_size.y, 1.));
		let mut builder = MeshBuilder::new();
		let char_size = Vec2::new(7.*3., 8.*3.);
		let padding = 4.*3.;
		let mut cursor = Vec2::ZERO;
		for line in self.messages.get_mut().iter().rev() {
			let (bg_color, fg_color) = match line.ty {
				MsgType::Log => (Vertex::color(vec4(0.1, 0.1, 0.3, 0.5)), Vertex::color(vec4(0.6, 0.6, 0.8, 1.0))),
				MsgType::Warning => (Vertex::color(vec4(0.2, 0.2, 0.0, 0.5)), Vertex::color(vec4(0.8, 0.8, 0.5, 1.0))),
				MsgType::Error => (Vertex::color(vec4(0.3, 0.1, 0.1, 0.5)), Vertex::color(vec4(0.8, 0.6, 0.6, 1.0))),
			};
			builder.uv_rect(Rect2D::with_extent(cursor, (line.size.as_vec2() + Vec2::ONE) * char_size), self.font_map[&None], 0.75, bg_color);
			cursor += padding * Vec2::ONE;
			for char in line.content.chars() {
				if char == '\n' {
					cursor.y += char_size.y;
					cursor.x = padding;
				} else {
					let rect = self.font_map[&Some(char)];
					builder.uv_rect(Rect2D::with_extent(cursor, char_size), rect, 0.5, fg_color);
					cursor.x += char_size.x;
				}
			}
			cursor.x = 0.;
			cursor.y += char_size.y + padding;
			if cursor.y > gfx.window_size.y {
				break;
			}
		}
		let mesh = gfx.imm_mesh(builder.build());
		gfx.shader_cfg(&self.shader, FontMaterial::new(&gfx, &self.font_texture, &self.font_sampler)).draw(&mesh, &(), mat);
	}

	fn msg(&self, msg: impl Into<String>, ty: MsgType) {
		let content = msg.into();
		let mut x = 0;
		let mut size = uvec2(0, 1);
		for char in content.chars() {
			if char == '\n' {
				x = 0;
				size.y += 1;
			} else {
				x += 1;
				size.x = size.x.max(x);
			}
		}
		self.messages.push(Msg {
			ty, content, size, time: 0.,
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