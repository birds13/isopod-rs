use std::sync::Arc;

use crate::util::*;

use super::*;
use ash::*;

mod frame_resources;
mod pipeline;
mod surface;
mod image;
mod render;
mod mesh;
mod buffer;
mod util;
mod ctx;

use frame_resources::*;
use pipeline::*;
use surface::*;
use image::*;
use mesh::*;
use buffer::*;
use util::*;
use ctx::*;

pub struct VulkanGfxBackend {
	_device_idler: VKDeviceIdler,

	pipelines: SparseVec<VKGfxPipeline>,
	texture_2ds: SparseVec<VKImage>,
	samplers: SparseVec<VKSampler>,
	meshes: SparseVec<VKMesh>,
	instances: SparseVec<VKInstances>,
	uniforms: SparseVec<VKBuffer>,
	framebuffers: SparseVec<VKFrameBuffer>,
	destroy_queue: Vec<Destroyable>,

	surface: VKSurface,

	frame_resources: (VKFrameResources, VKFrameResources),
	even_frame: bool,
	ctx: Arc<VKCtx>,
}

impl VulkanGfxBackend {
	pub fn load(sdl_video: &sdl2::VideoSubsystem, mut window_builder: sdl2::video::WindowBuilder) -> anyhow::Result<(Self, sdl2::video::Window)> {

		// do sdl setup
		sdl_video.vulkan_load_library_default().unwrap();
		let window = window_builder.vulkan().build().unwrap();

		// create context
		let ctx = Arc::new(VKCtx::new(&window.vulkan_instance_extensions().unwrap().into_iter().map(|s| 
			std::ffi::CString::new(s.as_bytes()).unwrap()
		).collect::<Vec<_>>())?);

		// create surface
		let surface = VKSurface::new(&ctx, &window)?;

		// create per-frame resources
		let frame_resources = (
			VKFrameResources::new(&ctx),
			VKFrameResources::new(&ctx),
		);

		Ok((Self {
			_device_idler: VKDeviceIdler { ctx: ctx.clone() },
			ctx, surface, frame_resources,
			even_frame: false,
			pipelines: SparseVec::new(), texture_2ds: SparseVec::new(), meshes: SparseVec::new(), instances: SparseVec::new(), uniforms: SparseVec::new(),
			framebuffers: SparseVec::new(), samplers: SparseVec::new(), destroy_queue: Vec::new(),
		}, window))
	}
}