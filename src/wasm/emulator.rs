use pixels::wgpu::{Backends};
use pixels::{PixelsBuilder, SurfaceTexture};
use web_sys::HtmlCanvasElement;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::web::WindowBuilderExtWebSys;
use winit::window::WindowBuilder;

use console_error_panic_hook;
use core::time::Duration;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::desktop;
use crate::desktop::audio::CpalAudio;
use crate::desktop::input::{Gamepad, InputCombiner, Keyboard};
use crate::wasm::load_save::StaticRom;
use crate::wasm::input::{WebButtonsInput, WebButtonsInputConfig};

use crate::io::{Audio, Gameboy, Input, Serial};
use crate::logs;
use crate::logs::{elog, log, LogLevel};
use crate::wasm::utils::SystemTime;
use wasm_bindgen::prelude::*;

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

#[wasm_bindgen]
pub struct Emulator {
    gameboy: Gameboy<Box<dyn Input>, Box<dyn Serial>, Box<dyn Audio>, StaticRom>,
    event_loop: EventLoop<()>,
    title: &'static str,
    keys: Arc<Mutex<HashSet<KeyCode>>>,
}

#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        set_panic_hook();
        logs::set_log_level("infos,errors,debug");

        let title = env!("GAME_TITLE");

        log(LogLevel::Infos, format!("Starting ..."));

        let keys: Arc<Mutex<HashSet<KeyCode>>> = Arc::new(Mutex::new(HashSet::new()));

        let audio: Box<dyn Audio> = Box::new(CpalAudio::new());

        let serial: Box<dyn Serial> = Box::new(desktop::serial::UnconnectedSerial {});

        let gamepad: Box<dyn Input> = Box::new(InputCombiner::new(vec![
            Box::new(Gamepad::new()),
            Box::new(Keyboard::new(keys.clone())),
            Box::new(WebButtonsInput::new(WebButtonsInputConfig {
                button_a_id: "gb-button-a",
                button_b_id: "gb-button-b",
                button_start_id: "gb-button-start",
                button_select_id: "gb-button-select",
                button_up_id: "gb-button-up",
                button_down_id: "gb-button-down",
                button_left_id: "gb-button-left",
                button_right_id: "gb-button-right",
            })),
        ]));

        let fs_load_save = StaticRom::new();

        let gameboy = Gameboy::<_, _, _, _>::new(gamepad, serial, audio, fs_load_save, 1.);

        Self {
            gameboy,
            event_loop: EventLoopBuilder::new().build().unwrap(),
            title,
            keys,
        }
    }

    pub fn load_state(&mut self) {
        self.gameboy.load_state().unwrap();
    }

    pub async fn run(self, canvas: HtmlCanvasElement) {
        let Self {
            event_loop,
            mut gameboy,
            title,
            keys,
        } = self;

        let mut frames = 0;
        let mut start_time = SystemTime::now();
        let size = LogicalSize::new((WIDTH * 4) as f64, (HEIGHT * 4) as f64);
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_canvas(Some(canvas))
            .build(&event_loop)
            .unwrap();
        let mut pixels = {
            let surface_texture = SurfaceTexture::new(WIDTH * 4, HEIGHT * 4, &window);
            PixelsBuilder::new(WIDTH, HEIGHT, surface_texture)
                .wgpu_backend(Backends::GL)
                .build_async()
                .await
                .unwrap()
        };
        let _ = event_loop.run(move |event, elwt| {
            if let Some(fb) = gameboy.sleep_and_draw() {
                desktop::window::draw(pixels.frame_mut(), &fb);
                frames += 1;
                if frames == 60 {
                    log(
                        LogLevel::Infos,
                        format!(
                            "FPS: {}",
                            frames as f32
                                / SystemTime::now()
                                    .duration_since(start_time)
                                    .unwrap()
                                    .as_secs_f32()
                        ),
                    );
                    frames = 0;
                    start_time = SystemTime::now();
                }
            }
            if let Err(err) = pixels.render() {
                elog(LogLevel::Error, format!("Error during render: {}", err));
                return;
            }
            if let Event::WindowEvent {
                window_id: _,
                event:
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        event: ref keyboard_event,
                        is_synthetic: _,
                    },
            } = event
            {
                if let PhysicalKey::Code(keycode) = keyboard_event.physical_key {
                    if keyboard_event.state.is_pressed() {
                        log(LogLevel::Debug, format!("KEY {:?} pressed", keycode));
                    } else {
                        log(LogLevel::Debug, format!("KEY {:?} unpressed", keycode));
                    }
                }

                if let Ok(mut keys) = keys.lock() {
                    if let PhysicalKey::Code(keycode) = keyboard_event.physical_key {
                        if keyboard_event.state.is_pressed() {
                            (*keys).insert(keycode);
                        } else {
                            (*keys).remove(&keycode);
                        }
                    }
                }
            }

            gameboy.run_until_next_sleep();
            elwt.set_control_flow(winit::event_loop::ControlFlow::wait_duration(
                Duration::from_micros(1000000 / 60),
            ));
        });
    }
}
