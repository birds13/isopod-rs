use isopod::gfx::util::MeshBuilder;
use isopod::gfx::*;
use isopod::math::*;
use isopod::*;

#[repr(C)]
#[derive(Copy, Clone, Default, isopod::VertexTy)]
struct Vertex {
	#[position]
	position: Vec3,
	#[tex_coord]
	uv: Vec2,
}

material_ty!(TexMat {
	tex: Texture2D,
	sp: Sampler,
});

struct OffscreenTest {
	shader: Shader<Vertex, (), TexMat, ()>,
	fb: Framebuffer,
	sp: Sampler,
}

impl isopod::App for OffscreenTest {
	fn update(&mut self, c: &isopod::EngineCtx) {
		let tex = c.gfx.create_texture2d(TextureData::from_png(include_bytes!("test.png")).unwrap().normalize::<Srgb>());

		let mut fs_quad_builder = MeshBuilder::new();
		fs_quad_builder.uv_rect(Rect2D::new(vec2(-1., -1.), vec2(1., 1.)), Rect2D::new(Vec2::ZERO, Vec2::ONE), 0.5, Vertex::default());
		let fs_quad = c.gfx.imm_mesh(fs_quad_builder.build());

		c.gfx.set_canvas(&self.fb, Some(vec4(0.1, 0.1, 0.1, 0.0)));
		let picture_cfg = c.gfx.shader_cfg(&self.shader, TexMat::cfg(&c.gfx, &tex, &self.sp));
		picture_cfg.draw(&fs_quad, &(), ());

		c.gfx.set_canvas(&ScreenCanvas, Some(vec4(0.1, 0.1, 0.1, 0.0)));
		let final_cfg = c.gfx.shader_cfg(&self.shader, TexMat::cfg(&c.gfx, &self.fb, &self.sp));
		final_cfg.draw(&fs_quad, &(), ());
	}
}

fn main() {
	isopod::run(|c| {
		OffscreenTest {
			shader: c.gfx.create_shader(ShaderDefinition {
				code: "r#
					[varying]
					vec2 vuv;

					[vertex]
					void main() {
						gl_Position = vec4(position, 1.0);
						vuv = uv;
					}

					[fragment]
					void main() {
						out_color = texture(sampler2D(tex, sp), vuv);
					}
				".into(),
				..Default::default()
			}),
			sp: c.gfx.create_sampler(SamplerDefinition::default()),
			fb: c.gfx.create_framebuffer(FramebufferDefinition { size: uvec2(512, 512), format: FramebufferFormat::Rgba8Srgb })
		}
	});
}