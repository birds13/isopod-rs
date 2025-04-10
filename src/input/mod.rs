
use std::{any::TypeId, collections::HashMap};

use glam::*;

mod key;
pub use key::Key;

use crate::util::BufferCell;

#[derive(Default)]
struct ButtonState {
	pressed: bool,
	pressed_this_frame: bool,
	released_this_frame: bool,
}

#[derive(Default)]
pub struct TextInput {
	pub n_backspaces: usize,
	pub text: String,
	pub enter: bool,
}

pub struct InputCtx {
	pub text_input: TextInput,
	new_mappings: BufferCell<(ButtonMapping, TypeId)>,
	mappings: HashMap<ButtonMapping, TypeId>,
	button_states: HashMap<TypeId, ButtonState>,
}

#[derive(PartialEq, Eq, Hash)]
pub enum ButtonMapping {
	LogicalKey(Key),
	PhysicalKey(Key),
}

impl InputCtx {
	pub(crate) fn new() -> Self {
		Self {
			new_mappings: BufferCell::new(),
			mappings: HashMap::new(),
			button_states: HashMap::new(),
			text_input: TextInput::default(),
		}
	}

	pub(crate) fn start_update(&mut self) {
		for (mapping, id) in self.new_mappings.get_mut().drain(..) {
			self.mappings.insert(mapping, id);
			self.button_states.insert(id, ButtonState::default());
		}
		for state in self.button_states.values_mut() {
			state.pressed_this_frame = false;
			state.released_this_frame = false;
		}
		self.text_input.n_backspaces = 0;
		self.text_input.enter = false;
		self.text_input.text.clear();
	}

	pub(crate) fn process_event(&mut self, event: sdl2::event::Event) {
		use sdl2::event::Event;
		match event {
			Event::TextInput { text, .. } => {
				self.text_input.text.push_str(&text);
			},
			Event::KeyDown { keycode, scancode, repeat, .. } => {
				if scancode == Some(sdl2::keyboard::Scancode::Backspace) {
					if self.text_input.text.pop().is_none() {
						self.text_input.n_backspaces += 1;
					}
				} else if scancode == Some(sdl2::keyboard::Scancode::Return) {
					self.text_input.enter = true;
				}
				if !repeat {
					if let Some(state) = keycode.map(|k| Key::from_keycode(k)).flatten()
						.map(|k| self.mappings.get(&ButtonMapping::PhysicalKey(k))).flatten()
						.map(|id| self.button_states.get_mut(id)).flatten()
					{
						state.pressed = true;
						state.pressed_this_frame = true;
					}
					if let Some(state) = scancode.map(|k| Key::from_scancode(k)).flatten()
						.map(|k| self.mappings.get(&ButtonMapping::LogicalKey(k))).flatten()
						.map(|id| self.button_states.get_mut(id)).flatten()
					{
						state.pressed = true;
						state.pressed_this_frame = true;
					}
				}
			},
			Event::KeyUp { keycode, scancode, repeat, .. } => {
				if !repeat {
					if let Some(state) = keycode.map(|k| Key::from_keycode(k)).flatten()
						.map(|k| self.mappings.get(&ButtonMapping::PhysicalKey(k))).flatten()
						.map(|id| self.button_states.get_mut(id)).flatten()
					{
						state.pressed = false;
						state.released_this_frame = true;
					}
					if let Some(state) = scancode.map(|k| Key::from_scancode(k)).flatten()
						.map(|k| self.mappings.get(&ButtonMapping::LogicalKey(k))).flatten()
						.map(|id| self.button_states.get_mut(id)).flatten()
					{
						state.pressed = false;
						state.released_this_frame = true;
					}
				}
			},
			_ => {},
		}
	}

	pub fn is_pressed<Action: 'static>(&self) -> bool {
		self.button_states.get(&TypeId::of::<Action>()).map(|state| state.pressed).unwrap_or(false)
	}

	pub fn is_just_pressed<Action: 'static>(&self) -> bool {
		self.button_states.get(&TypeId::of::<Action>()).map(|state| state.pressed_this_frame).unwrap_or(false)
	}

	pub fn is_just_released<Action: 'static>(&self) -> bool {
		self.button_states.get(&TypeId::of::<Action>()).map(|state| state.released_this_frame).unwrap_or(false)
	}

	pub fn map_button<Action: 'static>(&self, mapping: ButtonMapping) {
		self.new_mappings.push((mapping, TypeId::of::<Action>()));
	}
}