pub mod audio;
pub mod consts;

pub mod desktop;

pub mod display;
pub mod interrupts_timers;
pub mod io;
pub mod logs;
pub mod mmio;
pub mod opcodes;
pub mod state;
#[cfg(target_family = "wasm")]
pub mod utils_wasm;
#[cfg(target_family = "wasm")]
use winit::platform::web;
#[cfg(target_family = "wasm")]
use winit::platform::web::WindowBuilderExtWebSys;
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use winit::dpi::LogicalSize;
#[cfg(target_family = "wasm")]
use web_sys::HtmlCanvasElement;
use pixels::{Error, Pixels, SurfaceTexture, PixelsBuilder};
#[cfg(not(feature = "dynamic_rom"))]
use cpal::traits::StreamTrait;

use std::collections::HashSet;
use std::io::Empty;
use core::time::Duration;
use std::sync::{Arc, Mutex};
#[cfg(target_family = "wasm")]
use console_error_panic_hook;

use crate::desktop::input::{Gamepad, GamepadRecorder, GamepadReplay, Keyboard, InputCombiner};
use crate::desktop::load_save::StaticRom;
use crate::desktop::audio::{RodioAudio, HeadlessAudio};

use crate::io::{Input, Serial, Window, Audio, Gameboy};
use crate::logs::{log, elog, LogLevel};
#[cfg(target_family = "wasm")]
use crate::utils_wasm::SystemTime;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    console_error_panic_hook::set_once();
}

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub struct Emulator {
    gameboy: Gameboy::<Box<dyn Input>, Box<dyn Serial>, Box<dyn Audio>, StaticRom>,
    event_loop: EventLoop<()>,
    title: &'static str,
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        set_panic_hook();
        logs::set_log_level("infos,errors");

        let rom = env!("GAME_ROM_ASSET");

        let title = env!("GAME_TITLE");

        log(LogLevel::Infos, format!("Starting ..."));
        
        let (window, keys): (Box<dyn Window>, desktop::window::Keys) = (
            Box::new(desktop::window::Headless),
            Arc::new(Mutex::new(HashSet::new())),
        );
        
        let audio: Box<dyn Audio> = Box::new(HeadlessAudio{});
        
        let serial: Box<dyn Serial> = Box::new(desktop::serial::UnconnectedSerial {});
        
        let mut gamepad: Box<dyn Input> = Box::new(Gamepad::new());
        
        let mut fs_load_save = StaticRom::new("ABC");

        
        let mut gameboy = Gameboy::<_, _, _, _>::new(
            gamepad,
            serial,
            audio,
            fs_load_save,
            1.,
        );
        
        Self {
            gameboy,
            event_loop: EventLoopBuilder::new()
                .build()
                .unwrap(),
            title
        }
    }

    pub async fn run(self, canvas: HtmlCanvasElement) {
        let Self {
            event_loop,
            mut gameboy,
            title,
        } = self;

        let mut frames = 0;
        let mut start_time = SystemTime::now();
        let size = LogicalSize::new((WIDTH * 4) as f64, (HEIGHT * 4) as f64);
        let window = WindowBuilder::new()
                    .with_title(title)
                    .with_inner_size(size)
                    .with_min_inner_size(size)
                    .with_append(true)
                    .build(&event_loop)
                    .unwrap();
        let mut pixels = {
            let window_size = window.inner_size();

            let surface_texture =
                SurfaceTexture::new(WIDTH*4, HEIGHT*4, &window);
            Pixels::new_async(WIDTH, HEIGHT, surface_texture).await.unwrap()
        };
        event_loop.run(move |event, elwt| {
            if let Some(fb) = gameboy.sleep_and_draw() {
                use crate::logs::console_log;

                desktop::window::draw(pixels.frame_mut(), &fb);
                frames += 1;
                if frames == 60 {
                    log(LogLevel::Infos, format!("FPS: {}", frames as f32 / SystemTime::now().duration_since(start_time).unwrap().as_secs_f32()));
                    frames = 0;
                    start_time = SystemTime::now();
                }

            }
            if let Err(err) = pixels.render() {
                elog(LogLevel::Error, format!("Error during render: {}", err));
                return;
            }

            gameboy.run_until_next_sleep();
            elwt.set_control_flow(winit::event_loop::ControlFlow::wait_duration(Duration::from_micros(1000000/60)));
        });
    }
}
