pub mod audio;
pub mod consts;

pub mod display;
pub mod interrupts_timers;
pub mod io;
pub mod logs;
pub mod mmio;
pub mod opcodes;
pub mod state;
#[cfg(not(feature = "dynamic_rom"))]
use cpal::traits::StreamTrait;
use wasm_bindgen::prelude::*;

mod wasm_main {
    use crate::logs::{log, LogLevel};
    
    pub fn main() {
        log(LogLevel::Infos, "This was a triumph");
    }
}

#[wasm_bindgen]
pub fn main() {
    #[cfg(not(target_family = "wasm"))]
    desktop_main::main();

    #[cfg(target_family = "wasm")]
    wasm_main::main();
}
