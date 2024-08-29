pub mod audio;
pub mod consts;
pub mod display;
pub mod gamepad;
pub mod interrupts_timers;
pub mod io;
pub mod opcodes;
pub mod state;

use crate::gamepad::Gamepad;
use crate::state::{GBState, MemError};
use clap::Parser;
use std::time::SystemTime;
use std::{thread, time};

pub fn exec_opcode(state: &mut GBState) -> Result<u64, MemError> {
    let opcode = state.mem.r(state.cpu.pc)?;

    if state.is_debug {
        println!(
            "{:02x}:{:04x} = {:02x} (IME: {})",
            state.mem.rom_bank, state.cpu.pc, opcode, state.mem.ime
        );
    }

    state.cpu.pc += 1;

    let n1 = (opcode >> 3) & 0b111;
    let n2 = opcode & 0b111;

    match opcode >> 6 {
        0b00 => opcodes::op00(state, n1, n2),
        0b01 => opcodes::op01(state, n1, n2),
        0b10 => opcodes::op10(state, n1, n2),
        0b11 => opcodes::op11(state, n1, n2),
        _ => panic!(),
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The gameboy rom file
    rom: String,

    /// Setting this saves battery by using thread::sleep instead of spin_sleeping. It can result in lag and inconsistent timing.
    #[arg(long)]
    thread_sleep: bool,

    #[arg(short, long, default_value_t = 1.0)]
    speed: f32,
}

fn main() {
    let cli = Cli::parse();

    println!("Initializing Gamepad...");

    let mut gamepad = Gamepad::new();

    println!("Starting {:?}...", &cli.rom);

    let mut state = GBState::new();

    let save_file = format!("{}.sav", &cli.rom);

    state.mem.load_rom(&cli.rom).unwrap();

    if let Err(_) = state.mem.load_external_ram(&save_file) {
        println!(
            "\"{}\" not found. Initializing new external ram.",
            save_file
        );
    }

    let mut nanos_sleep: i128 = 0;
    let mut halt_time = 0;
    let mut was_previously_halted = false;

    let mut last_ram_bank_enabled = false;

    loop {
        if was_previously_halted && !state.mem.halt {
            println!("Halt cycles {}", halt_time);
            halt_time = 0;
        }
        was_previously_halted = state.mem.halt;
        let now = SystemTime::now();
        let c = if !state.mem.halt {
            exec_opcode(&mut state).unwrap()
        } else {
            halt_time += 4;
            4
        };

        state.div_timer(c);
        state.tima_timer(c);
        state.update_display_interrupts(c);
        state.check_interrupts().unwrap();

        nanos_sleep += c as i128 * (consts::CPU_CYCLE_LENGTH_NANOS as f32 / cli.speed) as i128;
        if nanos_sleep > 0 {
            gamepad.update_events();

            let action_button_reg = gamepad.get_action_gamepad_reg();
            let direction_button_reg = gamepad.get_direction_gamepad_reg();
            gamepad.check_special_actions(&mut state);

            if state.mem.joypad_is_action
                && (action_button_reg & (state.mem.joypad_reg >> 4))
                    != (state.mem.joypad_reg >> 4)
                || (!state.mem.joypad_is_action
                    && (direction_button_reg & state.mem.joypad_reg & 0b1111)
                        != (state.mem.joypad_reg & 0b1111))
            {
                state.mem.io[0x0f] |= 0b10000;
            }

            state.mem.joypad_reg = direction_button_reg | (action_button_reg << 4);

            if cli.thread_sleep {
                thread::sleep(time::Duration::from_nanos(nanos_sleep as u64 / 10));
            } else {
                while SystemTime::now().duration_since(now).unwrap().as_nanos()
                    < nanos_sleep as u128
                {}
            }

            nanos_sleep =
                nanos_sleep - SystemTime::now().duration_since(now).unwrap().as_nanos() as i128;

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
