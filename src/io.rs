use crate::state::{MemError, Memory};

impl Memory {
    pub fn r_io(&self, addr: u8) -> u8 {
        if addr > 0x50 {
            println!("Reading from 0xff{:02x} not implemented yet", addr);
        }
        match addr {
            0x00 => {
                if self.joypad_is_action {
                    (self.joypad_reg >> 4) | 0b11010000
                } else {
                    (self.joypad_reg & 0xf) | 0b11100000
                }
            }
            0x04 => self.div,
            0x40 => self.display.lcdc,
            0x42 => self.display.viewport_y,
            0x43 => self.display.viewport_x,
            0x41 => {
                let mut ret = match self.display.lcd_interrupt_mode {
                    3 => 0b01000000,
                    2 => 0b00100000,
                    1 => 0b00010000,
                    0 => 0b00001000,
                    _ => 0,
                };

                ret |= if self.display.ly > 0x90 {
                    1
                } else if self.display.stat < 80 {
                    2
                } else if self.display.stat < 280 {
                    3
                } else {
                    0
                };

                if self.display.ly == self.display.lyc + 1 {
                    ret |= 0b100;
                }

                ret
            }
            0x44 => self.display.ly,
            0x45 => self.display.lyc,
            0x47 => self.display.bg_palette,
            0x48 => self.display.obj_palettes[0],
            0x49 => self.display.obj_palettes[1],
            0x4a => self.display.window_y,
            0x4b => self.display.window_x,
            0x50 => {
                if self.boot_rom_on {
                    0xfe
                } else {
                    0xff
                }
            }
            _ => {
                // println!("Reading from 0xff{:02x} not implemented yet", addr);
                self.io[addr as usize]
            }
        }
    }

    pub fn w_io(&mut self, addr: u8, value: u8) -> Result<(), MemError> {
        match addr {
            0x00 => {
                self.joypad_is_action = !value & 0b00100000 != 0;
            }
            0x04 => {
                self.div = 0;
            }
            0x05 => {
                self.tima = value;
            }
            0x06 => {
                self.tma = value;
            }
            0x07 => {
                self.timer_enabled = value & 0b100 != 0;
                self.timer_speed = value & 0b11;
            }
            0x0f => {
                self.io[0x0f] = value;
            }
            0x10 => {
                self.audio.ch1.period_sweep_pace = (0b1110000 & value) >> 4;
                self.audio.ch1.period_sweep_direction = (0b1000 & value) >> 3;
                self.audio.ch1.period_sweep_slope = 0b111 & value;
            }
            0x11 => {
                self.audio.ch1.duty = value >> 6;
                self.audio.ch1.length_timer = value & 0b111111;
            }
            0x12 => {
                self.audio.ch1.initial_volume = value >> 4;
                self.audio.ch1.env_direction = (value & 0xf) >> 3;
                self.audio.ch1.sweep = value & 0b111;
            }
            0x13 => {
                self.audio.ch1.period_value &= 0xff00;
                self.audio.ch1.period_value |= value as u16;
            }
            0x14 => {
                self.audio.ch1.period_value &= 0xff;
                self.audio.ch1.period_value |= ((value & 0b111) as u16) << 8;
                self.audio.ch1.length_timer_enabled = value & 0b01000000 != 0;
                if value >> 7 == 1 {
                    self.audio.ch1.update();
                }
            }
            0x16 => {
                self.audio.ch2.duty = value >> 6;
                self.audio.ch2.length_timer = value & 0b111111;
            }
            0x17 => {
                self.audio.ch2.initial_volume = value >> 4;
                self.audio.ch2.env_direction = (value & 0xf) >> 3;
                self.audio.ch2.sweep = value & 0b111;
            }
            0x18 => {
                self.audio.ch2.period_value &= 0xff00;
                self.audio.ch2.period_value |= value as u16;
            }
            0x19 => {
                self.audio.ch2.period_value &= 0xff;
                self.audio.ch2.period_value |= ((value & 0b111) as u16) << 8;
                self.audio.ch2.length_timer_enabled = value & 0b01000000 != 0;
                if value >> 7 == 1 {
                    self.audio.ch2.update();
                }
            }
            0x1a => {
                if value & 0b10000000 != 0 {
                    self.audio.ch3.on = true;
                } else {
                    self.audio.ch3.on = false;
                }
                self.audio.ch3.update();
            }
            0x1b => {
                self.audio.ch3.length_timer = value & 0b111111;
            }
            0x1c => {
                let s = (value >> 5) & 0b11;
                if s == 0 {
                    self.audio.ch3.initial_volume = 0;
                } else {
                    self.audio.ch3.initial_volume = 0xf >> (s - 1);
                }
            }
            0x1d => {
                self.audio.ch3.period_value &= 0xff00;
                self.audio.ch3.period_value |= value as u16;
            }
            0x1e => {
                self.audio.ch3.period_value &= 0xff;
                self.audio.ch3.period_value |= ((value & 0b111) as u16) << 8;
                self.audio.ch3.period_value /= 2;
                self.audio.ch3.length_timer_enabled = value & 0b01000000 != 0;
                if value >> 7 == 1 {
                    self.audio.ch3.update();
                }
            }
            0x20 => {
                self.audio.ch4.length_timer = value & 0b111111;
            }
            0x21 => {
                self.audio.ch4.initial_volume = value >> 4;
                self.audio.ch4.env_direction = (value & 0xf) >> 3;
                self.audio.ch4.sweep = value & 0b111;
            }
            0x22 => {
                self.audio.ch4.clock_shift = value >> 4;
                self.audio.ch4.lsfr_width = (value & 0xf) >> 3;
                self.audio.ch4.clock_divider = value & 0b111;
            }
            0x23 => {
                self.audio.ch4.length_timer_enabled = value & 0b01000000 != 0;
                if value >> 7 == 1 {
                    self.audio.ch4.update();
                }
            }
            0x40 => self.display.lcdc = value,
            0x41 => {
                if value & 0b01000000 != 0 {
                    self.display.lcd_interrupt_mode = 3;
                } else if value & 0b00100000 != 0 {
                    self.display.lcd_interrupt_mode = 2;
                } else if value & 0b00010000 != 0 {
                    self.display.lcd_interrupt_mode = 1;
                } else if value & 0b00001000 != 0 {
                    self.display.lcd_interrupt_mode = 0;
                }
            }
            0x45 => self.display.lyc = value,
            0x42 => self.display.viewport_y = value,
            0x43 => self.display.viewport_x = value,
            0x46 => {
                if value < 0xe0 {
                    let addr = (value as u16) << 8;

                    for i in 0..0xa0 {
                        self.w(0xfe00 | i, self.r(addr | i)?)?;
                    }
                }
            }
            0x47 => self.display.bg_palette = value,
            0x48 => self.display.obj_palettes[0] = value,
            0x49 => self.display.obj_palettes[1] = value,
            0x4a => self.display.window_y = value,
            0x4b => self.display.window_x = value,
            0x4f => self.display.vram_bank = value & 1,
            0x50 => self.boot_rom_on = value & 1 == 0 && self.boot_rom_on,
            0x68 => {
                self.bgcram_pointer = 0b111111 & value;
                self.bgcram_pointer_autoincrement = value & 0b10000000 != 0;
            }
            0x69 => {
                self.display.cram[self.bgcram_pointer as usize] = value;
                if self.bgcram_pointer_autoincrement {
                    self.bgcram_pointer += 1;
                    self.bgcram_pointer &= 0b111111;
                }
            }
            0x6a => {
                self.obcram_pointer = 0b111111 & value;
                self.obcram_pointer_autoincrement = value & 0b10000000 != 0;
            }
            0x6b => {
                self.display.cram[self.obcram_pointer as usize + 0x40] = value;
                if self.obcram_pointer_autoincrement {
                    self.obcram_pointer += 1;
                    self.obcram_pointer &= 0b111111;
                }
            }
            _ => {
                if addr >= 0x4d {
                    println!(
                        "Writing to 0xff{:02x} not implemented yet ({:02x})",
                        addr, value
                    );
                }
            }
        }
        self.io[addr as usize] = value;

        if addr >= 0x30 && addr <= 0x3f {
            let i = (addr - 0x30) as usize;
            self.audio.ch3.wave_pattern[i * 2] = value >> 4;
            self.audio.ch3.wave_pattern[i * 2 + 1] = value & 0xf;
        }

        Ok(())
    }
}
