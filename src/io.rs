use std::time::SystemTime;
use std::{thread, time};

use crate::audio::MutableWave;
use crate::consts;
use crate::logs::{elog, log, LogLevel};
use crate::state::GBState;

pub trait Input {
    fn update_events(&mut self, cycles: u128) -> Option<u128>;
    fn get_action_gamepad_reg(&self) -> u8;
    fn get_direction_gamepad_reg(&self) -> u8;
    fn save_state(&mut self) -> bool;
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
    fn save_state(&mut self) -> bool {
        (**self).save_state()
    }
}

pub enum WindowSignal {
    Exit,
}

pub trait Window {
    fn update(&mut self, fb: Box<[u32; 160 * 144]>) -> Option<WindowSignal>;
}

impl<T: Window + ?Sized> Window for Box<T> {
    fn update(&mut self, fb: Box<[u32; 160 * 144]>) -> Option<WindowSignal> {
        (**self).update(fb)
    }
}

pub trait Serial {
    fn read_data(&self) -> u8;
    fn read_control(&self) -> u8;
    fn write_data(&mut self, data: u8);
    fn write_control(&mut self, control: u8);
    fn update_serial(&mut self, cycles: u128) -> bool;
}

impl<T: Serial + ?Sized> Serial for Box<T> {
    fn read_data(&self) -> u8 {
        (**self).read_data()
    }
    fn read_control(&self) -> u8 {
        (**self).read_data()
    }
    fn write_data(&mut self, data: u8) {
        (**self).write_data(data);
    }
    fn write_control(&mut self, control: u8) {
        (**self).write_control(control);
    }
    fn update_serial(&mut self, cycles: u128) -> bool {
        (**self).update_serial(cycles)
    }
}

pub trait Wave {
    fn next(&mut self, left: bool) -> Option<f32>;
}

pub trait Audio {
    fn new(wave: MutableWave) -> Self;
    fn next(&mut self);
}

pub trait LoadSave
where
    Self::Error: std::fmt::Display,
    Self::Error: std::fmt::Debug,
{
    type Error;
    fn load_bootrom(&self, boot_rom: &mut [u8]) -> Result<(), Self::Error>;
    fn load_rom(&self, rom: &mut [u8]) -> Result<(), Self::Error>;
    fn load_external_ram(&self, external_ram: &mut [u8]) -> Result<(), Self::Error>;
    fn save_external_ram(&self, external_ram: &[u8]) -> Result<(), Self::Error>;
    fn dump_state<S: Serial, A: Audio>(&self, state: &GBState<S, A>) -> Result<(), Self::Error>;
    fn save_state<S: Serial, A: Audio>(&self, state: &GBState<S, A>) -> Result<(), Self::Error>;
    fn load_state<S: Serial, A: Audio>(&self, state: &mut GBState<S, A>)
        -> Result<(), Self::Error>;
}

pub struct Gameboy<I: Input, W: Window, S: Serial, A: Audio, LS: LoadSave> {
    input: I,
    window: W,
    speed: f64,
    state: GBState<S, A>,
    load_save: LS,
}

impl<I: Input, W: Window, S: Serial, A: Audio, LS: LoadSave> Gameboy<I, W, S, A, LS> {
    pub fn new(input: I, window: W, serial: S, load_save: LS, speed: f64) -> Self {
        let mut gb = Self {
            input,
            window,
            speed,
            state: GBState::<S, A>::new(serial),
            load_save,
        };

        gb.load_save
            .load_bootrom(gb.state.mem.boot_rom.as_mut())
            .unwrap();
        gb.load_save.load_rom(gb.state.mem.rom.as_mut()).unwrap();

        if let Err(err) = gb
            .load_save
            .load_external_ram(gb.state.mem.external_ram.as_mut())
        {
            log(
                LogLevel::Infos,
                format!(
                    "Loading save failed ({}). Initializing new external ram.",
                    err
                ),
            );
        }

        gb
    }

    pub fn load_state(&mut self) -> Result<(), LS::Error> {
        self.load_save.load_state(&mut self.state)?;
        Ok(())
    }

    pub fn dump_state(&mut self) -> Result<(), LS::Error> {
        self.load_save.dump_state(&mut self.state)?;
        Ok(())
    }

    pub fn skip_bootrom(&mut self) {
        self.state.mem.boot_rom_on = false;
        self.state.cpu.pc = 0x100;
    }

    pub fn start(&mut self) {
        let Self {
            ref mut window,
            ref mut input,
            ref speed,
            ref mut state,
            ref load_save,
        } = self;

        let mut total_cycle_counter: u128 = 0;
        let mut nanos_sleep: f64 = 0.0;
        let mut halt_time = 0;
        let mut audio_counter = 0;
        let mut was_previously_halted = false;

        let mut last_ram_bank_enabled = false;
        let mut now = SystemTime::now();
        let mut last_halt_cycle = now;
        let mut last_halt_cycle_counter: u128 = 0;
        let mut next_precise_gamepad_update: Option<u128> = None;

        while !state.is_stopped {
            if was_previously_halted && !state.mem.halt {
                let n = SystemTime::now();
                log(
                    LogLevel::HaltCycles,
                    format!(
                        "Halt cycles {} (system average speed: {}Hz)",
                        halt_time,
                        last_halt_cycle_counter as f32 / n.duration_since(last_halt_cycle).unwrap().as_secs_f32(),
                    )
                );
                halt_time = 0;
            }
            was_previously_halted = state.mem.halt;
            let c = if !state.mem.halt {
                state.exec_opcode()
            } else {
                halt_time += 4;
                4
            };

            last_halt_cycle_counter += c as u128;
            state.cpu.dbg_cycle_counter += c;
            total_cycle_counter += c as u128;
            audio_counter += c;

            if audio_counter >= 32 {
                audio_counter -= 32;
                state.mem.audio.next();
            }

            state.div_timer(c);
            state.tima_timer(c);
            state.update_display_interrupts(c);
            state.check_interrupts();
            state.mem.update_serial(total_cycle_counter);

            nanos_sleep += c as f64 * (consts::CPU_CYCLE_LENGTH_NANOS / *speed) as f64;

            if nanos_sleep >= 0.0
                || next_precise_gamepad_update.map_or(false, |c| (c >= total_cycle_counter))
            {
                next_precise_gamepad_update = input.update_events(total_cycle_counter);

                let (action_button_reg, direction_button_reg, save_state) = (
                    input.get_action_gamepad_reg(),
                    input.get_direction_gamepad_reg(),
                    input.save_state(),
                );

                if save_state {
                    if let Err(err) = load_save.save_state(&state) {
                        elog(LogLevel::Error, format!("Failed save state: {:?}", err));
                    }
                }

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
            }

            if nanos_sleep > 0.0 {
                if let Some(fb) = state.mem.display.get_redraw_request() {
                    if let Some(WindowSignal::Exit) = window.update(fb) {
                        break;
                    }
                }

                thread::sleep(time::Duration::from_nanos(1)); //nanos_sleep as u64));

                let new_now = SystemTime::now();
                nanos_sleep =
                    nanos_sleep - new_now.duration_since(now).unwrap().as_nanos() as f64;
                now = new_now;

                if last_ram_bank_enabled && !state.mem.ram_bank_enabled {
                    if let Err(err) = load_save.save_external_ram(state.mem.external_ram.as_ref()) {
                        elog(
                            LogLevel::Error,
                            format!("Failed to save external RAM ({})", err),
                        );
                    }
                }
                last_ram_bank_enabled = state.mem.ram_bank_enabled;
            }
        }
    }
}
