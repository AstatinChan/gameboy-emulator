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
pub mod utils_wasm;
#[cfg(not(feature = "dynamic_rom"))]
use cpal::traits::StreamTrait;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use console_error_panic_hook;

use crate::desktop::input::{Gamepad, GamepadRecorder, GamepadReplay, Keyboard, InputCombiner};
use crate::desktop::load_save::StaticRom;
use crate::desktop::audio::{RodioAudio, HeadlessAudio};

use crate::io::{Input, Serial, Window, Audio, Gameboy};
use crate::logs::{log, LogLevel};
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

#[wasm_bindgen]
pub fn main() {
    utils_wasm::test();
    set_panic_hook();
    logs::set_log_level("infos,errors,debug,halt_cycles");


    let rom = env!("GAME_ROM_ASSET");

    let title = env!("GAME_TITLE");

    #[cfg(feature = "dynamic_rom")]
    log(LogLevel::Infos, format!("Starting ..."));
    
    #[cfg(not(feature = "dynamic_rom"))]
    log(LogLevel::Infos, format!("Starting {:?}...", title));
    
    let (window, keys): (Box<dyn Window>, desktop::window::Keys) = (
        Box::new(desktop::window::Headless),
        Arc::new(Mutex::new(HashSet::new())),
    );
    
    let audio: Box<dyn Audio> = Box::new(HeadlessAudio{});
    
    let serial: Box<dyn Serial> = Box::new(desktop::serial::UnconnectedSerial {});
    
    let mut gamepad: Box<dyn Input> = Box::new(Gamepad::new());
    
    let mut fs_load_save = StaticRom::new("ABC");
    
    let mut gameboy = Gameboy::<_, _, _, _, _>::new(
        gamepad,
        window,
        serial,
        audio,
        fs_load_save,
        1.,
    );
    
    gameboy.skip_bootrom();

    gameboy.start();
}
