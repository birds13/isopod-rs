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
#[derive(Clone, Copy, PartialEq)]
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
pub struct MaterialInner {
	pub id: Cell<usize>,
	pub attributes: Vec<MaterialAttributeRefID>,
}

pub struct Material<'frame, T: MaterialTy> {
	pub(crate) inner: MaterialInner,
	pub(crate) _data: PhantomData<(&'frame GfxCtx, T)>,
}

impl<'frame, T: MaterialTy> Material<'frame, T> {
	#[doc(hidden)]
	pub unsafe fn from_ref_ids(_ctx: &'frame GfxCtx, r: Vec<MaterialAttributeRefID>) -> Self {
		Self {
			inner: MaterialInner { id: Cell::new(ID_NONE), attributes: r },
			_data: PhantomData,
		}
	}
}

pub trait MaterialSet {
	type Set<'frame>;
	#[doc(hidden)]
	fn iter<'a>(set: &'a Self::Set<'a>) -> impl Iterator<Item = &'a MaterialInner>;
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>>;
}

impl MaterialSet for () {
	type Set<'frame> = ();
	fn iter<'a>(_: &'a Self::Set<'a>) -> impl Iterator<Item = &'a MaterialInner> { std::iter::empty() }
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> { Vec::with_capacity(0) }
}
impl<T: MaterialTy> MaterialSet for T {
	type Set<'frame> = Material<'frame, T>;
	fn iter<'a>(set: &'a Self::Set<'a>) -> impl Iterator<Item = &'a MaterialInner> {
		std::iter::once(&set.inner)
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T::layout()]
	}
}

impl<T0: MaterialTy, T1: MaterialTy> MaterialSet for (T0,T1) {
	type Set<'frame> = (Material<'frame, T0>, Material<'frame, T1>);
	fn iter<'a>(set: &'a Self::Set<'a>) -> impl Iterator<Item = &'a MaterialInner> {
		[&set.0.inner,&set.1.inner].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout()]
	}
}
impl<T0: MaterialTy, T1: MaterialTy, T2: MaterialTy> MaterialSet for (T0,T1,T2) {
	type Set<'frame> = (Material<'frame, T0>, Material<'frame, T1>, Material<'frame, T2>);
	fn iter<'a>(set: &'a Self::Set<'a>) -> impl Iterator<Item = &'a MaterialInner> {
		[&set.0.inner,&set.1.inner,&set.2.inner].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout(),T2::layout()]
	}
}
impl<T0: MaterialTy, T1: MaterialTy, T2: MaterialTy, T3: MaterialTy> MaterialSet for (T0,T1,T2,T3) {
	type Set<'frame> = (Material<'frame, T0>, Material<'frame, T1>, Material<'frame, T2>, Material<'frame, T3>);
	fn iter<'a>(set: &'a Self::Set<'a>) -> impl Iterator<Item = &'a MaterialInner> {
		[&set.0.inner,&set.1.inner,&set.2.inner,&set.3.inner].into_iter()
	}
	fn layouts() -> Vec<StructLayout<MaterialAttributeID>> {
		vec![T0::layout(),T1::layout(),T2::layout(),T3::layout()]
	}
}