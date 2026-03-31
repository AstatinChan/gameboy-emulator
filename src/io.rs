#[cfg(target_family = "wasm")]
use crate::wasm::utils::SystemTime;
#[cfg(not(target_family = "wasm"))]
use std::thread;
#[cfg(not(target_family = "wasm"))]
use std::time::{Duration, SystemTime};

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
    fn close_serial(&mut self);
}

impl<T: Serial + ?Sized> Serial for Box<T> {
    fn read_data(&self) -> u8 {
        (**self).read_data()
    }
    fn read_control(&self) -> u8 {
        (**self).read_control()
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

    fn close_serial(&mut self) {
        (**self).close_serial()
    }
}

pub trait Wave {
    fn next(&mut self, left: bool) -> Option<f32>;
}

pub trait Audio {
    fn attach_wave(&mut self, wave: MutableWave);
    fn next(&mut self);
}

impl<T: Audio + ?Sized> Audio for Box<T> {
    fn attach_wave(&mut self, wave: MutableWave) {
        (**self).attach_wave(wave)
    }

    fn next(&mut self) {
        (**self).next()
    }
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

pub struct Gameboy<I: Input, S: Serial, A: Audio, LS: LoadSave> {
    input: I,
    speed: f64,
    state: GBState<S, A>,
    load_save: LS,
    total_cycle_counter: u128,
    pub nanos_sleep: f64,
    halt_time: u64,
    audio_counter: u64,
    was_previously_halted: bool,

    last_ram_bank_enabled: bool,
    now: SystemTime,
    last_halt_cycle: SystemTime,
    last_halt_cycle_counter: u128,
    next_precise_gamepad_update: Option<u128>,
}

impl<I: Input, S: Serial, A: Audio, LS: LoadSave> Gameboy<I, S, A, LS> {
    pub fn new(input: I, serial: S, audio: A, load_save: LS, speed: f64) -> Self {
        let mut gb = Self {
            input,
            speed,
            state: GBState::<S, A>::new(serial, audio),
            load_save,
            total_cycle_counter: 0,
            nanos_sleep: 0.0,
            halt_time: 0,
            audio_counter: 0,
            was_previously_halted: false,

            last_ram_bank_enabled: false,
            now: SystemTime::now(),
            last_halt_cycle: SystemTime::now(),
            last_halt_cycle_counter: 0,
            next_precise_gamepad_update: None,
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

    pub fn update_joypad(&mut self) {
        self.next_precise_gamepad_update = self.input.update_events(self.total_cycle_counter);

        let (action_button_reg, direction_button_reg, save_state) = (
            self.input.get_action_gamepad_reg(),
            self.input.get_direction_gamepad_reg(),
            self.input.save_state(),
        );

        if save_state {
            if let Err(err) = self.load_save.save_state(&self.state) {
                elog(LogLevel::Error, format!("Failed save state: {:?}", err));
            }
        }

        if self.state.mem.joypad_is_action
            && (action_button_reg & (self.state.mem.joypad_reg >> 4))
                != (self.state.mem.joypad_reg >> 4)
            || (!self.state.mem.joypad_is_action
                && (direction_button_reg & self.state.mem.joypad_reg & 0b1111)
                    != (self.state.mem.joypad_reg & 0b1111))
        {
            self.state.mem.io[0x0f] |= 0b10000;
        }

        self.state.mem.joypad_reg = direction_button_reg | (action_button_reg << 4);
    }

    pub fn external_ram_save(&mut self) {
        if self.last_ram_bank_enabled && !self.state.mem.ram_bank_enabled {
            if let Err(err) = self
                .load_save
                .save_external_ram(self.state.mem.external_ram.as_ref())
            {
                elog(
                    LogLevel::Error,
                    format!("Failed to save external RAM ({})", err),
                );
            }
        }
        self.last_ram_bank_enabled = self.state.mem.ram_bank_enabled;
    }

    pub fn run_instr(&mut self) -> u64 {
        if self.was_previously_halted && !self.state.mem.halt {
            let n = SystemTime::now();
            log(
                LogLevel::HaltCycles,
                format!(
                    "Halt cycles {} (system average speed: {}Hz)",
                    self.halt_time,
                    self.last_halt_cycle_counter as f32
                        / n.duration_since(self.last_halt_cycle)
                            .unwrap()
                            .as_secs_f32(),
                ),
            );
            self.halt_time = 0;
        }
        self.was_previously_halted = self.state.mem.halt;
        let c = if !self.state.mem.halt {
            self.state.exec_opcode()
        } else {
            self.halt_time += 4;
            4
        };

        self.last_halt_cycle_counter += c as u128;
        self.state.cpu.dbg_cycle_counter += c;
        self.total_cycle_counter += c as u128;
        self.audio_counter += c;

        if self
            .next_precise_gamepad_update
            .map_or(false, |c| c >= self.total_cycle_counter)
        {
            self.update_joypad();
        }

        if self.audio_counter >= 32 {
            self.audio_counter -= 32;
            self.state.mem.audio.next();
        }

        self.state.div_timer(c);
        self.state.tima_timer(c);
        self.state.update_display_interrupts(c);
        self.state.check_interrupts();
        self.state.mem.update_serial(self.total_cycle_counter);

        return c;
    }

    pub fn run_until_next_sleep(&mut self) -> bool {
        self.update_joypad();
        self.external_ram_save();
        while !self.state.is_stopped {
            let c = self.run_instr();
            self.nanos_sleep += c as f64 * (consts::CPU_CYCLE_LENGTH_NANOS / self.speed) as f64;
            if self.nanos_sleep > 0.0 {
                return true;
            }
        }
        self.state.mem.serial.close_serial();
        return false;
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn sleep_and_draw(&mut self) -> Option<Box<[u32; 160 * 144]>> {
        thread::sleep(Duration::from_nanos(self.nanos_sleep as u64));

        let new_now = SystemTime::now();
        self.nanos_sleep =
            self.nanos_sleep - new_now.duration_since(self.now).unwrap().as_nanos() as f64;
        self.now = new_now;
        return self.state.mem.display.get_redraw_request();
    }

    #[cfg(target_family = "wasm")]
    pub fn sleep_and_draw(&mut self) -> Option<Box<[u32; 160 * 144]>> {
        let new_now = SystemTime::now();
        self.nanos_sleep =
            self.nanos_sleep - new_now.duration_since(self.now).unwrap().as_nanos() as f64;
        if self.nanos_sleep < -1_000_000_000. {
            self.nanos_sleep = self.nanos_sleep % 1_000_000_000. - 1_000_000_000.;
        }
        self.now = new_now;
        return self.state.mem.display.get_redraw_request();
    }
}
