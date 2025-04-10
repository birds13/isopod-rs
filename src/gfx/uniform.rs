#![allow(private_interfaces)]
use std::{marker::PhantomData, sync::Arc};

use super::*;

// SAFETY: it must be safe to convert &T to &[u8]
pub trait UniformTy: Copy + Clone {
	#[doc(hidden)]
	fn layout() -> StructLayout<UniformAttributeID>;
	fn into_bytes(&self) -> &[u8] {
		unsafe {
			std::slice::from_raw_parts(std::mem::transmute(std::slice::from_ref(self).as_ptr()), std::mem::size_of::<Self>())
		}
	}
}

impl UniformTy for () {
	fn layout() -> StructLayout<UniformAttributeID> { StructLayout::unit() }
}

impl<T: UniformAttribute> UniformTy for T {
	fn layout() -> StructLayout<UniformAttributeID> {
		StructLayout { attributes: vec![
			StructAttribute { attribute: T::ID, offset: 0, name: "value" }
		], size: std::mem::size_of::<T>() }
	}
}

impl<T: UniformTy> MaterialAttribute for T {
	fn id() -> MaterialAttributeID {
		MaterialAttributeID::Uniform(T::layout())
	}
}

pub struct ImmediateUniformBuffer<'frame, T: UniformTy> {
	pub(crate) start: usize,
	pub(crate) len: usize,
	pub(crate) _data: PhantomData<(&'frame (), T)>,
}

impl<'frame, T: UniformTy> MaterialAttributeRef<T> for ImmediateUniformBuffer<'frame, T> {
	fn id(&self) -> MaterialAttributeRefID {
		MaterialAttributeRefID::ImmediateUniform { start: self.start, len: self.len }
	}
}

pub struct UniformBuffer<T: UniformTy> {
	pub(crate) id: usize,
	pub(crate) _rc: Arc<()>,
	pub(crate) _data: PhantomData<T>,
}

impl<T: UniformTy> MaterialAttributeRef<T> for UniformBuffer<T> {
	fn id(&self) -> MaterialAttributeRefID {
		MaterialAttributeRefID::Uniform { id: self.id }
	}
}