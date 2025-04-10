pub use glam::*;

#[derive(Clone, Copy)]
pub struct Rect3D {
	pub start: Vec3,
	pub end: Vec3,
}

#[derive(Clone, Copy)]
pub struct URect3D {
	pub start: UVec3,
	pub end: UVec3,
}

impl URect3D {
	pub fn new(start: UVec3, end: UVec3) -> Self {
		Self { start, end }
	}

	pub fn sized(start: UVec3, size: UVec3) -> Self {
		Self { start, end: start + size }
	}

	pub fn size(&self) -> UVec3 {
		self.end - self.start
	}

	pub fn fit_inside(self, bounds: URect3D) -> Self {
		Self { start: bounds.start.max(self.start), end: bounds.end.min(self.end) }
	}
}

#[derive(Clone, Copy, Debug)]
pub struct Rect2D {
	pub start: Vec2,
	pub end: Vec2,
}

pub enum Rect2DAlign {
	TopLeft, TopRight, BottomLeft, BottomRight, Left, Right, Top, Bottom, Center,
}

impl Rect2D {
	pub const UNIT: Self = Self { start: Vec2::ZERO, end: Vec2::ONE };

	pub fn new(start: Vec2, end: Vec2) -> Self {
		Self { start, end }
	}

	pub fn with_extent(start: Vec2, extent: Vec2) -> Self {
		Self { start, end: start + extent }
	}

	pub fn centered(center: Vec2, extent: Vec2) -> Self {
		Self { start: center - extent * 0.5, end: center + extent * 0.5 }
	}

	pub fn translate(self, t: Vec2) -> Self {
		Self { start: self.start + t, end: self.end + t }
	}

	/*
	pub fn aligned_to_point(point: Vec2, extent: Vec2, alignment: Rect2DAlign) -> Self {
		match alignment {
			Rect2DAlign::TopLeft => Self { start: point, end: point + extent },
			Rect2DAlign::TopRight => Self { start: Vec2::new(point.x - extent.x, point.y), end: Vec2::new(point.x, point.y + extent.y)},
			Rect2DAlign::BottomLeft => Self { start: Vec2::new(point.x, point.y - extent.y), end: Vec2::new(point.x + extent.y, point.y)},
			Rect2DAlign::BottomRight => Self { start: point - extent, end: point },
			Rect2DAlign::Left => todo!(),
			Rect2DAlign::Right => todo!(),
			Rect2DAlign::Top => todo!(),
			Rect2DAlign::Bottom => todo!(),
			Rect2DAlign::Center => Self::centered(point, extent),
		}
	}
	*/
}

#[derive(Clone, Copy)]
pub struct URect2D {
	pub start: UVec2,
	pub end: UVec2,
}

impl URect2D {
	pub const UNIT: Self = Self { start: UVec2::ZERO, end: UVec2::ONE };

	pub fn new(start: UVec2, end: UVec2) -> Self {
		Self { start, end }
	}

	pub fn sized(size: UVec2) -> Self {
		Self { start: UVec2::ZERO, end: size }
	}

	pub fn with_start_and_size(start: UVec2, size: UVec2) -> Self {
		Self { start, end: start + size }
	}

	pub fn centered(center: UVec2, extent: UVec2) -> Self {
		Self { start: center - extent / 2, end: center + extent / 2 }
	}

	pub fn translate(self, t: UVec2) -> Self {
		Self { start: self.start + t, end: self.end + t }
	}

	pub fn size(&self) -> UVec2 {
		self.end - self.start
	}

	pub fn area(&self) -> u32 {
		(self.end.x - self.start.x) * (self.end.y * self.start.y)
	}
}