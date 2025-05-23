pub mod audio;
pub mod consts;
pub mod desktop;
pub mod display;
pub mod interrupts_timers;
pub mod io;
pub mod mmio;
pub mod opcodes;
pub mod state;

use crate::desktop::input::{Gamepad, GamepadRecorder, GamepadReplay, Keyboard};
use crate::desktop::load_save::FSLoadSave;
use crate::io::Input;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The gameboy rom file
    rom: String,

    /// Setting uses more battery and set the CPU to 100% but could sometimes solve inconsistent timing.
    #[arg(long)]
    loop_lock_timing: bool,

    #[arg(long)]
    fifo_input: Option<String>,

    #[arg(long)]
    fifo_output: Option<String>,

    #[arg(long)]
    record_input: Option<String>,

    #[arg(long)]
    replay_input: Option<String>,

    #[arg(long)]
    state_file: Option<String>,

    #[arg(short, long, default_value_t = false)]
    load_state: bool,

    #[arg(short, long, default_value_t = false)]
    keyboard: bool,

    #[arg(short, long, default_value_t = 1.0)]
    speed: f32,

    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

fn main() {
    let cli = Cli::parse();

    println!("Starting {:?}...", &cli.rom);

    let serial = desktop::serial::UnconnectedSerial {};
    let window = desktop::window::DesktopWindow::new().unwrap();

    let mut gamepad: Box<dyn Input> = if let Some(record_file) = cli.replay_input {
        Box::new(GamepadReplay::new(record_file))
    } else if cli.keyboard {
        Box::new(Keyboard::new(window.keys.clone()))
    } else {
        Box::new(Gamepad::new())
    };

    if let Some(record_file) = cli.record_input {
        gamepad = Box::new(GamepadRecorder::new(gamepad, record_file));
    };

    let mut fs_load_save = FSLoadSave::new(&cli.rom, format!("{}.sav", &cli.rom));
    if let Some(state_file) = &cli.state_file {
        fs_load_save = fs_load_save.state_file(state_file);
    }

    let mut gameboy = io::Gameboy::<_, _, _, desktop::audio::RodioAudio, _>::new(
        gamepad,
        window,
        serial,
        fs_load_save,
        cli.speed as f64,
    );

    if cli.debug {
        gameboy.debug();
    }

    if cli.load_state {
        gameboy.load_state().unwrap();
    }

    gameboy.start();
}
