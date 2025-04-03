use crate::consts::{PROGRAM_START_ADDRESS, STACK_START_ADDRESS};
use crate::display::Display;
use crate::audio::Channels;
use crate::io::{Serial, Audio};

pub mod reg {
    pub const B: u8 = 0;
    pub const C: u8 = 1;
    pub const D: u8 = 2;
    pub const E: u8 = 3;
    pub const H: u8 = 4;
    pub const L: u8 = 5;
    pub const A: u8 = 6;
    pub const F: u8 = 7;

    pub const BC: u8 = 0;
    pub const DE: u8 = 1;
    pub const HL: u8 = 2;
    pub const SP: u8 = 3;
}

pub mod flag {
    pub const NZ: u8 = 0;
    pub const Z: u8 = 1;
    pub const NC: u8 = 2;
    pub const C: u8 = 3;

    pub const CY: u8 = 1 << 4;
    pub const H: u8 = 1 << 5;
    pub const N: u8 = 1 << 6;
    pub const ZF: u8 = 1 << 7;
}

#[derive(Debug)]
pub struct CPU {
    /* B, C, D, E, H, L, A, F registers.
     * A is usually represented by 111 even though it's in index 6.
     * (HL) usually takes the 110 representation.
     * F isn't usually used by the 8bits register operations. */
    pub r: [u8; 8],

    pub pc: u16, // program counter
    pub sp: u16, // stack pointer

    pub dbg_cycle_counter: u64,
}

impl CPU {
    pub fn new() -> Self {
        Self {
            r: [0; 8],

            pc: PROGRAM_START_ADDRESS,
            sp: STACK_START_ADDRESS,

            dbg_cycle_counter: 0,
        }
    }

    pub fn r16(&self, r: u8) -> u16 {
        if r == reg::SP {
            return self.sp;
        } else {
            return self.r[r as usize * 2 + 1] as u16 | ((self.r[r as usize * 2] as u16) << 8);
        }
    }

    pub fn w16(&mut self, r: u8, value: u16) {
        if r == reg::SP {
            self.sp = value;
        } else {
            self.r[r as usize * 2 + 1] = (value & 0xff) as u8;
            self.r[r as usize * 2] = (value >> 8) as u8;
        }
    }

    pub fn check_flag(&self, flag: u8) -> bool {
        let f = self.r[reg::F as usize];

        match flag {
            flag::NZ => f >> 7 == 0,
            flag::Z => f >> 7 == 1,
            flag::NC => (f >> 4) & 1 == 0,
            flag::C => (f >> 4) & 1 == 1,
            _ => unimplemented!(),
        }
    }

    pub fn print_debug(&mut self) {
        println!(
            "PC: 0x{:04x}, SP: 0x{:04x}, A: 0x{:02x}, BC: 0x{:04x}, DE: 0x{:04x}, HL: 0x{:04x}, F: 0x{:02x}. Since last dbg: {} cycles",
            self.pc,
            self.sp,
            self.r[reg::A as usize],
            self.r16(reg::BC),
            self.r16(reg::DE),
            self.r16(reg::HL),
            self.r[reg::F as usize],
            self.dbg_cycle_counter,
        );

        self.dbg_cycle_counter = 0;
    }
}

pub struct Memory<S: Serial, A: Audio> {
    pub boot_rom: [u8; 0x900],

    pub cgb_mode: bool,

    pub bgcram_pointer: u8,

    pub bgcram_pointer_autoincrement: bool,

    pub obcram_pointer: u8,

    pub obcram_pointer_autoincrement: bool,

    pub boot_rom_on: bool,

    pub rom_bank: u8,

    pub ram_bank: u8,

    pub ram_bank_enabled: bool,

    // 32 KiB ROM bank 00
    pub rom: [u8; 0x200000],

    // 4 KiB Work RAM 00
    wram_00: [u8; 0x1000],

    // 4 KiB Work RAM 00
    wram_01: [u8; 0x1000],

    // External RAM
    pub external_ram: [u8; 0x8000],

    // 8 KiB Video RAM
    pub display: Display,

    pub io: [u8; 0x80],

    // High RAM
    hram: [u8; 0x7f],

    pub audio: Channels<A>,

    pub serial: S,

    pub ime: bool,

    pub div: u8,

    pub joypad_reg: u8,

    pub joypad_is_action: bool,

    pub interrupts_register: u8,

    pub halt: bool,

    pub tima: u8,

    pub tma: u8,

    pub timer_enabled: bool,

    pub timer_speed: u8,

    pub serial_data: u8,

    pub serial_control: u8,
}

#[derive(Debug)]
pub enum MemError {
    WritingToROM,
    Unimplemented,
    NotUsable,
}

mod serial_control_flags {
    pub const CLOCK_SELECT: u8 = 0b1;
    pub const TRANSFER_ENABLE: u8 = 0b10000000;
}

impl<S: Serial, A: Audio> Memory<S, A> {
    pub fn new(serial: S) -> Self {
        let mut display = Display::new();

        display.cls();

        Self {
            boot_rom: [0; 0x900],
            boot_rom_on: true,
            cgb_mode: false,
            bgcram_pointer: 0,
            bgcram_pointer_autoincrement: false,
            obcram_pointer: 0,
            obcram_pointer_autoincrement: false,
            rom_bank: 1,
            ram_bank: 0,
            ram_bank_enabled: false,
            rom: [0; 0x200000],
            wram_00: [0; 0x1000],
            wram_01: [0; 0x1000],
            external_ram: [0; 0x8000],
            display,
            io: [0; 0x80],
            hram: [0; 0x7f],
            audio: Channels::new(),
            ime: false,
            interrupts_register: 0,
            joypad_is_action: false,
            joypad_reg: 0,
            div: 0,
            halt: false,
            tima: 0,
            tma: 0,
            timer_enabled: false,
            timer_speed: 0,
            serial,
            serial_control: 0,
            serial_data: 0,
        }
    }

    pub fn update_serial(&mut self) {
        if self.serial.new_transfer() {
            println!("TRANSFER ENABLED");
            self.serial_control |= serial_control_flags::TRANSFER_ENABLE;
        }

        if self.serial.clock_master() {
            self.serial_control |= serial_control_flags::CLOCK_SELECT;
        } else {
            self.serial_control &= !serial_control_flags::CLOCK_SELECT;
        }

        if (self.serial_control & serial_control_flags::CLOCK_SELECT != 0
            || self.serial.new_transfer())
            && self.serial_control & serial_control_flags::TRANSFER_ENABLE != 0
        {
            self.serial.write(self.serial_data);
            self.serial_data = self.serial.read();
            self.serial_control &= !serial_control_flags::TRANSFER_ENABLE;
            self.io[0x0f] |= 0b1000;
        }
    }

    pub fn r(&self, addr: u16) -> Result<u8, MemError> {
        if (addr < 0x100 || (addr >= 0x200 && addr < 0x900)) && self.boot_rom_on {
            Ok(self.boot_rom[addr as usize])
        } else if addr < 0x4000 {
            Ok(self.rom[addr as usize])
        } else if addr < 0x8000 {
            Ok(self.rom[self.rom_bank as usize * 0x4000 + addr as usize - 0x4000 as usize])
        } else if addr >= 0xa000 && addr < 0xc000 {
            if self.ram_bank_enabled {
                Ok(self.external_ram[self.ram_bank as usize * 0x2000 + addr as usize - 0xa000])
            } else {
                Ok(0xff)
            }
        } else if addr >= 0xc000 && addr < 0xd000 {
            Ok(self.wram_00[addr as usize - 0xc000])
        } else if addr >= 0xd000 && addr < 0xe000 {
            Ok(self.wram_01[addr as usize - 0xd000])
        } else if (addr >= 0x8000 && addr < 0xa000) || (addr >= 0xfe00 && addr < 0xfea0) {
            self.display.r(addr & !0x8000)
        } else if addr >= 0xff00 && addr < 0xff80 {
            Ok(self.r_io((addr & 0xff) as u8))
        } else if addr >= 0xff80 && addr < 0xffff {
            Ok(self.hram[addr as usize - 0xff80])
        } else if addr == 0xffff {
            Ok(self.interrupts_register)
        } else {
            println!(
                "Trying to read at address 0x{:04x} which is unimplemented",
                addr
            );
            Ok(0) //Err(MemError::Unimplemented)
        }
    }

    pub fn w(&mut self, addr: u16, value: u8) -> Result<(), MemError> {
        if addr < 0x2000 {
            self.ram_bank_enabled = value == 0x0a;
            Ok(())
        } else if addr >= 0x2000 && addr < 0x4000 {
            if value == 0 {
                self.rom_bank = 1
            } else {
                self.rom_bank = value & 0b1111111;
            }
            Ok(())
        } else if addr >= 0x4000 && addr < 0x6000 {
            self.ram_bank = value & 0b11;
            Ok(())
        } else if addr >= 0xa000 && addr < 0xc000 {
            self.external_ram[self.ram_bank as usize * 0x2000 + addr as usize - 0xa000] = value;
            Ok(())
        } else if addr >= 0xc000 && addr < 0xd000 {
            self.wram_00[addr as usize - 0xc000] = value;
            Ok(())
        } else if addr >= 0xd000 && addr < 0xe000 {
            self.wram_01[addr as usize - 0xd000] = value;
            Ok(())
        } else if (addr >= 0x8000 && addr < 0xa000) || (addr >= 0xfe00 && addr < 0xfea0) {
            self.display.w(addr & !0x8000, value)
        } else if addr >= 0xff00 && addr < 0xff80 {
            Ok(self.w_io((addr & 0xff) as u8, value)?)
        } else if addr >= 0xff80 && addr < 0xffff {
            self.hram[addr as usize - 0xff80] = value;
            Ok(())
        } else if addr == 0xffff {
            self.interrupts_register = value;
            Ok(())
        } else {
            println!(
                "Trying to write at address 0x{:04x} which is unimplemented (value: {:02x})",
                addr, value
            );
            Ok(()) //Err(MemError::Unimplemented)
        }
    }
}

pub struct GBState<S: Serial, A: Audio> {
    pub cpu: CPU,
    pub mem: Memory<S, A>,
    pub is_debug: bool,

    pub div_cycles: u64,
    pub tima_cycles: u64,
}

impl<S: Serial, A: Audio> GBState<S, A> {
    pub fn new(serial: S) -> Self {
        let mem = Memory::new(serial);

        Self {
            cpu: CPU::new(),
            mem,
            is_debug: false,

            div_cycles: 0,
            tima_cycles: 0,
        }
    }

    pub fn r_reg(&self, r_i: u8) -> Result<u8, MemError> {
        if r_i < 6 {
            Ok(self.cpu.r[r_i as usize])
        } else if r_i == 7 {
            Ok(self.cpu.r[6])
        } else if r_i == 6 {
            self.mem.r(self.cpu.r16(reg::HL))
        } else {
            panic!("r_i must be a 3 bits register input number")
        }
    }

    pub fn w_reg(&mut self, r_i: u8, value: u8) -> Result<(), MemError> {
        if r_i < 6 {
            self.cpu.r[r_i as usize] = value;
        } else if r_i == 7 {
            self.cpu.r[6] = value;
        } else if r_i == 6 {
            self.mem.w(self.cpu.r16(reg::HL), value)?;
        } else {
            panic!("r_i must be a 3 bits register input number")
        }
        Ok(())
    }

    pub fn debug(&self, s: &str) -> () {
        if self.is_debug {
            println!("{}", s);
        }
    }
}
