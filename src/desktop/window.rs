use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::io::{Window, WindowSignal};

use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window as WinitWindow, WindowBuilder};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

pub type Keys = Rc<RefCell<HashSet<KeyCode>>>;

pub struct DesktopWindow<'a> {
    event_loop: EventLoop<()>,
    input: WinitInputHelper,
    window: Arc<WinitWindow>,
    pixels: Pixels<'a>,
    pub keys: Keys,
}

fn draw(frame: &mut [u8], fb: &[u32; 160 * 144]) {
    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        pixel.copy_from_slice(&((fb[i] << 8) | 0xff).to_be_bytes())
    }
}

impl<'a> DesktopWindow<'a> {
    pub fn new(title: impl Into<String>) -> Result<Self, Error> {
        let event_loop = EventLoop::new().unwrap();
        let input = WinitInputHelper::new();
        let window = Arc::new({
            let size = LogicalSize::new((WIDTH * 4) as f64, (HEIGHT * 4) as f64);
            WindowBuilder::new()
                .with_title(title.into())
                .with_inner_size(size)
                .with_min_inner_size(size)
                .build(&event_loop)
                .unwrap()
        });

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window.clone());
            Pixels::new(WIDTH, HEIGHT, surface_texture)?
        };

        Ok(Self {
            event_loop,
            input,
            window,
            pixels,
            keys: Rc::new(HashSet::new().into()),
        })
    }
}

impl<'a> Window for DesktopWindow<'a> {
    fn update(&mut self, fb: &[u32; 160 * 144]) -> Option<WindowSignal> {
        let mut res = None;
        let mut keys = (*self.keys).borrow_mut();
        self.event_loop
            .pump_events(Some(Duration::ZERO), |event, elwt| {
                if let Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } = event
                {
                    draw(self.pixels.frame_mut(), fb);
                    if let Err(err) = self.pixels.render() {
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
                    if let PhysicalKey::Code(keycode) = keyboard_event.physical_key {
                        if keyboard_event.state.is_pressed() {
                            keys.insert(keycode);
                        } else {
                            keys.remove(&keycode);
                        }
                    }
                }

                if self.input.update(&event) {
                    if self.input.close_requested() {
                        elwt.exit();
                        res = Some(WindowSignal::Exit);
                        return;
                    }

                    if let Some(size) = self.input.window_resized() {
                        if let Err(err) = self.pixels.resize_surface(size.width, size.height) {
                            eprintln!("Error during resize: {}", err);
                            return;
                        }
                    }

                    self.window.request_redraw();
                }
            });

        res
    }
}
