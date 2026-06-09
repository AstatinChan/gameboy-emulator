use crate::io::{Audio, Serial};
use crate::logs::{log, LogLevel};
use crate::state::{GBState};

impl<S: Serial, A: Audio> GBState<S, A> {
    pub fn r_io(&self, addr: u8) -> u8 {
        if addr > 0x50 {
            log(
                LogLevel::Debug,
                format!("Reading from 0xff{:02x} not implemented yet", addr),
            );
        }
        match addr {
            0x00 => {
                if self.mem.joypad_is_action {
                    (self.mem.joypad_reg >> 4) | 0b11010000
                } else {
                    (self.mem.joypad_reg & 0xf) | 0b11100000
                }
            }
            0x01 => self.mem.serial.read_data(),
            0x02 => self.mem.serial.read_control(),
            0x04 => self.mem.div,
            0x0f => self.mem.io[0x0f],
            0x40 => self.mem.display.lcdc,
            0x42 => self.mem.display.viewport_y,
            0x43 => self.mem.display.viewport_x,
            0x41 => {
                let mut ret = 0b00001000 << self.mem.display.lcd_interrupt_mode;

                ret |= if self.mem.display.ly >= 0x90 {
                    1
                } else if self.mem.display.stat < 80 {
                    2
                } else if self.mem.display.stat < 280 {
                    3
                } else {
                    0
                };

                if self.mem.display.ly == self.mem.display.lyc + 1 {
                    ret |= 0b100;
                }

                ret
            }
            0x44 => self.mem.display.ly,
            0x45 => self.mem.display.lyc,
            0x47 => self.mem.display.bg_palette,
            0x48 => self.mem.display.obj_palettes[0],
            0x49 => self.mem.display.obj_palettes[1],
            0x4a => self.mem.display.window_y,
            0x4b => self.mem.display.window_x,
            0x50 => {
                if self.mem.boot_rom_on {
                    0xfe
                } else {
                    0xff
                }
            }
            _ => {
                log(
                    LogLevel::Debug,
                    format!("Reading from 0xff{:02x} not implemented yet", addr),
                );
                self.mem.io[addr as usize]
            }
        }
    }

    pub fn w_io(&mut self, addr: u8, value: u8) {
        match addr {
            0x00 => {
                self.mem.joypad_is_action = !value & 0b00100000 != 0;
            }
            0x01 => {
                self.mem.serial.write_data(value);
            }
            0x02 => {
                self.mem.serial.write_control(value);
            }
            0x04 => {
                self.mem.div = 0;
            }
            0x05 => {
                self.mem.tima = value;
            }
            0x06 => {
                self.mem.tma = value;
            }
            0x07 => {
                self.mem.timer_enabled = value & 0b100 != 0;
                self.mem.timer_speed = value & 0b11;
            }
            0x0f => {
                self.mem.io[0x0f] = value;
            }
            0x10 => {
                self.mem.audio.ch1.period_sweep_pace = (0b1110000 & value) >> 4;
                self.mem.audio.ch1.period_sweep_direction = (0b1000 & value) >> 3;
                self.mem.audio.ch1.period_sweep_slope = 0b111 & value;
                self.mem.audio.ch1.update(false);
            }
            0x11 => {
                self.mem.audio.ch1.duty = value >> 6;
                self.mem.audio.ch1.length_timer = value & 0b111111;
                self.mem.audio.ch1.update(false);
            }
            0x12 => {
                self.mem.audio.ch1.initial_volume = value >> 4;
                self.mem.audio.ch1.env_direction = (value & 0xf) >> 3;
                self.mem.audio.ch1.sweep = value & 0b111;
                self.mem.audio.ch1.update(false);
            }
            0x13 => {
                self.mem.audio.ch1.period_value &= 0xff00;
                self.mem.audio.ch1.period_value |= value as u16;
                self.mem.audio.ch1.update(false);
            }
            0x14 => {
                self.mem.audio.ch1.period_value &= 0xff;
                self.mem.audio.ch1.period_value |= ((value & 0b111) as u16) << 8;
                self.mem.audio.ch1.length_timer_enabled = value & 0b01000000 != 0;
                if value >> 7 == 1 {
                    self.mem.audio.ch1.update(true);
                } else {
                    self.mem.audio.ch1.update(false);
                }
            }
            0x16 => {
                self.mem.audio.ch2.duty = value >> 6;
                self.mem.audio.ch2.length_timer = value & 0b111111;
                self.mem.audio.ch2.update(false);
            }
            0x17 => {
                self.mem.audio.ch2.initial_volume = value >> 4;
                self.mem.audio.ch2.env_direction = (value & 0xf) >> 3;
                self.mem.audio.ch2.sweep = value & 0b111;
                self.mem.audio.ch2.update(false);
            }
            0x18 => {
                self.mem.audio.ch2.period_value &= 0xff00;
                self.mem.audio.ch2.period_value |= value as u16;
                self.mem.audio.ch2.update(false);
            }
            0x19 => {
                self.mem.audio.ch2.period_value &= 0xff;
                self.mem.audio.ch2.period_value |= ((value & 0b111) as u16) << 8;
                self.mem.audio.ch2.length_timer_enabled = value & 0b01000000 != 0;
                if value >> 7 == 1 {
                    self.mem.audio.ch2.update(true);
                } else {
                    self.mem.audio.ch2.update(false);
                }
            }
            0x1a => {
                if value & 0b10000000 != 0 {
                    self.mem.audio.ch3.on = true;
                } else {
                    self.mem.audio.ch3.on = false;
                }
                self.mem.audio.ch3.update(true);
            }
            0x1b => {
                self.mem.audio.ch3.length_timer = value & 0b111111;
                self.mem.audio.ch3.update(false);
            }
            0x1c => {
                let s = (value >> 5) & 0b11;
                if s == 0 {
                    self.mem.audio.ch3.initial_volume = 0;
                } else {
                    self.mem.audio.ch3.initial_volume = 0xf >> (s - 1);
                }
                self.mem.audio.ch3.update(false);
            }
            0x1d => {
                self.mem.audio.ch3.period_value &= 0xff00;
                self.mem.audio.ch3.period_value |= value as u16;
                self.mem.audio.ch3.update(false);
            }
            0x1e => {
                self.mem.audio.ch3.period_value &= 0xff;
                self.mem.audio.ch3.period_value |= ((value & 0b111) as u16) << 8;
                self.mem.audio.ch3.period_value /= 2;
                self.mem.audio.ch3.length_timer_enabled = value & 0b01000000 != 0;
                if value >> 7 == 1 {
                    self.mem.audio.ch3.update(true);
                } else {
                    self.mem.audio.ch3.update(false);
                }
            }
            0x20 => {
                self.mem.audio.ch4.length_timer = value & 0b111111;
                self.mem.audio.ch4.update(false);
            }
            0x21 => {
                self.mem.audio.ch4.initial_volume = value >> 4;
                self.mem.audio.ch4.env_direction = (value & 0xf) >> 3;
                self.mem.audio.ch4.sweep = value & 0b111;
                self.mem.audio.ch4.update(false);
            }
            0x22 => {
                self.mem.audio.ch4.clock_shift = value >> 4;
                self.mem.audio.ch4.lsfr_width = (value & 0xf) >> 3;
                self.mem.audio.ch4.clock_divider = value & 0b111;
                self.mem.audio.ch4.update(false);
            }
            0x23 => {
                self.mem.audio.ch4.length_timer_enabled = value & 0b01000000 != 0;
                if value >> 7 == 1 {
                    self.mem.audio.ch4.update(true);
                } else {
                    self.mem.audio.ch4.update(false);
                }
            }
            0x24 => {
                let right_volume = value & 0x7;
                let left_volume = value & 0x70 >> 4;

                self.mem.audio.ch1.right_volume = right_volume;
                self.mem.audio.ch2.right_volume = right_volume;
                self.mem.audio.ch3.right_volume = right_volume;
                self.mem.audio.ch4.right_volume = right_volume;

                self.mem.audio.ch1.left_volume = left_volume;
                self.mem.audio.ch2.left_volume = left_volume;
                self.mem.audio.ch3.left_volume = left_volume;
                self.mem.audio.ch4.left_volume = left_volume;

                self.mem.audio.ch1.update(false);
                self.mem.audio.ch2.update(false);
                self.mem.audio.ch3.update(false);
                self.mem.audio.ch4.update(false);
            }
            0x25 => {
                self.mem.audio.ch1.right = value & 0x01 != 0;
                self.mem.audio.ch2.right = value & 0x02 != 0;
                self.mem.audio.ch3.right = value & 0x04 != 0;
                self.mem.audio.ch4.right = value & 0x08 != 0;
                self.mem.audio.ch1.left = value & 0x10 != 0;
                self.mem.audio.ch2.left = value & 0x20 != 0;
                self.mem.audio.ch3.left = value & 0x40 != 0;
                self.mem.audio.ch4.left = value & 0x80 != 0;
                self.mem.audio.ch1.update(false);
                self.mem.audio.ch2.update(false);
                self.mem.audio.ch3.update(false);
                self.mem.audio.ch4.update(false);
            }
            0x40 => {
                if (self.mem.display.lcdc & 0x80) != 0 && (value & 0x80 == 0) && self.mem.display.ly < 0x90 {
                    log(LogLevel::Error, format!("WARNING: LCD Operations stopped outside of VBlank. This could cause damage to a real hardware. This is a bug in the ROM and should be fixed.\n\t(Debug infos: PC = ${:04x}. LY = {}. Stat = {})", self.cpu.pc, self.mem.display.ly, self.mem.display.stat));

                }
                self.mem.display.lcdc = value
            },
            0x41 => {
                if value & 0b01000000 != 0 {
                    self.mem.display.lcd_interrupt_mode = 3;
                } else if value & 0b00100000 != 0 {
                    self.mem.display.lcd_interrupt_mode = 2;
                } else if value & 0b00010000 != 0 {
                    self.mem.display.lcd_interrupt_mode = 1;
                } else if value & 0b00001000 != 0 {
                    self.mem.display.lcd_interrupt_mode = 0;
                }
            }
            0x45 => self.mem.display.lyc = value,
            0x42 => self.mem.display.viewport_y = value,
            0x43 => self.mem.display.viewport_x = value,
            0x46 => {
                if value < 0xe0 {
                    let addr = (value as u16) << 8;

                    for i in 0..0xa0 {
                        self.w_mem(0xfe00 | i, self.r_mem(addr | i))
                    }
                }
            }
            0x47 => self.mem.display.bg_palette = value,
            0x48 => self.mem.display.obj_palettes[0] = value,
            0x49 => self.mem.display.obj_palettes[1] = value,
            0x4a => self.mem.display.window_y = value,
            0x4b => self.mem.display.window_x = value,
            0x4f => self.mem.display.vram_bank = value & 1,
            0x50 => self.mem.boot_rom_on = value & 1 == 0 && self.mem.boot_rom_on,
            0x68 => {
                self.mem.bgcram_pointer = 0b111111 & value;
                self.mem.bgcram_pointer_autoincrement = value & 0b10000000 != 0;
            }
            0x69 => {
                self.mem.display.cram[self.mem.bgcram_pointer as usize] = value;
                if self.mem.bgcram_pointer_autoincrement {
                    self.mem.bgcram_pointer += 1;
                    self.mem.bgcram_pointer &= 0b111111;
                }
            }
            0x6a => {
                self.mem.obcram_pointer = 0b111111 & value;
                self.mem.obcram_pointer_autoincrement = value & 0b10000000 != 0;
            }
            0x6b => {
                self.mem.display.cram[self.mem.obcram_pointer as usize + 0x40] = value;
                if self.mem.obcram_pointer_autoincrement {
                    self.mem.obcram_pointer += 1;
                    self.mem.obcram_pointer &= 0b111111;
                }
            }
            _ => {
                if addr != 0x25 && addr != 0x24 && addr != 0x26 && addr < 0x30 && addr > 0x3f {
                    log(
                        LogLevel::Debug,
                        format!(
                            "Writing to 0xff{:02x} not implemented yet ({:02x})",
                            addr, value
                        ),
                    );
                }
            }
        }
        self.mem.io[addr as usize] = value;

        if addr >= 0x30 && addr <= 0x3f {
            let i = (addr - 0x30) as usize;
            self.mem.audio.ch3.wave_pattern[i * 2] = value >> 4;
            self.mem.audio.ch3.wave_pattern[i * 2 + 1] = value & 0xf;
        }
    }
}
