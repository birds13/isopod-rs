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
	fn id() -> MaterialAttributeID { MaterialAttributeID::Sampler }
}

impl MaterialAttributeRef<Sampler> for Sampler {
	fn id(&self) -> MaterialAttributeRefID { MaterialAttributeRefID::Sampler { id: self.id } }
}

#[derive(Clone)]
pub struct Texture2D {
	pub(crate) id: usize,
	pub(crate) size: UVec2,
	pub(crate) attribute: (TextureAttributeID, NormalizationID),
	pub(crate) _rc: Arc<()>,
}

impl Texture2D {
	pub fn size(&self) -> UVec2 { self.size }
}

#[derive(PartialEq, Clone, Copy)]
#[doc(hidden)]
pub(crate) enum CanvasID {
	Framebuffer(usize),
	Screen,
}

pub trait Canvas {
	#[doc(hidden)]
	fn id(&self) -> CanvasID;
}

pub struct ScreenCanvas;
impl Canvas for ScreenCanvas {
	fn id(&self) -> CanvasID { CanvasID::Screen }
}

#[derive(Clone, Copy, EnumCount, EnumIter)]
pub enum FramebufferFormat {
	Rgba8Srgb,
}

#[derive(Clone)]
pub struct FramebufferDefinition {
	pub size: UVec2,
	pub format: FramebufferFormat,
}

pub struct Framebuffer {
	pub(crate) id: usize,
	pub(crate) size: UVec2,
	pub(crate) format: FramebufferFormat,
	pub(crate) _rc: Arc<FramebufferDefinition>,
}

impl Framebuffer {
	pub fn size(&self) -> UVec2 { self.size }
}

impl Canvas for Framebuffer {
	fn id(&self) -> CanvasID { CanvasID::Framebuffer(self.id) }
}

impl MaterialAttribute for Texture2D {
	fn id() -> MaterialAttributeID { MaterialAttributeID::Texture2D }
}

impl MaterialAttributeRef<Texture2D> for Texture2D {
	fn id(&self) -> MaterialAttributeRefID {
		MaterialAttributeRefID::Texture2D { id: self.id }
	}
}

impl MaterialAttributeRef<Texture2D> for Framebuffer {
	fn id(&self) -> MaterialAttributeRefID {
		MaterialAttributeRefID::FrameBuffer { id: self.id }
	}
}