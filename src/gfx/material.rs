#![allow(private_interfaces)]
use std::marker::PhantomData;

use super::*;

pub(crate) const MAX_MATERIALS: usize = 4;

#[doc(hidden)]
#[derive(Clone)]
pub enum MaterialAttributeID {
	Texture2D,
	Sampler,
	Uniform(StructLayout<UniformAttributeID>),
}

pub trait MaterialAttribute {
	#[doc(hidden)]
	fn id() -> MaterialAttributeID;
}

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MaterialAttributeRefID {
	Texture2D { id: usize },
	Sampler { id: usize },
	Uniform { id: usize },
	FrameBuffer { id: usize },
	ImmediateUniform { start: usize, len: usize },
}

pub trait MaterialAttributeRef<T: MaterialAttribute> {
	#[doc(hidden)]
	fn id(&self) -> MaterialAttributeRefID;
}

pub trait MaterialTy {
	#[doc(hidden)]
	fn layout() -> StructLayout<MaterialAttributeID>;
}

#[doc(hidden)]
pub(crate) struct MaterialCfgRaw {
	pub id: usize,
	pub attributes: Vec<MaterialAttributeRefID>,
}

pub struct MaterialCfg<'frame, T: MaterialTy> {
	pub(crate) raw: MaterialCfgRaw,
	pub(crate) _data: PhantomData<(&'frame GfxCtx, T)>,
}

impl<'frame, T: MaterialTy> MaterialCfg<'frame, T> {
	#[doc(hidden)]
	pub unsafe fn from_ref_ids(_ctx: &'frame GfxCtx, id: usize, r: Vec<MaterialAttributeRefID>) -> Self {
		Self {
			raw: MaterialCfgRaw { id, attributes: r },
			_data: PhantomData,
		}
	}
}

pub trait MaterialSet {
	type Cfgs<'frame>;
	#[doc(hidden)]
	fn iter<'a>(set: &'a Self::Cfgs<'a>) -> impl Iterator<Item = &'a MaterialCfgRaw>;
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>>;
}

impl MaterialSet for () {
	type Cfgs<'frame> = ();
	fn iter<'a>(_: &'a Self::Cfgs<'a>) -> impl Iterator<Item = &'a MaterialCfgRaw> { std::iter::empty() }
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> { Vec::with_capacity(0) }
}
impl<T: MaterialTy + 'static> MaterialSet for T {
	type Cfgs<'frame> = &'frame MaterialCfg<'frame, T>;
	fn iter<'a>(set: &'a Self::Cfgs<'a>) -> impl Iterator<Item = &'a MaterialCfgRaw> {
		std::iter::once(&set.raw)
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T::layout()]
	}
}

impl<T0: MaterialTy + 'static, T1: MaterialTy + 'static> MaterialSet for (T0,T1) {
	type Cfgs<'frame> = (&'frame MaterialCfg<'frame, T0>, &'frame MaterialCfg<'frame, T1>);
	fn iter<'a>(set: &'a Self::Cfgs<'a>) -> impl Iterator<Item = &'a MaterialCfgRaw> {
		[&set.0.raw,&set.1.raw].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout()]
	}
}
impl<T0: MaterialTy + 'static, T1: MaterialTy + 'static, T2: MaterialTy + 'static> MaterialSet for (T0,T1,T2) {
	type Cfgs<'frame> = (&'frame MaterialCfg<'frame, T0>, &'frame MaterialCfg<'frame, T1>, &'frame MaterialCfg<'frame, T2>);
	fn iter<'a>(set: &'a Self::Cfgs<'a>) -> impl Iterator<Item = &'a MaterialCfgRaw> {
		[&set.0.raw,&set.1.raw,&set.2.raw].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout(),T2::layout()]
	}
}
impl<T0: MaterialTy + 'static, T1: MaterialTy + 'static, T2: MaterialTy + 'static, T3: MaterialTy + 'static> MaterialSet for (T0,T1,T2,T3) {
	type Cfgs<'frame> = (&'frame MaterialCfg<'frame, T0>, &'frame MaterialCfg<'frame, T1>, &'frame MaterialCfg<'frame, T2>, &'frame MaterialCfg<'frame, T3>);
	fn iter<'a>(set: &'a Self::Cfgs<'a>) -> impl Iterator<Item = &'a MaterialCfgRaw> {
		[&set.0.raw,&set.1.raw,&set.2.raw,&set.3.raw].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout(),T2::layout(),T3::layout()]
	}
}

#[macro_export]
macro_rules! material_ty {
	($crate_name:ident | $name:ident { $( $attribute_name:ident : $attribute_ty:ty ),+ $(,)* }) => {
		pub struct $name;
		impl $crate_name::gfx::MaterialTy for $name {
			fn layout() -> $crate_name::gfx::StructLayout<$crate_name::gfx::MaterialAttributeID> {
				let mut v = Vec::new();
				$(v.push($crate_name::gfx::StructAttribute {
					name: stringify!($attribute_name),
					offset: 0,
					attribute: <$attribute_ty as $crate_name::gfx::MaterialAttribute>::id(),
				});)+
				$crate_name::gfx::StructLayout { size: v.len(), attributes: v}
			}
		}
		impl $name {
			fn cfg<'frame>(
				ctx: &'frame $crate_name::gfx::GfxCtx,
				$( $attribute_name: &impl $crate_name::gfx::MaterialAttributeRef<$attribute_ty>, )+
			) -> $crate_name::gfx::MaterialCfg<'frame, Self> {
				let mut __v = Vec::new();
				let __id = ctx.unique_id();
				$(__v.push($attribute_name.id());)+
				unsafe { $crate_name::gfx::MaterialCfg::from_ref_ids(ctx, __id, __v) }
			}
		}
	};
	($name:ident { $( $attribute_name:ident : $attribute_ty:ty ),+ $(,)* }) => {
		pub struct $name;
		impl ::isopod::gfx::MaterialTy for $name {
			fn layout() -> ::isopod::gfx::StructLayout<::isopod::gfx::MaterialAttributeID> {
				let mut v = Vec::new();
				$(v.push(::isopod::gfx::StructAttribute {
					name: stringify!($attribute_name),
					offset: 0,
					attribute: <$attribute_ty as ::isopod::gfx::MaterialAttribute>::id(),
				});)+
				::isopod::gfx::StructLayout { size: v.len(), attributes: v}
			}
		}
		impl $name {
			fn cfg<'frame>(
				ctx: &'frame ::isopod::gfx::GfxCtx,
				$( $attribute_name: &impl ::isopod::gfx::MaterialAttributeRef<$attribute_ty>, )+
			) -> ::isopod::gfx::MaterialCfg<'frame, Self> {
				let mut __v = Vec::new();
				let __id = ctx.unique_id();
				$(__v.push($attribute_name.id());)+
				unsafe { ::isopod::gfx::MaterialCfg::from_ref_ids(ctx, __id, __v) }
			}
		}
	};
}
