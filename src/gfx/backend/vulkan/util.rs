
use std::ops::{Deref, DerefMut};

use ash::*;
use glam::*;

use super::*;

pub enum Destroyable {
	GfxPipeline(VKGfxPipeline),
	Texture2D(VKImage),
	Sampler(VKSampler),
	Mesh(VKMesh),
	Instances(VKInstances),
	Uniform(VKBuffer),
	FrameBuffer(VKFrameBuffer),
}

pub fn u16_tuple_to_extent3d(t: (u16,u16,u16)) -> vk::Extent3D {
	vk::Extent3D { width: t.0 as u32, height: t.1 as u32, depth: t.2 as u32 }
}

pub fn extent2d_to_extent3d(e: vk::Extent2D) -> vk::Extent3D {
	vk::Extent3D { width: e.width, height: e.height, depth: 1 }
}

pub fn uvec2_to_extent2d(v: UVec2) -> vk::Extent2D {
	vk::Extent2D { width: v.x, height: v.y }
}

pub struct VKDeviceIdler {
	pub ctx: Arc<VKCtx>,
}

impl Drop for VKDeviceIdler {
	fn drop(&mut self) {
		unsafe {
			self.ctx.device.device_wait_idle().unwrap();
		}
	}
}

pub struct UnsafeDestroyable<T> {
	inner: Option<T>,
}

impl<T> UnsafeDestroyable<T> {
	pub fn new(v: T) -> Self {
		Self { inner: Some(v) }
	}
	// SAFETY: this type MUST not be derefed after this is called
	pub unsafe fn destroy(&mut self) {
		self.inner.take();
	}
}

impl<T> Deref for UnsafeDestroyable<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		unsafe { self.inner.as_ref().unwrap_unchecked() }
	}
}