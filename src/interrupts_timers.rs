use crate::display::DisplayInterrupt;
use crate::io::{Audio, Serial};
use crate::state::GBState;

const TIMA_TIMER_SPEEDS: [u64; 4] = [1024, 16, 64, 256];

impl<S: Serial, A: Audio> GBState<S, A> {
    pub fn check_interrupts(&mut self) {
        if self.mem.ime {
            let interrupts = self.mem.io[0x0f] & self.mem.interrupts_register & 0b11111;
            for i in 0..5 {
                if interrupts & (1 << i) != 0 {
                    self.push(self.cpu.pc);

                    self.mem.ime = false;
                    self.cpu.pc = 0x40 + (i << 3);
                    self.mem.halt = false;

                    self.mem.io[0x0f] &= !(1 << i);
                    break;
                }
            }
        }
    }

    pub fn tima_timer(&mut self, c: u64) {
        if self.mem.timer_enabled
            && self.tima_cycles >= TIMA_TIMER_SPEEDS[self.mem.timer_speed as usize]
        {
            if self.mem.tima == 0xff {
                self.mem.io[0x0f] |= 0b100;
                self.mem.tima = self.mem.tma;
            } else {
                self.mem.tima += 1;
            }
            self.tima_cycles %= TIMA_TIMER_SPEEDS[self.mem.timer_speed as usize];
        }
        self.tima_cycles += c;
    }

    pub fn update_display_interrupts(&mut self, c: u64) {
        let interrupt = self.mem.display.update_display(c);

        match interrupt {
            DisplayInterrupt::Vblank => {
                self.mem.io[0x0f] |= 1;
            }
            DisplayInterrupt::Stat => {
                self.mem.io[0xf] |= 2;
            }
            DisplayInterrupt::Both => {
                self.mem.io[0xf] |= 3;
            }
            _ => {}
        }
    }

    pub fn div_timer(&mut self, c: u64) {
        if self.div_cycles >= 256 {
            self.mem.div += 1;

            self.div_cycles = 0;
        }
        self.div_cycles += c;
    }
}
