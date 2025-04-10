pub mod gfx;
pub mod input;
pub mod math;
pub mod console;

pub use bytemuck;

pub use isopod_derive::*;
pub use rustc_hash;

mod util;

use std::{cell::Cell, collections::VecDeque};

pub trait Game {
	fn update(&mut self, c: &EngineCtx);
}

pub struct EngineCtx {
	pub gfx: gfx::GfxCtx,
	pub input: input::InputCtx,
	pub dt: f64,
	pub fps: f64,
	dt_buffer: VecDeque<f64>,
	dt_buffer_sum: f64,
	last_fps_update: usize,
	should_quit: Cell<bool>,
}

impl EngineCtx {
	fn new() -> Self {
		Self {
			gfx: gfx::GfxCtx::new(),
			input: input::InputCtx::new(),
			dt: 0.,
			fps: 1.,
			dt_buffer: VecDeque::new(),
			dt_buffer_sum: 0.,
			last_fps_update: 0,
			should_quit: Cell::new(false),
		}
	}

	pub fn quit(&self) {
		self.should_quit.set(true);
	}
}

pub fn run_game<F: (FnOnce(&EngineCtx) -> T) + 'static, T: Game + 'static>(load_fn: F) {

	let sdl = sdl2::init().unwrap();
	let sdl_video = sdl.video().unwrap();
	let mut event_pump = sdl.event_pump().unwrap();

	let mut ctx = EngineCtx::new();
	let mut gfx_sys = gfx::GfxSys::new(&sdl_video);

	// initial load
	gfx_sys.start_update(&mut ctx.gfx);
	let mut game = load_fn(&ctx);
	gfx_sys.render(&mut ctx.gfx);

	let mut frame_time = std::time::Instant::now();
	while !ctx.should_quit.get() {

		// handle events
		ctx.input.start_update();
		for event in event_pump.poll_iter() {
			use sdl2::event::Event;
			match event {
				Event::Quit { .. } => {
					ctx.should_quit.set(true);
				},
				event => {
					ctx.input.process_event(event);
				},
			}
		}
		gfx_sys.start_update(&mut ctx.gfx);
		game.update(&ctx);
		gfx_sys.render(&mut ctx.gfx);

		let new_time = std::time::Instant::now();
		ctx.dt = new_time.duration_since(frame_time).as_secs_f64();
		frame_time = new_time;

		ctx.dt_buffer_sum += ctx.dt;
		ctx.dt_buffer.push_back(ctx.dt);
		if ctx.dt_buffer.len() >= 60 {
			ctx.dt_buffer_sum -= ctx.dt_buffer.pop_front().unwrap();
		}
		ctx.last_fps_update += 1;
		if ctx.last_fps_update >= 15 {
			ctx.fps = 1.0 / (ctx.dt_buffer_sum / ctx.dt_buffer.len() as f64);
			ctx.last_fps_update = 0;
		}
	}
}