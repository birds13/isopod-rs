use rustc_hash::FxHashMap;

use crate::math::*;

use super::*;


/// Used to construct sprite atlases using [`pack_sprite_atlas`] or [`pack_sprite_atlas_array`].
pub struct SpriteSlice<'texture, Key: std::hash::Hash + PartialEq + Eq, T: TextureAttribute> {
	/// Will be used as a key in the generated hashmap when packing as an atlas.
	pub key: Key,
	/// Source texture.
	pub texture: &'texture TextureData<T>,
	/// Source texture layer.
	/// Should be `0` if the texture only has 1 layer.
	pub layer: u32,
	/// Source texture rect.
	pub rect: URect2D,
}

/// CPU modifiable texture.
/// 
/// You can turn this into a GPU usable texture by calling either:
/// - [`create_texture2d`](GfxCtx::create_texture2d) to create a [`Texture2D`].
/// - TODO: other formats
pub struct TextureData<T: TextureAttribute> {
	pub(crate) pixels: Vec<T>,
	size: UVec3,
}

pub(crate) struct TextureDataBytes {
	pub bytes: Vec<u8>,
	pub attribute: (TextureAttributeID, NormalizationID),
	pub size: UVec3,
}

impl<T: TextureAttribute> TextureData<T> {
	/// Create from a set of encoded bytes matching the [`TextureAttribute`] for this texture.
	/// 
	/// Will fail (return [None]) if the number of bytes does not match the specified size.
	pub fn new_from_bytes(bytes: Vec<u8>, size: UVec3) -> Option<Self> {
		let len = size.x as usize * size.y as usize * size.z as usize;
		let pixels = bytemuck::cast_slice::<u8, T>(&bytes);
		if pixels.len() != len {
			None
		} else {
			Some(Self { pixels: pixels.to_vec(), size })
		}
	}

	/// Creates a new empty texture with the given size.
	/// 
	/// Empty means the [default](Default::default) value of [`TextureAttribute`] (probably zeros).
	pub fn new_empty(size: UVec3) -> Self {
		let len = size.x as usize * size.y as usize * size.z as usize;
		let mut vec = Vec::with_capacity(len);
		let default = T::default();
		for _ in 0..len {
			vec.push(default);
		}
		Self{ pixels: vec, size }
	}

	pub(crate) fn into_bytes(self) -> TextureDataBytes {
		TextureDataBytes { bytes: bytemuck::cast_slice(&self.pixels).to_vec(), attribute: T::IDS, size: self.size }
	}

	/// Returns size of the texture.
	pub fn size(&self) -> UVec3 {
		self.size
	}

	/// Returns size of the texture excluding the z component.
	pub fn size_2d(&self) -> UVec2 {
		UVec2::new(self.size.x, self.size.y)
	}

	/// Fills a rectangle with the given value.
	pub fn blit_rect(&mut self, mut rect: URect3D, value: T) {
		rect = rect.fit_inside(URect3D::new(UVec3::ZERO, self.size));
		let size = rect.size();
		for z in rect.start.z..rect.end.z {
			for y in rect.start.y..rect.end.y {
				let offset = (z * size.x * size.y + y * size.x) as usize;
				for x in rect.start.x as usize..rect.end.x as usize {
					self.pixels[offset + x] = value;
				}
			}
		}
	}

	/// Fills a rectangle taking data from `src`.
	pub fn blit_from(&mut self, src: &Self, mut rect_size: UVec3, mut src_offset: UVec3, mut dst_offset: UVec3) {
		let src_rect = URect3D::sized(src_offset, rect_size).fit_inside(URect3D::new(UVec3::ZERO, src.size));
		let dst_rect = URect3D::sized(dst_offset, rect_size).fit_inside(URect3D::new(UVec3::ZERO, self.size));
		rect_size = src_rect.size().min(dst_rect.size());
		src_offset = src_rect.start;
		dst_offset = dst_rect.start;
		for z in 0..rect_size.z {
			for y in 0..rect_size.y {
				let src_offset = ((z+src_offset.z)*(src.size.x*src.size.y) + (y+src_offset.y)*(src.size.x) + src_offset.x) as usize;
				let dst_offset = ((z+dst_offset.z)*(self.size.x*self.size.y) + (y+dst_offset.y)*(self.size.x) + dst_offset.x) as usize;
				self.pixels[dst_offset..dst_offset+rect_size.x as usize].copy_from_slice(&src.pixels[src_offset..src_offset+rect_size.x as usize]);
			}
		}
	}

	/// Creates a [`SpriteSlice`] for use with [`pack_sprite_atlas`] or [`pack_sprite_atlas_array`].
	pub fn sprite_slice<'texture, Key: std::hash::Hash + PartialEq + Eq>(&'texture self, key: Key, rect: URect2D, layer: u32) -> SpriteSlice<'texture, Key, T> {
		SpriteSlice { key, texture: self, rect, layer }
	}
}

impl<T: TextureAttribute> Into<TextureDataBytes> for TextureData<T> {
	fn into(self) -> TextureDataBytes {
		self.into_bytes()
	}
}

pub fn pack_sprite_atlas_array<'image, Key: std::hash::Hash + PartialEq + Eq, T: TextureAttribute>(
	mut sprites: Vec<SpriteSlice<'image, Key, T>>,
	max_size: UVec3
) -> Option<(TextureData<T>, FxHashMap<Key, (usize, Rect2D)>)>  {
	
	sprites.sort_by(|a,b| (a.rect.area()).cmp(&(b.rect.area())));
	let pixels_sum = sprites.iter().map(|sprite| sprite.rect.area()).sum::<u32>();
	let max_x = sprites.iter().max_by(|a,b| (a.rect.size().x).cmp(&b.rect.size().x)).unwrap().rect.size().x;
	let max_y = sprites.iter().max_by(|a,b| (a.rect.size().y).cmp(&b.rect.size().y)).unwrap().rect.size().y;


	let mut spaces = (0..max_size.z).rev().map(|z| (z, URect2D::sized(UVec2::new(max_size.x, max_size.y)))).collect::<Vec<_>>();
	let placements = sprites.iter().map(|sprite| {
		for i in (0..spaces.len()).rev() {
			let diff = spaces[i].1.size().as_ivec2() - sprite.rect.size().as_ivec2();
			if diff.x >= 0 && diff.y >= 0 {
				let (z, space) = spaces.swap_remove(i);
				if diff.x > 0 && diff.y > 0 {
					if diff.x > diff.y {
						spaces.push((z, URect2D::new(UVec2::new(space.end.x - diff.x as u32, space.start.y), space.end)));
						spaces.push((z, URect2D::new(
							UVec2::new(space.start.x, space.end.y - diff.y as u32),
							UVec2::new(space.end.x - diff.x as u32, space.end.y),
						)));
					} else {
						spaces.push((z, URect2D::new(UVec2::new(space.start.x, space.end.y - diff.y as u32), space.end)));
						spaces.push((z, URect2D::new(
							UVec2::new(space.end.x - diff.x as u32, space.start.y),
							UVec2::new(space.end.x, space.end.y - diff.y as u32),
						)));
					}
				} else if diff.x > 0 {
					spaces.push((z, URect2D::new(UVec2::new(space.end.x - diff.x as u32, space.start.y), space.end)));
				} else if diff.y > 0 {
					spaces.push((z, URect2D::new(UVec2::new(space.start.x, space.end.y - diff.y as u32), space.end)));
				}
				return (z, space.start);
			}
		}
		(0, UVec2::ZERO)
	}).collect::<Vec<_>>();

	

	let mut atlas = TextureData::new_empty(max_size);
	let atlas_2d_size = Vec2::new(max_size.x as f32, max_size.y as f32);
	let mut map = FxHashMap::default();
	for (sprite, (z, start)) in sprites.into_iter().zip(placements) {
		atlas.blit_from(
			sprite.texture,
			UVec3::new(sprite.rect.size().x, sprite.rect.size().y, 1),
			UVec3::new(sprite.rect.start.x, sprite.rect.start.y, sprite.layer),
			UVec3::new(start.x, start.y, z)
		);
		map.insert(sprite.key, (z as usize, Rect2D::with_extent(start.as_vec2() / atlas_2d_size, sprite.rect.size().as_vec2() / atlas_2d_size)));
	}

	Some((atlas, map))
}

pub fn pack_sprite_atlas<'image, Key: std::hash::Hash + PartialEq + Eq + std::fmt::Debug, T: TextureAttribute>(sprites: Vec<SpriteSlice<'image, Key, T>>, max_size: UVec2) -> Option<(TextureData<T>, FxHashMap<Key, Rect2D>)> {
	pack_sprite_atlas_array(sprites, UVec3::new(max_size.x, max_size.y, 1)).map(|(atlas, map)| {
		(atlas, map.into_iter().map(|(k, (_, v))| (k,v)).collect())
	})
}