use std::time::SystemTime;
use std::{thread, time};

use crate::state::GBState;
use crate::consts;

pub trait Input {
    fn update_events(&mut self, cycles: u128) -> Option<u128>;
    fn get_action_gamepad_reg(&self) -> u8;
    fn get_direction_gamepad_reg(&self) -> u8;
}

impl<T: Input + ?Sized> Input for Box<T> {
    fn update_events(&mut self, cycles: u128) -> Option<u128> {
        (**self).update_events(cycles)
    }
    fn get_action_gamepad_reg(&self) -> u8 {
        (**self).get_action_gamepad_reg()
    }
    fn get_direction_gamepad_reg(&self) -> u8 {
        (**self).get_direction_gamepad_reg()
    }
}

pub enum WindowSignal {
    Exit,
}

pub trait Window {
    fn update(&mut self, fb: &[u32; 160 * 144]) -> Option<WindowSignal>;
}

pub trait Serial {
    // Should not be blocking
    fn write(&mut self, byte: u8);
    fn read(&mut self) -> u8;

    fn new_transfer(&mut self) -> bool; // since last read
    fn clock_master(&mut self) -> bool;

    fn set_clock_master(&mut self, clock_master: bool);
}

pub trait Audio {
    fn new<S: Iterator<Item = f32> + Send + 'static>(wave: S) -> Self;
}

pub trait LoadSave where Self::Error: std::fmt::Display, Self::Error: std::fmt::Debug {
    type Error;
    fn load_bootrom(&self, boot_rom: &mut [u8]) -> Result<(), Self::Error>;
    fn load_rom(&self, rom: &mut [u8]) -> Result<(), Self::Error>;
    fn load_external_ram(&self, external_ram: &mut [u8]) -> Result<(), Self::Error>;
    fn save_external_ram(&self, external_ram: &[u8]) -> Result<(), Self::Error>;
}

pub struct Gameboy<
    I: Input,
    W: Window,
    S: Serial,
    A: Audio,
    LS: LoadSave,
> {
    input: I,
    window: W,
    speed: f64,
    state: GBState<S, A>,
    load_save: LS,
}

impl<I: Input, W: Window, S: Serial, A: Audio, LS: LoadSave> Gameboy<I, W, S, A, LS> {
    pub fn new(input: I, window: W, serial: S, load_save: LS, speed: f64) -> Self {
        Self {
            input,
            window,
            speed,
            state: GBState::<S, A>::new(serial),
            load_save,
        }
    }

    
    pub fn start(self) {
        let Self {
            mut window,
            mut input,
            speed,
            mut state,
            load_save,
        } = self;

        load_save.load_bootrom(&mut state.mem.boot_rom).unwrap();
        load_save.load_rom(&mut state.mem.rom).unwrap();

        if let Err(err) = load_save.load_external_ram(&mut state.mem.external_ram) {
            println!(
                "Loading save failed ({}). Initializing new external ram.",
                err
            );
        }
        let mut total_cycle_counter: u128 = 0;
        let mut nanos_sleep: f64 = 0.0;
        let mut halt_time = 0;
        let mut was_previously_halted = false;

        let mut last_ram_bank_enabled = false;
        let mut now = SystemTime::now();
        let mut next_precise_gamepad_update: Option<u128> = None;

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

            nanos_sleep += c as f64 * (consts::CPU_CYCLE_LENGTH_NANOS as f64 / speed) as f64;

            if nanos_sleep >= 0.0 || next_precise_gamepad_update.map_or(false, |c| (c >= total_cycle_counter)) {
                next_precise_gamepad_update = input.update_events(total_cycle_counter);

                let (action_button_reg, direction_button_reg) = (
                    input.get_action_gamepad_reg(),
                    input.get_direction_gamepad_reg(),
                );

                if state.mem.joypad_is_action
                    && (action_button_reg & (state.mem.joypad_reg >> 4)) != (state.mem.joypad_reg >> 4)
                    || (!state.mem.joypad_is_action
                        && (direction_button_reg & state.mem.joypad_reg & 0b1111)
                            != (state.mem.joypad_reg & 0b1111))
                {
                    state.mem.io[0x0f] |= 0b10000;
                }

                state.mem.joypad_reg = direction_button_reg | (action_button_reg << 4);
            }


            if nanos_sleep > 0.0 {
                if let Some(fb) = state.mem.display.redraw_request {
                    if let Some(WindowSignal::Exit) = window.update(&fb) {
                        break;
                    }
                }

                thread::sleep(time::Duration::from_nanos(nanos_sleep as u64 / 10));

                nanos_sleep =
                    nanos_sleep - SystemTime::now().duration_since(now).unwrap().as_nanos() as f64;
                now = SystemTime::now();

                if last_ram_bank_enabled && !state.mem.ram_bank_enabled {
                    if let Err(err) = load_save.save_external_ram(&state.mem.external_ram) {
                        println!("Failed to save external RAM ({})", err);
                    }
                }
                last_ram_bank_enabled = state.mem.ram_bank_enabled;
            }
        }
    }
}
