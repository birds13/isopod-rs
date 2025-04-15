
use isopod::gfx::*;
use isopod::material_ty;
use isopod::math::*;

#[repr(C)]
#[derive(Copy, Clone, Default, isopod::VertexTy)]
pub struct Vertex {
	position: Vec3,
	normal: Vec3,
	uv: Vec2,
	color: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Default, isopod::UniformTy)]
pub struct ViewBuffer {
	proj: Mat4,
	camera: Mat4,
	camera_position: Vec3,
	_p: Padding<4>,
}

material_ty!(ViewMaterial {
	view: UniformBuffer<'a, ViewBuffer>
});

material_ty!(TextureMaterial {
	tex: GPUTexture2D,
	sp: Sampler,
});

struct SceneDemo {
	dragon_mesh: GPUMeshRes<Vertex>,
	floor_mesh: GPUMeshRes<Vertex>,
	dragon_shader: Shader<Vertex, (), ViewMaterial, ()>,
	floor_shader: Shader<Vertex, (), (ViewMaterial, TextureMaterial), ()>,
	floor_texture: GPUTexture2D,
	pixel_sampler: Sampler,
	t: f32,
}

impl isopod::App for SceneDemo {
	fn update(&mut self, c: &isopod::EngineCtx) {

		self.t += c.dt as f32;
		let window_size = c.gfx.window_canvas.size().as_vec2();

		// set screen as render target
		c.gfx.set_canvas(&c.gfx.window_canvas, None);

		// camera matrix calculations
		let camera_matrix = Mat4::from_translation(Vec3::new(0., -1.0, 4.)) * Mat4::from_rotation_x(-0.25) * Mat4::from_rotation_y(self.t*0.5);
		let camera_position = -camera_matrix.transform_point3(Vec3::ZERO);

		// setup material cfgs for projection/camera matrices
		let view_buffer = c.gfx.imm_uniform_buffer(ViewBuffer {
			proj: Mat4::perspective_lh(1.22, window_size.x/window_size.y, 0.01, 100.),
			camera: camera_matrix,
			camera_position,
			_p: Padding::new(),
		});
		let view_cfg = c.gfx.material_cfg(ViewMaterialRefs {
			view: &view_buffer
		});

		// setup material cfg for floor texture
		let floor_texture_cfg = c.gfx.material_cfg(TextureMaterialRefs {
			tex: &self.floor_texture,
			sp: &self.pixel_sampler
		});

		// draw dragon
		c.gfx.shader_cfg(&self.dragon_shader, &view_cfg).draw(&self.dragon_mesh, &GPUInstances::one(), ());

		// draw floor
		c.gfx.shader_cfg(&self.floor_shader, (&view_cfg, &floor_texture_cfg)).draw(&self.floor_mesh, &GPUInstances::one(), ());
	}
}

fn main() {
	// load scene and process meshes
	let gltf = isopod::gltf::decode_gltf("examples/demo_scene/scene.glb".into()).unwrap();
	let mut dragon_mesh = MeshU32::new();
	let mut floor_mesh = MeshU32::new();

	for node in gltf.nodes.iter() {
		for primitive in node.mesh.as_ref().map(|mesh| &mesh.primitives).unwrap_or(&vec![]) {

			// filter by material name and then grab all information and attach it to relavant meshes
			if let Some(mesh) = primitive.material.as_ref().map(|material| material.name.as_ref().map(|name| match name.as_str() {
				"dragon" => Some(&mut dragon_mesh),
				"floor" => Some(&mut floor_mesh),
				_ => None,
			})).flatten().flatten() {
				let index_offset = mesh.vertices.len() as u32;

				let uvs = primitive.tex_coords.get(0).map(|uv| uv.as_slice()).unwrap_or(&[]);
				let colors = primitive.colors.get(0).map(|color| color.as_slice()).unwrap_or(&[]);
				for i in 0..primitive.n_vertices {
					let position = primitive.positions.get(i).map(|v| Vec3::from(*v)).unwrap_or_default();
					let normal = primitive.normals.get(i).map(|v| Vec3::from(*v)).unwrap_or_default();
					mesh.vertices.push(Vertex {
						position: (node.global_transform * vec4(position.x, position.y, position.z, 1.0)).xyz(),
						normal: (node.global_transform * vec4(normal.x, normal.y, normal.z, 1.0)).xyz(),
						uv: uvs.get(i).map(|v| (*v).into()).unwrap_or_default(),
						color: colors.get(i).map(|v| (*v).into()).unwrap_or(Vec4::ONE),
					});
				}
				if let Some(indices) = &primitive.indices {
					for index in indices {
						mesh.indices.push(index_offset + index)
					}
				} else {
					for i in 0..primitive.n_vertices as u32 {
						mesh.indices.push(index_offset + i);
					}
				}
			}
		}
	}

	// run engine
	isopod::run(|c| {
		SceneDemo {
			dragon_mesh: c.gfx.register_mesh(Mesh::U32(dragon_mesh)),
			floor_mesh: c.gfx.register_mesh(Mesh::U32(floor_mesh)),
			dragon_shader: c.gfx.register_shader(ShaderDefinition {
				code: include_str!("dragon_shader.txt").into(),
				depth_test: true,
				depth_write: true,
				..Default::default()
			}),
			floor_shader: c.gfx.register_shader(ShaderDefinition {
				code: include_str!("floor_shader.txt").into(),
				depth_test: true,
				depth_write: true,
				..Default::default()
			}),
			floor_texture: c.gfx.register_texture2d_srgb(Texture::from_png(include_bytes!("floor.png")).unwrap()),
			pixel_sampler: c.gfx.register_sampler(SamplerDefinition {
				wrap_mode: SamplerWrapMode::Repeat,
				min_linear: false,
				mag_linear: false
			}),
			t: 0.,
		}
	});
}