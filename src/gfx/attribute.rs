#![allow(private_interfaces)]

use std::marker::PhantomData;

use crate::{math::*, util::align_up};

#[doc(hidden)]
#[derive(Clone)]
pub struct StructAttribute<T> {
	pub attribute: T,
	pub offset: usize,
	pub name: &'static str,
}

#[doc(hidden)]
#[derive(Clone)]
pub struct StructLayout<T> {
	pub attributes: Vec<StructAttribute<T>>,
	pub size: usize,
}

impl<T> StructLayout<T> {
	pub fn unit() -> Self {
		StructLayout { attributes: Vec::with_capacity(0), size: 0 }
	}

	pub fn is_empty(&self) -> bool {
		self.size == 0 || self.attributes.is_empty()
	}
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Padding<const N: usize> {
	_bytes: [u8; N],
}

impl<const N: usize> Default for Padding<N> {
	fn default() -> Self {
		Self { _bytes: [0;N] }
	}
}

impl<const N: usize> Padding<N> {
	pub fn new() -> Self { Self::default() }
}

/// Enum representation of a [`TextureFormat`].
/// 
/// SRGB formats (starting with `Srgb`):
/// - Will convert from SRGB colorspace to linear colorspace when read from in a shader.
/// - Will convert from linear colorspace to SRGB colorspace when written to in a shader.
#[derive(Clone, Copy, strum_macros::VariantArray, PartialEq, Eq)]
pub enum TextureFormatID {
	F32, Vec2, Vec4,
	U8, U8Vec2, U8Vec4,
	U16, U16Vec2, U16Vec4,
	SrgbU8, SrgbU8Vec2, SrgbU8Vec4,
}

/// Indicates usability as a pixel value in a texture.
pub trait TextureFormat: Clone + Copy + bytemuck::Pod + Default {
	#[doc(hidden)]
	const TEXTURE_ID: TextureFormatID;
}

/// Indicates usability as a pixel value in an SRGB texture.
pub trait SrgbTextureFormat: Clone + Copy + bytemuck::Pod + Default {
	#[doc(hidden)]
	const NO_SRGB_ID: TextureFormatID;
	const SRGB_ID: TextureFormatID;
}

impl<T: SrgbTextureFormat> TextureFormat for T {
	const TEXTURE_ID: TextureFormatID = T::NO_SRGB_ID;
}

/// Indicates usability as a pixel value in an unsigned integer texture.
pub trait UIntTextureFormat: Clone + Copy + bytemuck::Pod + Default {
	#[doc(hidden)]
	const ID: TextureFormatID;
}

macro_rules! impl_tex_format {
	($ty:ty, $id:ident) => {
		impl TextureFormat for $ty {
			const TEXTURE_ID: TextureFormatID = TextureFormatID :: $id;
		}
	};
}

macro_rules! impl_srgb_tex_format {
	($ty:ty, $id:ident, $srgb_id:ident) => {
		impl SrgbTextureFormat for $ty {
			const NO_SRGB_ID: TextureFormatID = TextureFormatID :: $id;
			const SRGB_ID: TextureFormatID = TextureFormatID :: $srgb_id;
		}
	};
}

impl_tex_format!(f32, F32);
impl_tex_format!(Vec2, Vec2);
impl_tex_format!(Vec4, Vec4);

impl_srgb_tex_format!(u8, U8, SrgbU8);
impl_srgb_tex_format!(U8Vec2, U8Vec2, SrgbU8Vec2);
impl_srgb_tex_format!(U8Vec4, U8Vec4, SrgbU8Vec4);

impl_tex_format!(u16, U16);
impl_tex_format!(U16Vec2, U16Vec2);
impl_tex_format!(U16Vec4, U16Vec4);

#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum VertexAttributeID {
	Padding,
	F32, Vec2, Vec3, Vec4,
	U8, U8Vec2, U8Vec4, U8UNorm, U8Vec2UNorm, U8Vec4UNorm,
	U16, U16Vec2, U16Vec4, U16UNorm, U16Vec2UNorm, U16Vec4UNorm,
	U32, U32Vec2, U32Vec4,
}

/// Allows for use as an attribute in a [VertexTy].
pub trait VertexAttribute: Clone + Copy + bytemuck::NoUninit {
	#[doc(hidden)]
	const ID: VertexAttributeID;
}

impl<const N: usize> VertexAttribute for Padding<N> {
	const ID: VertexAttributeID = VertexAttributeID::Padding;
}

macro_rules! impl_vertex_attr {
	($ty:ty, $id:ident) => {
		impl VertexAttribute for $ty {
			const ID: VertexAttributeID = VertexAttributeID:: $id;
		}
	};
}

macro_rules! impl_uint_vertex_attr {
	($v: tt, $vec2:ty, $vec4:ty) => {
		paste::paste! {
			impl_vertex_attr!([< u $v >], [< U $v >]);
			impl_vertex_attr!($vec2, [< U $v Vec2 >]);
			impl_vertex_attr!($vec4, [< U $v Vec4 >]);
		}
	};
}

impl_vertex_attr!(f32, F32);
impl_vertex_attr!(Vec2, Vec2);
impl_vertex_attr!(Vec3, Vec3);
impl_vertex_attr!(Vec4, Vec4);
impl_uint_vertex_attr!(8, U8Vec2, U8Vec4);
impl_uint_vertex_attr!(16, U16Vec2, U16Vec4);
impl_uint_vertex_attr!(32, UVec2, UVec4);


#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UniformAttributeID {
	Padding,
	F32, Vec2, Vec3, Vec4,
	I32, IVec2, IVec3, IVec4,
	U32, UVec2, UVec3, UVec4,
	Mat2, Mat3, Mat4,
}

/// Allows for use as an attribute in a [UniformTy].
pub trait UniformAttribute: Clone + Copy + bytemuck::NoUninit {
	#[doc(hidden)]
	const ALIGNMENT: usize;
	#[doc(hidden)]
	const ID: UniformAttributeID;
}

// SAFETY: idk if this is actually safe yet
unsafe impl<const N: usize> bytemuck::NoUninit for Padding<N> {}

impl<const N: usize> UniformAttribute for Padding<N> {
	const ALIGNMENT: usize = 1;
	const ID: UniformAttributeID = UniformAttributeID::Padding;
}

macro_rules! impl_uniform_attr {
	($ty:ty, $id:ident, $alignment:expr) => {
		impl UniformAttribute for $ty {
			const ALIGNMENT: usize = $alignment;
			const ID: UniformAttributeID = UniformAttributeID:: $id;
		}
	};
}

impl_uniform_attr!(f32, F32, 4);
impl_uniform_attr!(Vec2, Vec2, 4*2);
impl_uniform_attr!(Vec3, Vec3, 4*4);
impl_uniform_attr!(Vec4, Vec4, 4*4);

impl_uniform_attr!(i32, I32, 4);
impl_uniform_attr!(IVec2, IVec2, 4*2);
impl_uniform_attr!(IVec3, IVec3, 4*4);
impl_uniform_attr!(IVec4, IVec4, 4*4);

impl_uniform_attr!(u32, U32, 4);
impl_uniform_attr!(UVec2, UVec2, 4*2);
impl_uniform_attr!(UVec3, UVec3, 4*4);
impl_uniform_attr!(UVec4, UVec4, 4*4);

impl_uniform_attr!(Mat2, Mat2, 8);
impl_uniform_attr!(Mat3, Mat3, 16);
impl_uniform_attr!(Mat4, Mat4, 16);