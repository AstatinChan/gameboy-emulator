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
use crate::io::{Serial, Input};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The gameboy rom file
    rom: String,

    /// Serial communication input from a FIFO file
    #[arg(long)]
    fifo_input: Option<String>,

    /// Serial communication output from a FIFO file
    #[arg(long)]
    fifo_output: Option<String>,

    /// Record the inputs into a file when they happen so it can be replayed with --replay-input
    #[arg(long)]
    record_input: Option<String>,

    /// Replay the inputs from the file recorded by record_input
    #[arg(long)]
    replay_input: Option<String>,

    /// The file to which the state will be save when using the X (North) button of the controller
    /// and loaded from when using the --load-state parameter
    #[arg(long)]
    state_file: Option<String>,

    /// Start at the state defined by --state-file instead of the start of bootrom with empty mem
    #[arg(short, long, default_value_t = false)]
    load_state: bool,

    /// Gets inputs from keyboard instead of gamepad
    #[arg(short, long, default_value_t = false)]
    keyboard: bool,

    #[arg(short, long, default_value_t = 1.0)]
    speed: f32,

    /// Will print all of the opcodes executed (WARNING: THERE ARE MANY)
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    /// Skip bootrom (will start the execution at 0x100 with all registers empty
    #[arg(long, default_value_t = false)]
    skip_bootrom: bool,

    /// Dump state to files on stop
    #[arg(long, default_value_t = false)]
    stop_dump_state: bool,

    /// Window title
    #[arg(long, default_value = "Gameboy Emulator")]
    title: String
}

fn main() {
    let cli = Cli::parse();

    println!("Starting {:?}...", &cli.rom);

    let window = desktop::window::DesktopWindow::new(cli.title).unwrap();

    let serial: Box<dyn Serial> = match (cli.fifo_input, cli.fifo_output) {
        (Some(fifo_input), Some(fifo_output)) => Box::new(desktop::serial::FIFOSerial::new(fifo_input, fifo_output)),
        _ => Box::new(desktop::serial::UnconnectedSerial {})
    };

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

    if cli.skip_bootrom {
        gameboy.skip_bootrom();
    }

    gameboy.start();

    if cli.stop_dump_state {
        gameboy.dump_state().unwrap();
    }
}
