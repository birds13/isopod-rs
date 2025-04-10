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

#[doc(hidden)]
#[derive(Clone, Copy)]
pub(crate) enum NormalizationID {
	None, Srgb, MinusOneToOne, ZeroToOne,
}

/// A marker trait that modifies how values should be interpreted in a shader.
pub trait Normalization: Clone + Copy + 'static {}

/// A [Normalization] that maps between SRGB and linear colorspaces.
/// 
/// This behaves similarly to [ZeroToOne] but converts from SRGB colorspace to linear when read from and converts from linear to SRGB colorspace when written to.
#[derive(Clone, Copy)]
pub struct Srgb;
impl Normalization for Srgb {}
/// A [Normalization] that maps values between zero and one.
/// 
/// Zero corresponds to zero and one to the maximum value for the type.
#[derive(Clone, Copy)]
pub struct ZeroToOne;
impl Normalization for ZeroToOne {}
/// A [Normalization] that maps values between minus one and one.
/// 
/// Minus one corresponds to the minimum value for the type and one corresponds to the maximum value.
#[derive(Clone, Copy)]
pub struct MinusOneToOne;
impl Normalization for MinusOneToOne {}

/// A wrapper type that changes how the underlying value `v` is interpreted in a shader.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Normalized<T, N: Normalization> {
	pub v: T,
	_data: PhantomData<N>,
}
impl<T: Default, N: Normalization> Default for Normalized<T,N> {
	fn default() -> Self {
		Self { v: Default::default(), _data: PhantomData }
	}
}
impl<T, N: Normalization> Normalized<T,N> {
	pub fn new(v: T) -> Self {
		Self { v, _data: PhantomData }
	}
}

// SAFETY: this is a wrapper type for a value of type 'T' which must be Zeroable because its layout is constant and the same as T
unsafe impl<T: bytemuck::Zeroable, N: Normalization> bytemuck::Zeroable for Normalized<T,N> {}

// SAFETY: this is a wrapper type for a value of type 'T' which must be Pod because its layout is constant and the same as T
unsafe impl<T: bytemuck::NoUninit + bytemuck::Zeroable, N: Normalization> bytemuck::Pod for Normalized<T,N> {}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub(crate) enum TextureAttributeID {
	F32, Vec2, Vec4,
	U8, U8Vec2, U8Vec4,
	U16, U16Vec2, U16Vec4,
	U32,
}

/// Indicates usability as a pixel value in a texture.
pub trait TextureAttribute: Clone + Copy + bytemuck::Pod + Default {
	#[doc(hidden)]
	const IDS: (TextureAttributeID, NormalizationID);
}

macro_rules! impl_texture_attr {
	($ty:ty, $id:ident) => {
		impl TextureAttribute for $ty {
			const IDS: (TextureAttributeID, NormalizationID) = (TextureAttributeID:: $id, NormalizationID::None);
		}
	};
}

macro_rules! impl_norm_srgb_texture_attr {
	($ty:ty, $id:ident) => {
		impl TextureAttribute for Normalized<$ty, Srgb> {
			const IDS: (TextureAttributeID, NormalizationID) = (TextureAttributeID:: $id, NormalizationID::Srgb);
		}
	};
}

macro_rules! impl_norm_other_texture_attr {
	($ty:ty, $id:ident) => {
		impl TextureAttribute for Normalized<$ty, ZeroToOne> {
			const IDS: (TextureAttributeID, NormalizationID) = (TextureAttributeID:: $id, NormalizationID::ZeroToOne);
		}
		impl TextureAttribute for Normalized<$ty, MinusOneToOne> {
			const IDS: (TextureAttributeID, NormalizationID) = (TextureAttributeID:: $id, NormalizationID::MinusOneToOne);
		}
	};
}



impl_texture_attr!(f32, F32);
impl_texture_attr!(Vec2, Vec2);
impl_texture_attr!(Vec4, Vec4);
impl_texture_attr!(u32, U32);

impl_texture_attr!(u8, U8);
impl_norm_srgb_texture_attr!(u8, U8);
impl_norm_other_texture_attr!(u8, U8);

impl_texture_attr!(U8Vec2, U8Vec2);
impl_norm_srgb_texture_attr!(U8Vec2, U8Vec2);
impl_norm_other_texture_attr!(U8Vec2, U8Vec2);

impl_texture_attr!(U8Vec4, U8Vec4);
impl_norm_srgb_texture_attr!(U8Vec4, U8Vec4);
impl_norm_other_texture_attr!(U8Vec4, U8Vec4);

impl_texture_attr!(u16, U16);
impl_norm_other_texture_attr!(u16, U16);

impl_texture_attr!(U16Vec2, U16Vec2);
impl_norm_other_texture_attr!(U16Vec2, U16Vec2);

impl_texture_attr!(U16Vec4, U8Vec4);
impl_norm_other_texture_attr!(U16Vec4, U16Vec4);

#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum VertexAttributeID {
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

macro_rules! impl_uint_unorm_vertex_attr {
	($v: tt, $vec2:ty, $vec4:ty) => {
		paste::paste! {
			impl_vertex_attr!(Normalized<[< u $v >], ZeroToOne>, [< U $v UNorm >]);
			impl_vertex_attr!(Normalized<$vec2, ZeroToOne>, [< U $v Vec2UNorm >]);
			impl_vertex_attr!(Normalized<$vec4, ZeroToOne>, [< U $v Vec4UNorm >]);	
		}
	};
}

impl_vertex_attr!(f32, F32);
impl_vertex_attr!(Vec2, Vec2);
impl_vertex_attr!(Vec3, Vec3);
impl_vertex_attr!(Vec4, Vec4);
impl_uint_vertex_attr!(8, U8Vec2, U8Vec4);
impl_uint_unorm_vertex_attr!(8, U8Vec2, U8Vec4);
impl_uint_vertex_attr!(16, U16Vec2, U16Vec4);
impl_uint_unorm_vertex_attr!(16, U16Vec2, U16Vec4);
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