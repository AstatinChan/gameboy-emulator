pub mod audio;
pub mod consts;
pub mod display;
pub mod gamepad;
pub mod interrupts_timers;
pub mod io;
pub mod opcodes;
pub mod serial;
pub mod state;
pub mod window;

use crate::gamepad::{Gamepad, GamepadRecorder, GamepadReplay, Input, Keyboard};
use crate::state::GBState;
use clap::Parser;
use std::time::SystemTime;
use std::{thread, time};

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

    #[arg(short, long, default_value_t = false)]
    keyboard: bool,

    #[arg(short, long, default_value_t = 1.0)]
    speed: f32,
}

fn main() {
    let cli = Cli::parse();
    let mut total_cycle_counter: u128 = 0;

    println!("Initializing Gamepad...");

    println!("Starting {:?}...", &cli.rom);

    let mut state = match (cli.fifo_input, cli.fifo_output) {
        (Some(fifo_input), Some(fifo_output)) => {
            GBState::new(Box::new(serial::FIFOSerial::new(fifo_input, fifo_output)))
        }
        (None, None) => GBState::new(Box::new(serial::UnconnectedSerial {})),
        _ => panic!("If using fifo serial, both input and output should be set"),
    };

    let save_file = format!("{}.sav", &cli.rom);

    state.mem.load_rom(&cli.rom).unwrap();

    if let Err(_) = state.mem.load_external_ram(&save_file) {
        println!(
            "\"{}\" not found. Initializing new external ram.",
            save_file
        );
    }

    let mut window = window::Window::new().unwrap();

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

    let mut nanos_sleep: i128 = 0;
    let mut halt_time = 0;
    let mut was_previously_halted = false;

    let mut last_ram_bank_enabled = false;
    let mut now = SystemTime::now();

    loop {
        if was_previously_halted && !state.mem.halt {
            println!("Halt cycles {}", halt_time);
            halt_time = 0;
        }
        was_previously_halted = state.mem.halt;
        let c = if !state.mem.halt {
            state.exec_opcode().unwrap()
        } else {
            halt_time += 4;
            4
        };

        state.cpu.dbg_cycle_counter += c;
        total_cycle_counter += c as u128;

        state.div_timer(c);
        state.tima_timer(c);
        state.update_display_interrupts(c);
        state.check_interrupts().unwrap();
        state.mem.update_serial();

        nanos_sleep += c as i128 * (consts::CPU_CYCLE_LENGTH_NANOS as f32 / cli.speed) as i128;
        if nanos_sleep > 0 {
            gamepad.update_events(total_cycle_counter, &state);

            let (action_button_reg, direction_button_reg) = (
                gamepad.get_action_gamepad_reg(),
                gamepad.get_direction_gamepad_reg(),
            );

            if let Some(fb) = state.mem.display.redraw_request {
                if let Some(window::WindowSignal::Exit) = window.update(&fb) {
                    break;
                }
            }
            // gamepad.check_special_actions(&mut state.is_debug);

            if state.mem.joypad_is_action
                && (action_button_reg & (state.mem.joypad_reg >> 4)) != (state.mem.joypad_reg >> 4)
                || (!state.mem.joypad_is_action
                    && (direction_button_reg & state.mem.joypad_reg & 0b1111)
                        != (state.mem.joypad_reg & 0b1111))
            {
                state.mem.io[0x0f] |= 0b10000;
            }

            state.mem.joypad_reg = direction_button_reg | (action_button_reg << 4);

            if !cli.loop_lock_timing {
                thread::sleep(time::Duration::from_nanos(nanos_sleep as u64 / 10));
            } else {
                while SystemTime::now().duration_since(now).unwrap().as_nanos()
                    < nanos_sleep as u128
                {
                    for _ in 0..100_000_000 {}
                }
            }

            nanos_sleep =
                nanos_sleep - SystemTime::now().duration_since(now).unwrap().as_nanos() as i128;
            now = SystemTime::now();

            if last_ram_bank_enabled && !state.mem.ram_bank_enabled {
                println!("Saving to \"{}\"...", save_file);

                if let Err(_) = state.mem.save_external_ram(&save_file) {
                    println!("Failed to save external RAM");
                }
            }
            last_ram_bank_enabled = state.mem.ram_bank_enabled;
        }
    }
}
