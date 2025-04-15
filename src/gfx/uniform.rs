#![allow(private_interfaces)]
use std::{marker::PhantomData, sync::Arc};

use super::*;

// SAFETY: it must be safe to convert &T to &[u8]
pub unsafe trait UniformTy: Copy + Clone {
	#[doc(hidden)]
	fn layout() -> StructLayout<UniformAttributeID>;
	fn into_bytes(&self) -> &[u8] {
		unsafe {
			std::slice::from_raw_parts(std::mem::transmute(std::slice::from_ref(self).as_ptr()), std::mem::size_of::<Self>())
		}
	}
}

unsafe impl UniformTy for () {
	fn layout() -> StructLayout<UniformAttributeID> { StructLayout::unit() }
}

unsafe impl<T: UniformAttribute> UniformTy for T {
	fn layout() -> StructLayout<UniformAttributeID> {
		StructLayout { attributes: vec![
			StructAttribute { attribute: T::ID, offset: 0, name: "value" }
		], size: std::mem::size_of::<T>() }
	}
}

#[derive(Clone)]
pub(crate) enum UniformBufferInner<'a> {
	Immediate {
		start: usize,
		len: usize,
		_lifetime: PhantomData<&'a ()>,
	},
	Resource {
		id: usize,
		_rc: Arc<()>,
	}
}

#[derive(Clone)]
pub struct UniformBuffer<'a, T: UniformTy> {
	pub(crate) inner: UniformBufferInner<'a>,
	pub(crate) _data: PhantomData<T>,
}

pub type UniformBufferRes<T> = UniformBuffer<'static, T>;

impl<'a, T: UniformTy> MaterialAttribute for UniformBuffer<'a, T> {
	fn id() -> MaterialAttributeID {
		MaterialAttributeID { inner: MaterialAttributeIDInner::Uniform(T::layout()) }
	}

	fn ref_id(&self) -> MaterialAttributeRefID {
		match self.inner {
			UniformBufferInner::Immediate { start, len, .. } => MaterialAttributeRefID {
				inner: MaterialAttributeRefIDInner::ImmediateUniform { start, len }
			},
			UniformBufferInner::Resource { id, .. } => MaterialAttributeRefID {
				inner: MaterialAttributeRefIDInner::Uniform { id }
			},
		}
	}

	fn into_any<'b>(&'b self) -> MaterialAttributeAny<'b> {
		MaterialAttributeAny { inner: MaterialAttributeAnyInner::Uniform(self.inner.clone()) }
	}
}