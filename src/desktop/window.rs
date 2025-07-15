use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::io::{Window, WindowSignal};

use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoopBuilder;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::window::{WindowBuilder};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

pub type Keys = Arc<Mutex<HashSet<KeyCode>>>;

pub struct DesktopWindow {
    fb_send: Sender<Box<[u32; 160 * 144]>>,
    signal_recv: Receiver<WindowSignal>,
    pub keys: Keys,
}

impl DesktopWindow {
    pub fn new(title: impl Into<String>) -> Result<Self, Error> {
        let title: String = title.into();
        let (fb_send, fb_recv) = channel();
        let (signal_send, signal_recv) = channel();

        let keys = Arc::new(Mutex::new(HashSet::new()));

        let key_eventloop = keys.clone();
        thread::spawn(move || {
            let keys = key_eventloop;
            let event_loop = EventLoopBuilder::new().with_any_thread(true).build().unwrap();
            let mut input = WinitInputHelper::new();
            let window = Arc::new({
                let size = LogicalSize::new((WIDTH * 4) as f64, (HEIGHT * 4) as f64);
                WindowBuilder::new()
                    .with_title(title)
                    .with_inner_size(size)
                    .with_min_inner_size(size)
                    .build(&event_loop)
                    .unwrap()
            });

            let mut pixels = {
                let window_size = window.inner_size();
                let surface_texture =
                    SurfaceTexture::new(window_size.width, window_size.height, window.clone());
                Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap()
            };
            let mut fb = Box::new([0; 160 * 144]);
            event_loop
            .run(|event, elwt| {
                if let Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } = event
                {
                    loop {
                        if let Ok(new_fb) = fb_recv.try_recv() {
                            fb = new_fb;
                        } else {
                            break;
                        }
                    }
                    draw(pixels.frame_mut(), &fb);
                    if let Err(err) = pixels.render() {
                        eprintln!("Error during render: {}", err);
                        return;
                    }
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

                if input.update(&event) {
                    if input.close_requested() {
                        elwt.exit();
                        if let Err(err) = signal_send.send(WindowSignal::Exit) {
                            eprintln!("window signal send failed with error {}", err);
                        }
                        return;
                    }

                    if let Some(size) = input.window_resized() {
                        if let Err(err) = pixels.resize_surface(size.width, size.height) {
                            eprintln!("Error during resize: {}", err);
                            return;
                        }
                    }

                    window.request_redraw();
                }
            }).unwrap();
        });


        Ok(Self {
            fb_send,
            signal_recv,
            keys,
        })
    }
}

impl Window for DesktopWindow {
    fn update(&mut self, fb: Box<[u32; 160 * 144]>) -> Option<WindowSignal> {
        if let Err(err) = self.fb_send.send(fb) {
            eprintln!("Framebuffer channel send failed with error: {}", err);
        }

        if let Ok(signal) = self.signal_recv.try_recv() {
            Some(signal)
        } else {
            None
        }
    }
}

fn draw(frame: &mut [u8], fb: &[u32; 160 * 144]) {
    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        pixel.copy_from_slice(&((fb[i] << 8) | 0xff).to_be_bytes())
    }
}
