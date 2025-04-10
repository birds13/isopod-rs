use std::{cell::UnsafeCell, collections::VecDeque, sync::{Arc, Weak}};

pub struct BufferCell<T> {
	inner: UnsafeCell<Vec<T>>,
}

impl<T> BufferCell<T> {
	
	pub fn new() -> Self {
		Self {inner: UnsafeCell::new(Vec::new())}
	}

	pub fn get_mut(&mut self) -> &mut Vec<T> {
		self.inner.get_mut()
	}

	pub fn len(&self) -> usize {
		let inner = unsafe {&mut *self.inner.get()};
		inner.len()
	}

	pub fn push(&self, v: T) {
		let inner = unsafe {&mut *self.inner.get()};
		inner.push(v);
	}
}


impl<T> Default for BufferCell<T> {
	fn default() -> Self {
		Self::new()
	}
}

pub struct BufferDequeCell<T> {
	inner: UnsafeCell<VecDeque<T>>,
}

impl<T> BufferDequeCell<T> {
	
	pub fn new() -> Self {
		Self {inner: UnsafeCell::new(VecDeque::new())}
	}

	pub fn get_mut(&mut self) -> &mut VecDeque<T> {
		self.inner.get_mut()
	}

	pub fn push(&self, v: T) {
		let inner = unsafe {&mut *self.inner.get()};
		inner.push_back(v);
	}
}


impl<T> Default for BufferDequeCell<T> {
	fn default() -> Self {
		Self::new()
	}
}

pub struct ByteBufferCell {
	inner: UnsafeCell<Vec<u8>>,
}

impl ByteBufferCell {
	pub fn new() -> Self {
		Self { inner: UnsafeCell::new(Vec::new()) }
	}

	pub fn get_mut(&mut self) -> &mut Vec<u8> {
		self.inner.get_mut()
	}

	pub fn push_bytes(&self, bytes: &[u8]) {
		let inner = unsafe {&mut *self.inner.get()};
		inner.extend_from_slice(bytes);
	}

	pub fn align_to(&self, alignment: usize) {
		let inner = unsafe {&mut *self.inner.get()};
		for _ in inner.len()..align_up(inner.len(), alignment) {
			inner.push(0);
		}
	}

	pub fn len(&self) -> usize {
		let inner = unsafe {& *self.inner.get()};
		inner.len()
	}
}

impl Default for ByteBufferCell {
	fn default() -> Self { Self::new() }
}

#[derive(Default)]
struct IDVecInner<T> {
	vec: Vec<Option<T>>,
	free: Vec<usize>,
}

pub struct IDVec<T> {
	inner: UnsafeCell<IDVecInner<T>>,
}

impl<T> IDVec<T> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert(&self, v: T) -> usize {
		let inner = unsafe {&mut *self.inner.get()};
		if let Some(id) = inner.free.pop() {
			inner.vec[id] = Some(v);
			id
		} else {
			inner.vec.push(Some(v));
			inner.vec.len()-1
		}
	}

	#[allow(unused)]
	pub fn remove(&mut self, id: usize) -> Option<T> {
		let inner = self.inner.get_mut();
		inner.free.push(id);
		inner.vec[id].take()
	}

	#[allow(unused)]
	pub fn get(&mut self, id: usize) -> Option<&mut T> {
		self.inner.get_mut().vec.get_mut(id).map(|o| o.as_mut()).flatten()
	}
}

impl<T> Default for IDVec<T> {
	fn default() -> Self {
		Self { inner: UnsafeCell::new(IDVecInner { vec: vec![], free: vec![] }) }
	}
}

pub struct IDArenaCell<T> {
	inner: IDVec<Weak<T>>,
}

impl<T> IDArenaCell<T> {
	pub fn new() -> Self { 
		Self::default()
	}

	pub fn insert(&self, v: &Arc<T>) -> usize {
		self.inner.insert(Arc::downgrade(v))
	}

	pub fn remove_unused(&mut self) -> Vec<usize> {
		let inner = self.inner.inner.get_mut();
		let mut removed = vec![];
		for (i, v) in inner.vec.iter_mut().enumerate() {
			if v.as_ref().map_or(false, |w| w.strong_count() == 0) {
				*v = None;
				removed.push(i);
				inner.free.push(i);
			}
		}
		removed
	}
}

impl<T> Default for IDArenaCell<T> {
	fn default() -> Self {
		Self { inner: IDVec::new() }
	}
}

pub struct SparseVec<T> {
	pub vec: Vec<Option<T>>,
}

impl<T> SparseVec<T> {
	pub fn new() -> Self {
		Self { vec: vec![] }
	}

	pub fn insert(&mut self, index: usize, v: T) {
		while index >= self.vec.len() {
			self.vec.push(None);
		}
		self.vec[index] = Some(v);
	}

	#[allow(unused)]
	pub fn remove(&mut self, index: usize) -> Option<T> {
		self.vec[index].take()
	}

	pub fn get(&self, index: usize) -> Option<&T> {
		self.vec.get(index).and_then(|o| o.as_ref())
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
		self.vec.get_mut(index).and_then(|o| o.as_mut())
	}

	#[allow(unused)]
	pub fn into_iter(self) -> impl std::iter::Iterator<Item = T> {
		self.vec.into_iter().flatten()
	}

	pub fn iter(&self) -> impl std::iter::Iterator<Item = &T> {
		self.vec.iter().filter_map(|v| v.as_ref())
	}

	pub fn iter_mut(&mut self) -> impl std::iter::Iterator<Item = &mut T> {
		self.vec.iter_mut().filter_map(|v| v.as_mut())
	}
}

pub const ID_NONE: usize = usize::MAX;

pub const fn align_up(v: usize, multiple: usize) -> usize {
	if multiple == 0 {
		v
	} else {
		let remainder = v % multiple;
		if remainder == 0 {
			v
		} else {
			v + multiple - remainder
		}
	}
}