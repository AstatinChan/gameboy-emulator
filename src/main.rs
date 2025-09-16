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

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener};

use crate::desktop::input::{Gamepad, GamepadRecorder, GamepadReplay, Keyboard};
use crate::desktop::load_save::FSLoadSave;
use crate::desktop::audio::{RodioAudio, HeadlessAudio};
use crate::io::{Input, Serial, Window, Audio};
use crate::logs::{log, LogLevel};
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

    /// Skip bootrom (will start the execution at 0x100 with all registers empty
    #[arg(long, default_value_t = false)]
    skip_bootrom: bool,

    /// Dump state to files on stop
    #[arg(long, default_value_t = false)]
    stop_dump_state: bool,

    /// Do not create a window
    #[arg(long, default_value_t = false)]
    headless: bool,

    /// Window title
    #[arg(long, default_value = "Gameboy Emulator")]
    title: String,

    /// Serial tcp listen port
    #[arg(short = 'L', long)]
    listen: Option<u16>,

    /// Serial tcp connect address <address:port>
    #[arg(short, long)]
    connect: Option<String>,

    /// Don't send (or expect) a byte as a response to a serial transfer
    #[arg(long, default_value_t = false)]
    no_response: bool,

    /// Auto restart on stop or crash
    #[arg(long, default_value_t = false)]
    restart_on_stop: bool,

    /// Verbosity. Coma separated values (possible values: infos,debug,opcode_dump,halt_cycles,audio_latency,errors,none)
    #[arg(short, long, default_value = "infos,errors")]
    verbosity: String,
}

fn main() {
    let cli = Cli::parse();

    logs::set_log_level(cli.verbosity);

    let listener = if let Some(port) = cli.listen {
        Some(TcpListener::bind(("0.0.0.0", port)).unwrap())
    } else {
        None
    };

    loop {
        log(LogLevel::Infos, format!("Starting {:?}...", &cli.rom));

        let (window, keys): (Box<dyn Window>, desktop::window::Keys) = if cli.headless {
            (
                Box::new(desktop::window::Headless),
                Arc::new(Mutex::new(HashSet::new())),
            )
        } else {
            let window = desktop::window::DesktopWindow::new(cli.title.clone()).unwrap();
            let keys = window.keys.clone();
            (Box::new(window), keys)
        };

        let audio: Box<dyn Audio> = if cli.headless {
            Box::new(HeadlessAudio{})
        } else {
            Box::new(RodioAudio::new())
        };

        let serial: Box<dyn Serial> =
            match (cli.fifo_input.clone(), cli.fifo_output.clone(), &listener, cli.connect.clone()) {
                (_, _, Some(listener), _) => Box::new(desktop::serial::TcpSerial::new_listener(
                    listener.try_clone().unwrap(),
                    cli.no_response,
                )),
                (_, _, _, Some(addr)) => {
                    Box::new(desktop::serial::TcpSerial::connect(addr, cli.no_response))
                }
                (Some(fifo_input), Some(fifo_output), _, _) => Box::new(
                    desktop::serial::FIFOSerial::new(fifo_input, fifo_output, cli.no_response),
                ),
                _ => Box::new(desktop::serial::UnconnectedSerial {}),
            };

        let mut gamepad: Box<dyn Input> = if let Some(record_file) = cli.replay_input.clone() {
            Box::new(GamepadReplay::new(record_file))
        } else if cli.keyboard {
            Box::new(Keyboard::new(keys))
        } else {
            Box::new(Gamepad::new())
        };

        if let Some(record_file) = cli.record_input.clone() {
            gamepad = Box::new(GamepadRecorder::new(gamepad, record_file));
        };

        let mut fs_load_save = FSLoadSave::new(&cli.rom, format!("{}.sav", &cli.rom));
        if let Some(state_file) = &cli.state_file {
            fs_load_save = fs_load_save.state_file(state_file);
        }

        let mut gameboy = io::Gameboy::<_, _, _, _, _>::new(
            gamepad,
            window,
            serial,
            audio,
            fs_load_save,
            cli.speed as f64,
        );

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
        
        if !cli.restart_on_stop {
            break;
        }
    }
}
