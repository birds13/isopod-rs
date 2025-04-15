use isopod::gfx::*;
use isopod::math::*;

#[repr(C)]
#[derive(Copy, Clone, Default, isopod::VertexTy)]
struct Vertex {
	position: Vec3,
	color: Vec3,
}

impl Vertex {
	pub fn new(position: Vec3, color: Vec3) -> Self {
		Self { position, color }
	}
}

struct TriangleDemo {
	triangle_shader: Shader<Vertex, (), (), ()>,
}

impl isopod::App for TriangleDemo {
	fn update(&mut self, c: &isopod::EngineCtx) {
		c.gfx.set_canvas(&c.gfx.window_canvas, None);
		let triangle_cfg = c.gfx.shader_cfg(&self.triangle_shader, ());
		triangle_cfg.draw(
			&c.gfx.imm_mesh(Mesh::NoIndices(vec![
				Vertex::new(vec3(-0.5, -0.5, 0.5), vec3(1.0, 0.0, 0.0)),
				Vertex::new(vec3(0.5, -0.5, 0.5), vec3(0.0, 1.0, 0.0)),
				Vertex::new(vec3(0.0, 0.6, 0.5), vec3(0.0, 0.0, 1.0)),
			])),
			&GPUInstances::one(),
			()
		);
	}
}

fn main() {
	isopod::run(|c| {
		TriangleDemo {
			triangle_shader: c.gfx.register_shader(ShaderDefinition {
				code: include_str!("shader.txt").into(),
				..Default::default()
			})
		}
	});
}