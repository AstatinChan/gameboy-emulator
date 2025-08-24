use crate::io::{Audio, Serial};
use crate::logs::{log, LogLevel};
use crate::state::{flag, reg, GBState};

// The opcodes functions are returning the number of cycles used.

impl<S: Serial, A: Audio> GBState<S, A> {
    fn r_16b_from_pc(&mut self) -> u16 {
        let p: u16 = self.mem.r(self.cpu.pc) as u16 | ((self.mem.r(self.cpu.pc + 1) as u16) << 8);
        self.cpu.pc += 2;

        p
    }

    fn r_8b_from_pc(&mut self) -> u8 {
        let p = self.mem.r(self.cpu.pc);
        self.cpu.pc += 1;

        p
    }

    fn ldrr(&mut self, n1: u8, n2: u8) -> () {
        // Load a register into another register
        // LD r, r
        self.w_reg(n1, self.r_reg(n2))
    }

    fn ldr8(&mut self, n1: u8) -> u64 {
        // Load an raw 8b value into a register
        let p = self.r_8b_from_pc();

        self.w_reg(n1, p);
        8
    }

    fn ldrr16(&mut self, rr: u8, x: u16) {
        // Load a raw 16b value into a register
        self.cpu.w16(rr, x);
    }

    fn ldnnsp(&mut self) -> u64 {
        // Load SP into an arbitrary position in memory
        let p = self.r_16b_from_pc();

        self.mem.w(p, (self.cpu.sp & 0xff) as u8);
        self.mem.w(p + 1, (self.cpu.sp >> 8) as u8);
        20
    }

    fn ldsphl(&mut self) -> u64 {
        self.cpu.sp = self.cpu.r16(reg::HL);
        8
    }

    fn ldnna(&mut self, nn: u16) -> () {
        // Load A into an arbitrary position in memory
        self.mem.w(nn, self.cpu.r[reg::A as usize]);
        ()
    }

    fn ldann(&mut self, nn: u16) -> () {
        // Load A from an arbitrary position in memory
        self.cpu.r[reg::A as usize] = self.mem.r(nn);
        ()
    }

    pub fn push(&mut self, x: u16) -> () {
        self.cpu.sp -= 2;

        self.mem.w(self.cpu.sp, (x & 0xff) as u8);

        self.mem.w(self.cpu.sp + 1, (x >> 8) as u8);

        ()
    }

    fn pop(&mut self) -> u16 {
        let res = self.mem.r(self.cpu.sp) as u16 | ((self.mem.r(self.cpu.sp + 1) as u16) << 8);

        self.cpu.sp += 2;

        res
    }

    fn jr8(&mut self) -> u64 {
        // Unconditional relative jump
        let p = self.r_8b_from_pc();

        self.cpu.pc = (self.cpu.pc as i16 + p as i8 as i16) as u16;

        12
    }

    fn jrcc8(&mut self, n1: u8) -> u64 {
        // Conditional relative jump
        let p = self.r_8b_from_pc();
        let mut cycles = 8;

        if self.cpu.check_flag(n1 & 0b11) {
            cycles += 4;
            self.cpu.pc = (self.cpu.pc as i16 + p as i8 as i16) as u16;
        }

        cycles
    }

    fn jp16(&mut self) -> u64 {
        // Unconditional absolute jump
        let p = self.r_16b_from_pc();

        self.cpu.pc = p;

        16
    }

    fn jphl(&mut self) -> u64 {
        // Unconditional absolute jump to HL
        self.cpu.pc = self.cpu.r16(reg::HL);

        4
    }

    fn jpcc16(&mut self, n1: u8) -> u64 {
        // Conditional absolute jump
        let p = self.r_16b_from_pc();
        let mut cycles = 8;

        if self.cpu.check_flag(n1 & 0b11) {
            cycles += 4;
            self.cpu.pc = p;
        }

        cycles
    }

    fn call(&mut self) -> u64 {
        // Unconditional function call
        let p = self.r_16b_from_pc();

        self.push(self.cpu.pc);
        self.cpu.pc = p;

        24
    }

    fn callcc(&mut self, n1: u8) -> u64 {
        // Conditional function call
        let p = self.r_16b_from_pc();
        let mut cycles = 12;

        if self.cpu.check_flag(n1 & 0b11) {
            cycles += 12;
            self.push(self.cpu.pc);
            self.cpu.pc = p;
        }

        cycles
    }

    fn ret(&mut self) -> u64 {
        let res = self.pop();

        if res == 0 {
            log(LogLevel::Debug, format!("CPU: {:?}", self.cpu));
            panic!("RET to start");
        }

        self.cpu.pc = res;

        16
    }

    fn retcc(&mut self, n1: u8) -> u64 {
        let mut cycles = 8;
        if self.cpu.check_flag(n1 & 0b11) {
            cycles += 12;
            self.cpu.pc = self.pop();
        }

        cycles
    }

    fn ld00a(&mut self, n1: u8) -> u64 {
        // Load register A into or from memory pointed by rr (BC, DE or HL(+/-))
        // LD (rr), A
        // LD A, (rr)
        let ptr_reg = match n1 & 0b110 {
            0b000 => reg::B,
            0b010 => reg::C,
            _ => reg::HL,
        };

        if n1 & 0b001 == 1 {
            self.cpu.r[reg::A as usize] = self.mem.r(self.cpu.r16(ptr_reg));
        } else {
            self.mem
                .w(self.cpu.r16(ptr_reg), self.cpu.r[reg::A as usize]);
        }

        if n1 & 0b110 == 0b100 {
            self.cpu.w16(reg::HL, self.cpu.r16(reg::HL) + 1); // (HL+)
        }

        if n1 & 0b110 == 0b110 {
            self.cpu.w16(reg::HL, self.cpu.r16(reg::HL) - 1); // (HL-)
        }

        8
    }

    fn inc8(&mut self, n1: u8) -> u64 {
        // Increment 8 bit register
        self.w_reg(n1, self.r_reg(n1) + 1);
        self.cpu.r[reg::F as usize] &= !(flag::N | flag::ZF | flag::H);
        if self.r_reg(n1) == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }

        if self.r_reg(n1) & 0xf == 0x0 {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        4
    }

    fn dec8(&mut self, n1: u8) -> u64 {
        // Decrement 8 bit register
        self.w_reg(n1, self.r_reg(n1) - 1);
        self.cpu.r[reg::F as usize] |= flag::N;

        self.cpu.r[reg::F as usize] &= !(flag::ZF | flag::H);
        if self.r_reg(n1) == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }

        if self.r_reg(n1) & 0xf == 0xf {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        4
    }

    fn inc16(&mut self, rr: u8) -> u64 {
        // Increment 16 bit register
        self.cpu.w16(rr, self.cpu.r16(rr) + 1);
        8
    }

    fn dec16(&mut self, rr: u8) -> u64 {
        // Decrement 16 bit register
        self.cpu.w16(rr, self.cpu.r16(rr) - 1);
        8
    }

    fn ccf(&mut self) {
        // Flip carry flag
        self.cpu.r[reg::F as usize] = (self.cpu.r[reg::F as usize] & 0b10011111) ^ 0b00010000
    }

    fn scf(&mut self) {
        // Set carry flag
        self.cpu.r[reg::F as usize] = (self.cpu.r[reg::F as usize] & 0b10011111) | 0b00010000
    }

    fn daa(&mut self) {
        // Decimal Adjust Accumulator
        // Adjust the A register after a addition or substraction to keep valid BCD representation
        let nibble_low = self.cpu.r[reg::A as usize] & 0b1111;
        let sub_flag = (self.cpu.r[reg::F as usize] & flag::N) != 0;
        let half_carry_flag = (self.cpu.r[reg::F as usize] & flag::H) != 0;

        if (half_carry_flag || nibble_low > 9) && !sub_flag {
            self.cpu.r[reg::A as usize] += 0x06;
        }
        if (half_carry_flag || nibble_low > 9) && sub_flag {
            self.cpu.r[reg::A as usize] -= 0x06;
        }

        let nibble_high = self.cpu.r[reg::A as usize] >> 4;

        self.cpu.r[reg::F as usize] &= !(flag::CY | flag::ZF);

        if nibble_high > 9 && !sub_flag {
            self.cpu.r[reg::A as usize] += 0x60;
            self.cpu.r[reg::F as usize] |= flag::CY;
        }
        if nibble_high > 9 && sub_flag {
            self.cpu.r[reg::A as usize] -= 0x60;
            self.cpu.r[reg::F as usize] |= flag::CY;
        }

        if self.cpu.r[reg::A as usize] == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }

        self.cpu.r[reg::F as usize] &= !flag::H;
    }

    fn cpl(&mut self) {
        // Flip all bits in register A
        self.cpu.r[reg::F as usize] = self.cpu.r[reg::F as usize] | flag::N | flag::H;
        self.cpu.r[reg::A as usize] ^= 0xff;
    }

    fn addsp8(&mut self) -> u64 {
        let n = self.r_8b_from_pc() as i8;

        self.cpu.sp = (self.cpu.sp as i32 + n as i32) as u16;

        self.cpu.r[reg::F as usize] &= !(flag::N | flag::H | flag::CY);

        if (self.cpu.sp & 0xff) as i32 + n as i32 & !0xff != 0 {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        if (self.cpu.sp as i32 + n as i32) & !0xffff != 0 {
            self.cpu.r[reg::F as usize] |= flag::CY;
        }
        16
    }

    fn add(&mut self, x: u8) {
        // ADD a number to A and store the result in A
        let res = x as u16 + self.cpu.r[reg::A as usize] as u16;

        self.cpu.r[reg::F as usize] = 0;

        if (x & 0xf) + (self.cpu.r[reg::A as usize] & 0xf) > 0xf {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        if res > 0xff {
            self.cpu.r[reg::F as usize] |= flag::CY;
        }

        self.cpu.r[reg::A as usize] = res as u8;

        if self.cpu.r[reg::A as usize] == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }
    }

    fn addhlrr(&mut self, rr: u8) -> u64 {
        let n = self.cpu.r16(rr);
        let hl = self.cpu.r16(reg::HL);

        self.cpu.w16(reg::HL, (hl as i32 + n as i32) as u16);

        self.cpu.r[reg::F as usize] &= !(flag::N | flag::H | flag::CY);

        if (hl & 0xff) as i32 + n as i32 & !0xff != 0 {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        if (hl as i32 + n as i32) & !0xffff != 0 {
            self.cpu.r[reg::F as usize] |= flag::CY;
        }

        8
    }

    fn adc(&mut self, x: u8) {
        // ADD a number and the carry flag to A and store the result in A
        let carry = (self.cpu.r[reg::F as usize] & flag::CY) >> 4;
        let res = x as u16 + self.cpu.r[reg::A as usize] as u16 + carry as u16;

        self.cpu.r[reg::F as usize] = 0;

        if (x & 0xf) + ((self.cpu.r[reg::A as usize] & 0xf) + carry) > 0xf {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        if res > 0xff {
            self.cpu.r[reg::F as usize] |= flag::CY;
        }

        self.cpu.r[reg::A as usize] = res as u8;

        if self.cpu.r[reg::A as usize] == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }
    }

    fn sub(&mut self, x: u8) {
        // SUB a number to A and store the result in A
        self.cpu.r[reg::F as usize] = flag::N;

        if (x & 0xf) > (self.cpu.r[reg::A as usize] & 0xf) {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        if x > self.cpu.r[reg::A as usize] {
            self.cpu.r[reg::F as usize] |= flag::CY;
        }

        self.cpu.r[reg::A as usize] = self.cpu.r[reg::A as usize] - x;

        if self.cpu.r[reg::A as usize] == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }
    }

    fn sbc(&mut self, x: u8) {
        // SUB a number and the carry flag to A and store the result in A
        let carry = (self.cpu.r[reg::F as usize] & flag::CY) >> 4;
        self.cpu.r[reg::F as usize] = flag::N;

        if (x & 0xf) > (self.cpu.r[reg::A as usize] & 0xf) - carry {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        if x as i32 > self.cpu.r[reg::A as usize] as i32 - carry as i32 {
            self.cpu.r[reg::F as usize] |= flag::CY;
        }

        self.cpu.r[reg::A as usize] = self.cpu.r[reg::A as usize] - x - carry;

        if self.cpu.r[reg::A as usize] == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }
    }

    fn and(&mut self, x: u8) {
        // AND a number to A and store the result in A
        self.cpu.r[reg::A as usize] &= x;

        self.cpu.r[reg::F as usize] = flag::H;

        if self.cpu.r[reg::A as usize] == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }
    }

    fn xor(&mut self, x: u8) {
        // XOR a number to A and store the result in A
        self.cpu.r[reg::A as usize] ^= x;

        self.cpu.r[reg::F as usize] = 0;

        if self.cpu.r[reg::A as usize] == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }
    }

    fn or(&mut self, x: u8) {
        // OR a number to A and store the result in A
        self.cpu.r[reg::A as usize] |= x;

        self.cpu.r[reg::F as usize] = 0;

        if self.cpu.r[reg::A as usize] == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }
    }

    fn cp(&mut self, x: u8) {
        // SUB a number to A and update the flags accordingly without updating A
        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);

        self.cpu.r[reg::F as usize] |= flag::N;

        if x & 0xf > self.cpu.r[reg::A as usize] & 0xf {
            self.cpu.r[reg::F as usize] |= flag::H;
        }

        if x > self.cpu.r[reg::A as usize] {
            self.cpu.r[reg::F as usize] |= flag::CY;
        }

        let res = self.cpu.r[reg::A as usize] - x;

        if res == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }
    }

    fn rlc(&mut self, r_i: u8) -> () {
        // ROTATE LEFT the input register
        let mut n = self.r_reg(r_i);
        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
        self.cpu.r[reg::F as usize] |= (n >> 7) << 4;
        n <<= 1;
        n |= (self.cpu.r[reg::F as usize] & flag::CY) >> 4;
        self.w_reg(r_i, n)
    }

    fn rrc(&mut self, r_i: u8) -> () {
        // ROTATE RIGHT the input register
        let mut n = self.r_reg(r_i);
        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
        self.cpu.r[reg::F as usize] |= (n & 1) << 4;
        n >>= 1;
        n |= ((self.cpu.r[reg::F as usize] & flag::CY) >> 4) << 7;
        self.w_reg(r_i, n)
    }

    fn rl(&mut self, r_i: u8) -> () {
        // ROTATE LEFT THROUGH CARRY the input register
        // (RLC IS ROTATE AND RL IS ROTATE THROUGH CARRY ! IT DOESN'T MAKE ANY SENSE !!)
        let mut n = self.r_reg(r_i);
        let carry = (self.cpu.r[reg::F as usize] & flag::CY) >> 4;

        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
        self.cpu.r[reg::F as usize] |= (n >> 7) << 4;
        n <<= 1;
        n |= carry;
        self.w_reg(r_i, n)
    }

    fn rr(&mut self, r_i: u8) -> () {
        // ROTATE RIGHT THROUGH CARRY the input register
        let mut n = self.r_reg(r_i);
        let carry = (self.cpu.r[reg::F as usize] & flag::CY) >> 4;

        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
        self.cpu.r[reg::F as usize] |= (n & 1) << 4;
        n >>= 1;
        n |= carry << 7;
        self.w_reg(r_i, n)
    }

    fn sla(&mut self, r_i: u8) -> () {
        // Shift left Arithmetic (b0=0) the input register
        let mut n = self.r_reg(r_i);

        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
        self.cpu.r[reg::F as usize] |= (n >> 7) << 4;
        n <<= 1;

        if n == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }

        self.w_reg(r_i, n)
    }

    fn sra(&mut self, r_i: u8) -> () {
        // Shift right Arithmetic (b7=b7) the input register
        let mut n = self.r_reg(r_i);

        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
        self.cpu.r[reg::F as usize] |= (n & 0b1) << 4;
        let b7 = n & 0b10000000;
        n >>= 1;
        n |= b7;

        if n == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }

        self.w_reg(r_i, n)
    }

    fn swap(&mut self, r_i: u8) -> () {
        // Swap the high nibble and low nibble
        let mut n = self.r_reg(r_i);

        let nibble_low = n & 0b1111;
        let nibble_high = n >> 4;

        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);

        n = nibble_high | (nibble_low << 4);

        if n == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }

        self.w_reg(r_i, n)
    }

    fn srl(&mut self, r_i: u8) -> () {
        // Shift right Logical (b7=0) the input register
        let mut n = self.r_reg(r_i);

        self.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
        self.cpu.r[reg::F as usize] |= (n & 0b1) << 4;
        n >>= 1;

        if n == 0 {
            self.cpu.r[reg::F as usize] |= flag::ZF;
        }

        self.w_reg(r_i, n)
    }

    fn bit(&mut self, n1: u8, n2: u8) -> () {
        let z = (((self.r_reg(n2) >> n1) & 1) ^ 1) << 7;

        self.cpu.r[reg::F as usize] &= !(flag::N | flag::ZF);
        self.cpu.r[reg::F as usize] |= flag::H | z;
        ()
    }

    fn set(&mut self, n1: u8, n2: u8) -> () {
        self.w_reg(n2, self.r_reg(n2) | (1 << n1))
    }

    fn res(&mut self, n1: u8, n2: u8) -> () {
        self.w_reg(n2, self.r_reg(n2) & !(1 << n1))
    }

    // I don't remember why I separated op00, op01, op10 and op11 AND I'M NOT GOING TO CHANGE IT
    // BECAUSE I LOVE CHAOS

    fn op00(&mut self, n1: u8, n2: u8) -> u64 {
        // Dispatcher for the instructions starting with 0b00 based on their 3 LSB
        match n2 {
            0b000 => match n1 {
                0b000 => 4,
                0b001 => self.ldnnsp(),
                0b010 => {
                    self.is_stopped = true;
                    4
                }
                0b011 => self.jr8(),
                _ => self.jrcc8(n1),
            },
            0b001 => match n1 {
                0b001 | 0b011 | 0b101 | 0b111 => self.addhlrr(n1 >> 1),
                0b000 | 0b010 | 0b100 | 0b110 => {
                    let p = self.r_16b_from_pc();
                    self.ldrr16(n1 >> 1, p);
                    12
                }
                _ => panic!(),
            },
            0b010 => self.ld00a(n1),
            0b011 => match n1 {
                0b001 | 0b011 | 0b101 | 0b111 => self.dec16(n1 >> 1),
                0b000 | 0b010 | 0b100 | 0b110 => self.inc16(n1 >> 1),
                _ => panic!(),
            },
            0b100 => self.inc8(n1),
            0b101 => self.dec8(n1),
            0b110 => self.ldr8(n1),
            0b111 => {
                match n1 {
                    0b000 => self.rlc(7),
                    0b001 => self.rrc(7),
                    0b010 => self.rl(7),
                    0b011 => self.rr(7),
                    0b100 => self.daa(),
                    0b101 => self.cpl(),
                    0b110 => self.scf(),
                    0b111 => self.ccf(),
                    _ => panic!(),
                };
                4
            }
            _ => panic!(),
        }
    }

    fn op01(&mut self, n1: u8, n2: u8) -> u64 {
        // Dispatcher for the instructions starting with 0b01 (LD r,r and HALT)
        if n1 == 0b110 && n2 == 0b110 {
            self.mem.halt = true;
            4
        } else {
            self.ldrr(n1, n2);

            if n1 == 0b110 || n2 == 0b110 {
                8
            } else {
                4
            }
        }
    }

    fn op10(&mut self, n1: u8, n2: u8) -> u64 {
        // Dispatcher for the instructions starting with 0b10 (Arithmetic)
        match n1 {
            0b000 => self.add(self.r_reg(n2)),
            0b001 => self.adc(self.r_reg(n2)),
            0b010 => self.sub(self.r_reg(n2)),
            0b011 => self.sbc(self.r_reg(n2)),
            0b100 => self.and(self.r_reg(n2)),
            0b101 => self.xor(self.r_reg(n2)),
            0b110 => self.or(self.r_reg(n2)),
            0b111 => self.cp(self.r_reg(n2)),
            _ => panic!(),
        }

        if n2 == 0b110 {
            8
        } else {
            4
        }
    }

    fn op11(&mut self, n1: u8, n2: u8) -> u64 {
        match n2 {
            0b000 => match n1 {
                0b100 => {
                    let n = self.r_8b_from_pc();
                    self.ldnna(n as u16 | 0xff00);
                    12
                }
                0b101 => self.addsp8(),
                0b110 => {
                    let n = self.r_8b_from_pc();
                    self.ldann(n as u16 | 0xff00);
                    12
                }
                0b111 => {
                    let n = self.r_8b_from_pc();
                    self.ldrr16(reg::HL, n as u16 + self.cpu.sp);
                    12
                }
                _ => self.retcc(n1 & 0b11),
            },
            0b001 => match n1 {
                0b001 => self.ret(),
                0b011 => {
                    self.mem.ime = true;

                    self.ret()
                }
                0b101 => self.jphl(),
                0b111 => self.ldsphl(),
                _ => {
                    let p = self.pop();
                    self.cpu.r[(n1 >> 1) as usize * 2 + 1] = (p & 0xff) as u8;
                    self.cpu.r[(n1 >> 1) as usize * 2] = (p >> 8) as u8;
                    12
                }
            },
            0b010 => match n1 {
                0b100 => {
                    self.ldnna(self.cpu.r[reg::C as usize] as u16 | 0xff00);
                    8
                }
                0b101 => {
                    let nn = self.r_16b_from_pc();
                    self.ldnna(nn);
                    16
                }
                0b110 => {
                    self.ldann(self.cpu.r[reg::C as usize] as u16 | 0xff00);
                    8
                }
                0b111 => {
                    let nn = self.r_16b_from_pc();
                    self.ldann(nn);
                    16
                }
                _ => self.jpcc16(n1 & 0b11),
            },
            0b011 => match n1 {
                0b000 => self.jp16(),
                0b001 => self.op_bitwise(), // Bitwise operations
                0b011 | 0b100 | 0b101 => unimplemented!(),
                0b010 => {
                    self.cpu.print_debug();
                    0
                }
                0b110 => {
                    self.mem.ime = false;
                    4
                }
                0b111 => {
                    self.mem.ime = true;
                    4
                }
                _ => panic!(),
            },
            0b100 => self.callcc(n1 & 0b11),
            0b101 => match n1 {
                0b001 => self.call(),
                0b011 | 0b101 | 0b111 => unimplemented!(),
                _ => {
                    let value = self.cpu.r[(n1 >> 1) as usize * 2 + 1] as u16
                        | ((self.cpu.r[(n1 >> 1) as usize * 2] as u16) << 8);
                    self.push(value);
                    16
                }
            },
            0b110 => {
                let p = self.r_8b_from_pc();

                match n1 {
                    0b000 => self.add(p),
                    0b001 => self.adc(p),
                    0b010 => self.sub(p),
                    0b011 => self.sbc(p),
                    0b100 => self.and(p),
                    0b101 => self.xor(p),
                    0b110 => self.or(p),
                    0b111 => self.cp(p),
                    _ => panic!(),
                }
                8
            }
            0b111 => {
                let p = n1 << 3;

                self.push(self.cpu.pc);
                self.cpu.pc = p as u16;
                16
            } // RST
            _ => panic!(),
        }
    }

    fn op_bitwise(&mut self) -> u64 {
        let p = self.r_8b_from_pc();
        let opcode = p >> 6;
        let n1 = p >> 3 & 0b111;
        let n2 = p & 0b111;

        match opcode {
            0b00 => match n1 {
                0b000 => self.rlc(n2),
                0b001 => self.rrc(n2),
                0b010 => self.rl(n2),
                0b011 => self.rr(n2),
                0b100 => self.sla(n2),
                0b101 => self.sra(n2),
                0b110 => self.swap(n2),
                0b111 => self.srl(n2),
                _ => panic!(),
            },
            0b01 => self.bit(n1, n2),
            0b10 => self.res(n1, n2),
            0b11 => self.set(n1, n2),
            _ => panic!(),
        };
        if n2 == 0b110 {
            16
        } else {
            8
        }
    }

    pub fn exec_opcode(&mut self) -> u64 {
        let opcode = self.mem.r(self.cpu.pc);

        log(
            LogLevel::OpcodeDump,
            format!(
                "{:02x}:{:04x} = {:02x} (IME: {})",
                self.mem.rom_bank, self.cpu.pc, opcode, self.mem.ime
            ),
        );

        self.cpu.pc += 1;

        let n1 = (opcode >> 3) & 0b111;
        let n2 = opcode & 0b111;

        match opcode >> 6 {
            0b00 => self.op00(n1, n2),
            0b01 => self.op01(n1, n2),
            0b10 => self.op10(n1, n2),
            0b11 => self.op11(n1, n2),
            _ => panic!(),
        }
    }
}
