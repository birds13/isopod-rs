use std::{any::{Any, TypeId}, cell::RefCell, ops::Deref, sync::{Arc, Mutex, Weak}};
use qcell::{QCell, QCellOwner, TCell};
use rustc_hash::FxHashMap;

use crate::EngineCtx;

struct ResourceMarker;

pub struct Res<T> {
	inner: Arc<qcell::TCell<ResourceMarker, T>>,
}

pub trait Resource: 'static + Send + Sync + Sized {
	fn load(data: &[u8], ctx: &EngineCtx) -> Result<Self, String>;
	fn default(ctx: &EngineCtx) -> Self;
}

#[derive(PartialEq, Eq, Hash)]
struct RID {
	type_id: TypeId,
	path: String,
}

pub(crate) struct ResourceStorage {
	resources: RefCell<FxHashMap<RID, Weak<dyn Any + Send + Sync>>>,
	defaults: RefCell<FxHashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
	resource_owner: qcell::TCellOwner<ResourceMarker>,
}

impl ResourceStorage {
	pub fn new() -> Self {
		Self {
			resource_owner: qcell::TCellOwner::new(),
			resources: RefCell::new(FxHashMap::default()),
			defaults: RefCell::new(FxHashMap::default()),
		}
	}

	pub fn read<'a, T: Resource>(&'a self, res: &'a Res<T>) -> &'a T {
		res.inner.ro(&self.resource_owner)
	}

	pub fn get<T: Resource>(&self, ctx: &EngineCtx, path: impl Into<String>) -> Res<T> {
		let key = RID { type_id: TypeId::of::<T>(), path: path.into() };
		let existing = self.resources.borrow().get(&key).map(|weak| weak.upgrade()).flatten();
		if let Some(v) = existing {
			Res { inner: v.downcast::<qcell::TCell<ResourceMarker,T>>().unwrap() }
		} else {
			match T::load(&[], ctx) {
				Ok(v) => {
					let res = Res { inner: Arc::new(qcell::TCell::new(v)) };
					let res_weak = Arc::downgrade(&res.inner);
					self.resources.borrow_mut().insert(key, res_weak);
					res
				},
				Err(error) => {
					// will handle error here in the future
					self.default::<T>(ctx)
				},
			}
		}
	}

	pub fn default<T: Resource>(&self, ctx: &EngineCtx) -> Res<T> {
		let key = TypeId::of::<T>();
		let existing = self.defaults.borrow().get(&key).map(|v| v.clone());
		if let Some(v) = existing {
			Res { inner: v.downcast::<qcell::TCell<ResourceMarker,T>>().unwrap() }
		} else {
			let v = T::default(ctx);
			let arc = Arc::new(qcell::TCell::new(v));
			self.defaults.borrow_mut().insert(key, arc.clone());
			Res { inner: arc }
		}
	}
}