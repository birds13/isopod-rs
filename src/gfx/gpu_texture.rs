#![allow(private_interfaces)]
use std::sync::Arc;

use crate::math::*;
use strum_macros::{EnumCount, EnumIter};

use super::*;

#[derive(Default, Clone, Copy)]
pub enum SamplerWrapMode {
	#[default]
	Repeat,
	Extend,
	Mirror,
}

#[derive(Default, Clone)]
pub struct SamplerDefinition {
	pub wrap_mode: SamplerWrapMode,
	pub min_linear: bool,
	pub mag_linear: bool,
}

pub struct Sampler {
	pub(crate) id: usize,
	pub(crate) _rc: Arc<SamplerDefinition>,
}

impl MaterialAttribute for Sampler {
	fn id() -> MaterialAttributeID {
		MaterialAttributeID { inner: MaterialAttributeIDInner::Sampler }
	}
	fn ref_id(&self) -> MaterialAttributeRefID {
		MaterialAttributeRefID { inner: MaterialAttributeRefIDInner::Sampler { id: self.id } }
	}
}

#[derive(Clone)]
pub(crate) struct Texture2DMeta {
	pub size: UVec2,
	pub format: TextureFormatID,
}

pub(crate) enum Texture2DTy {
	Texture2D, FramebufferColor, FramebufferDepth,
}

pub struct GPUTexture2D {
	pub(crate) ty: Texture2DTy,
	pub(crate) id: usize,
	pub(crate) rc: Arc<Texture2DMeta>,
}

impl GPUTexture2D {
	pub fn size(&self) -> UVec2 {
		self.rc.size
	}
}

impl MaterialAttribute for GPUTexture2D {
	fn id() -> MaterialAttributeID {
		MaterialAttributeID { inner: MaterialAttributeIDInner::Texture2D }
	}

	fn ref_id(&self) -> MaterialAttributeRefID {
		match self.ty {
			Texture2DTy::Texture2D => MaterialAttributeRefID { inner: MaterialAttributeRefIDInner::Texture2D { id: self.id }},
			Texture2DTy::FramebufferColor => MaterialAttributeRefID { inner: MaterialAttributeRefIDInner::FramebufferColor { id: self.id }},
			Texture2DTy::FramebufferDepth => MaterialAttributeRefID { inner: MaterialAttributeRefIDInner::FramebufferDepth { id: self.id }},
		}
	}
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub(crate) enum CanvasID {
	Framebuffer(usize),
	Screen
}

pub struct Canvas {
	pub(crate) id: CanvasID,
	pub(crate) size: UVec2,
}

impl Canvas {
	pub fn size(&self) -> UVec2 {
		self.size
	}
}

pub struct Framebuffer {
	color: GPUTexture2D,
	depth: GPUTexture2D,
	canvas: Canvas,
}

impl Framebuffer {
	pub fn new(id: usize, rc: Arc<Texture2DMeta>) -> Self {
		Self {
			color: GPUTexture2D { ty: Texture2DTy::FramebufferColor, id, rc: rc.clone() },
			depth: GPUTexture2D { ty: Texture2DTy::FramebufferDepth, id, rc: rc.clone() },
			canvas: Canvas { id: CanvasID::Framebuffer(id), size: rc.size }
		}
	}

	pub fn size(&self) -> UVec2 {
		self.canvas.size
	}

	pub fn color(&self) -> &GPUTexture2D {
		&self.color
	}

	pub fn depth(&self) -> &GPUTexture2D {
		&self.depth
	}

	pub fn canvas(&self) -> &Canvas {
		&self.canvas
	}
}