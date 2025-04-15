use std::{any::Any, marker::PhantomData};

use super::*;

pub(crate) const MAX_MATERIALS: usize = 4;

#[doc(hidden)]
#[derive(Clone)]
pub enum MaterialAttributeIDInner {
	Texture2D,
	Sampler,
	Uniform(StructLayout<UniformAttributeID>),
}

#[doc(hidden)]
#[derive(Clone)]
pub struct MaterialAttributeID {
	pub(crate) inner: MaterialAttributeIDInner,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum MaterialAttributeRefIDInner {
	Texture2D { id: usize },
	Sampler { id: usize },
	Uniform { id: usize },
	FramebufferColor { id: usize },
	FramebufferDepth { id: usize },
	ImmediateUniform { start: usize, len: usize },
}

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct  MaterialAttributeRefID {
	pub(crate) inner: MaterialAttributeRefIDInner,
}

#[derive(Clone)]
pub(crate) enum MaterialAttributeAnyInner<'a> {
	Texture2D(GPUTexture2D),
	Sampler(Sampler),
	Uniform(UniformBufferInner<'a>),
}

#[doc(hidden)]
#[derive(Clone)]
pub struct MaterialAttributeAny<'a> {
	pub(crate) inner: MaterialAttributeAnyInner<'a>,
}

pub trait MaterialAttribute {
	#[doc(hidden)]
	fn id() -> MaterialAttributeID;
	#[doc(hidden)]
	fn ref_id(&self) -> MaterialAttributeRefID;
	#[doc(hidden)]
	fn into_any<'a>(&'a self) -> MaterialAttributeAny<'a>;
}

pub unsafe trait MaterialTy: Sized {
	type Refs<'a>: MaterialRefs<'a ,Self>;
	#[doc(hidden)]
	fn layout<'a>() -> StructLayout<MaterialAttributeID>;
}

pub unsafe trait MaterialRefs<'a, T: MaterialTy> {
	#[doc(hidden)]
	fn into_refs(&self) -> Vec<MaterialAttributeRefID>;
	#[doc(hidden)]
	fn into_any(self) -> Vec<MaterialAttributeAny<'a>>;
}

pub(crate) struct MaterialInner<'a> {
	pub id: usize,
	pub attributes: Vec<MaterialAttributeRefID>,
	pub _data: PhantomData<&'a ()>,
}

#[doc(hidden)]
pub struct MaterialRaw<'a> {
	pub(crate) inner: MaterialInner<'a>,
}

pub struct MaterialCfg<'a, T: MaterialTy> {
	pub(crate) raw: MaterialRaw<'a>,
	pub(crate) _data: PhantomData<T>,
}

pub trait MaterialSet {
	type Materials<'frame>;
	#[doc(hidden)]
	fn iter<'a>(set: &'a Self::Materials<'a>) -> impl Iterator<Item = &'a MaterialRaw<'a>>;
	#[doc(hidden)]
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>>;
}

impl MaterialSet for () {
	type Materials<'frame> = ();
	fn iter<'a>(_: &'a Self::Materials<'a>) -> impl Iterator<Item = &'a MaterialRaw<'a>> { std::iter::empty() }
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> { Vec::with_capacity(0) }
}
impl<T: MaterialTy + 'static> MaterialSet for T {
	type Materials<'frame> = &'frame MaterialCfg<'frame, T>;
	fn iter<'a>(set: &'a Self::Materials<'a>) -> impl Iterator<Item = &'a MaterialRaw<'a>> {
		std::iter::once(&set.raw)
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T::layout()]
	}
}

impl<T0: MaterialTy + 'static, T1: MaterialTy + 'static> MaterialSet for (T0,T1) {
	type Materials<'frame> = (&'frame MaterialCfg<'frame, T0>, &'frame MaterialCfg<'frame, T1>);
	fn iter<'a>(set: &'a Self::Materials<'a>) -> impl Iterator<Item = &'a MaterialRaw<'a>> {
		[&set.0.raw,&set.1.raw].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout()]
	}
}
impl<T0: MaterialTy + 'static, T1: MaterialTy + 'static, T2: MaterialTy + 'static> MaterialSet for (T0,T1,T2) {
	type Materials<'frame> = (&'frame MaterialCfg<'frame, T0>, &'frame MaterialCfg<'frame, T1>, &'frame MaterialCfg<'frame, T2>);
	fn iter<'a>(set: &'a Self::Materials<'a>) -> impl Iterator<Item = &'a MaterialRaw<'a>> {
		[&set.0.raw,&set.1.raw,&set.2.raw].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout(),T2::layout()]
	}
}
impl<T0: MaterialTy + 'static, T1: MaterialTy + 'static, T2: MaterialTy + 'static, T3: MaterialTy + 'static> MaterialSet for (T0,T1,T2,T3) {
	type Materials<'frame> = (&'frame MaterialCfg<'frame, T0>, &'frame MaterialCfg<'frame, T1>, &'frame MaterialCfg<'frame, T2>, &'frame MaterialCfg<'frame, T3>);
	fn iter<'a>(set: &'a Self::Materials<'a>) -> impl Iterator<Item = &'a MaterialRaw<'a>> {
		[&set.0.raw,&set.1.raw,&set.2.raw,&set.3.raw].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout(),T2::layout(),T3::layout()]
	}
}

#[macro_export]
macro_rules! material_ty {
	($crate_name:ident | $name:ident { $( $attribute_name:ident : $attribute_ty:ty ),+ $(,)* }) => {
		paste::paste! {
			pub struct $name;
			pub struct [<$name Refs>] <'a> {
				$( $attribute_name: &'a $attribute_ty, )+
			}
			unsafe impl<'a> $crate_name::gfx::MaterialRefs<'a, $name> for [<$name Refs>] <'a> {
				fn into_refs(&self) -> Vec<$crate_name::gfx::MaterialAttributeRefID> {
					let mut __v = Vec::new();
					$(__v.push($crate_name::gfx::MaterialAttribute::ref_id(self. $attribute_name));)+
					__v
				}
				fn into_any(self) -> Vec<MaterialAttributeAny<'a>> {
					let mut __v = Vec::new();
					$(__v.push($crate_name::gfx::MaterialAttribute::into_any(self. $attribute_name));)+
					__v
				}
			}
			unsafe impl $crate_name::gfx::MaterialTy for $name {
				type Refs<'a> = [<$name Refs>] <'a>;
				fn layout<'a>() -> $crate_name::gfx::StructLayout<$crate_name::gfx::MaterialAttributeID> {
					let mut __v = Vec::new();
					$(__v.push($crate_name::gfx::StructAttribute {
						name: stringify!($attribute_name),
						offset: 0,
						attribute: <$attribute_ty as $crate_name::gfx::MaterialAttribute>::id(),
					});)+
					$crate_name::gfx::StructLayout { size: __v.len(), attributes: __v}
				}
			}
		}
	};
	($name:ident { $( $attribute_name:ident : $attribute_ty:ty ),+ $(,)* }) => {
		material_ty!(isopod | $name { $( $attribute_name : $attribute_ty ),+ } );
	};
}
