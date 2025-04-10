//! Framework for creating games in rust.
//! 
//! This library is currently heavily work-in-progress!

pub mod gfx;
pub mod input;
pub mod math;
pub mod console;

pub use isopod_derive::*;
pub use rustc_hash;

mod util;

use std::{cell::Cell, collections::VecDeque};


pub trait App {
	/// Called every time the screen is rendered and provides access to the [EngineCtx].
	fn update(&mut self, c: &EngineCtx);
}

pub struct EngineCtx {
	pub gfx: gfx::GfxCtx,
	pub input: input::InputCtx,
	pub console: console::Console,
	/// Time (in seconds) that has elapsed since [update](App::update) was last called.
	pub dt: f64,
	/// Estimation of the current number of frames rendered per second.
	pub fps: f64,
	dt_buffer: VecDeque<f64>,
	dt_buffer_sum: f64,
	last_fps_update: usize,
	should_quit: Cell<bool>,
}

impl EngineCtx {
	fn new() -> Self {
		let gfx = gfx::GfxCtx::new();
		Self {
			input: input::InputCtx::new(),
			console: console::Console::new(&gfx),
			gfx,
			dt: 0.,
			fps: 1.,
			dt_buffer: VecDeque::new(),
			dt_buffer_sum: 0.,
			last_fps_update: 0,
			should_quit: Cell::new(false),
		}
	}

	/// Quits the application at the end of the current call to [update](App::update).
	pub fn quit(&self) {
		self.should_quit.set(true);
	}
}

/// Starts the engine with a given function that returns an [App].
pub fn run<F: (FnOnce(&EngineCtx) -> T) + 'static, T: App + 'static>(load_fn: F) {

	let sdl = sdl2::init().unwrap();
	let sdl_video = sdl.video().unwrap();
	let mut event_pump = sdl.event_pump().unwrap();

	let mut gfx_sys = gfx::GfxSys::new(&sdl_video);
	let mut ctx = EngineCtx::new();

	// initial load
	gfx_sys.start_update(&mut ctx.gfx, false);
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
		gfx_sys.start_update(&mut ctx.gfx, true);
		game.update(&ctx);
		ctx.console.update(&ctx.gfx, &ctx.input);
		gfx_sys.render(&mut ctx.gfx);


		// timing
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