
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Scancode;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
	A,B,C,D,E,F,G,H,I,J,K,L,M,N,O,P,Q,R,S,T,U,V,W,X,Y,Z,
	F1,F2,F3,F4,F5,F6,F7,F8,F9,F10,F11,F12,
	BACKSPACE,TAB,RETURN,ESCAPE,SPACE,
	UP,LEFT,RIGHT,DOWN,INSERT,HOME,PAGEUP,PAGEDOWN,
}

macro_rules! one_to_one_mapping {
	( $name:ident : $t:ty : $( $k:ident ),* $(,)*) => {
		fn $name(v: $t) -> Option<Key> {
			match v {
				$(
					<$t>:: $k => Some(Key:: $k),
				)*
				_ => None,
			}
		}
	};
}

one_to_one_mapping!( keycode_one_to_one_mapping : Keycode :
	A,B,C,D,E,F,G,H,I,J,K,L,M,N,O,P,Q,R,S,T,U,V,W,X,Y,Z,
	F1,F2,F3,F4,F5,F6,F7,F8,F9,F10,F11,F12,
	BACKSPACE,TAB,RETURN,ESCAPE,SPACE,
	UP,LEFT,RIGHT,DOWN,INSERT,HOME,PAGEUP,PAGEDOWN,
);

one_to_one_mapping!( scancode_one_to_one_mapping : Scancode :
	A,B,C,D,E,F,G,H,I,J,K,L,M,N,O,P,Q,R,S,T,U,V,W,X,Y,Z,
	F1,F2,F3,F4,F5,F6,F7,F8,F9,F10,F11,F12,
);

impl Key {
	pub(crate) fn from_keycode(key: Keycode) -> Option<Key> {
		match key {
			key => keycode_one_to_one_mapping(key),
		}
	}

	pub(crate) fn from_scancode(key: Scancode) -> Option<Key> {
		match key {
			Scancode::Backspace => Some(Key::BACKSPACE),
			Scancode::Tab => Some(Key::TAB),
			Scancode::Return => Some(Key::RETURN),
			Scancode::Escape => Some(Key::ESCAPE),
			Scancode::Space => Some(Key::SPACE),
			Scancode::Up => Some(Key::UP),
			Scancode::Left => Some(Key::LEFT),
			Scancode::Right => Some(Key::RIGHT),
			Scancode::Down => Some(Key::DOWN),
			Scancode::Insert => Some(Key::INSERT),
			Scancode::Home => Some(Key::HOME),
			Scancode::PageUp => Some(Key::PAGEUP),
			Scancode::PageDown => Some(Key::PAGEDOWN),
			key => scancode_one_to_one_mapping(key),
		}
	}
}