use crate::state::{flag, reg, GBState, MemError};

// The opcodes functions are returning the number of cycles used.

pub fn r_16b_from_pc(state: &mut GBState) -> Result<u16, MemError> {
    let p: u16 = state.mem.r(state.cpu.pc)? as u16 | ((state.mem.r(state.cpu.pc + 1)? as u16) << 8);
    state.cpu.pc += 2;

    Ok(p)
}

pub fn r_8b_from_pc(state: &mut GBState) -> Result<u8, MemError> {
    let p = state.mem.r(state.cpu.pc)?;
    state.cpu.pc += 1;

    Ok(p)
}

pub fn ldrr(state: &mut GBState, n1: u8, n2: u8) -> Result<(), MemError> {
    // Load a register into another register
    // LD r, r
    state.w_reg(n1, state.r_reg(n2)?)
}

pub fn ldr8(state: &mut GBState, n1: u8) -> Result<u64, MemError> {
    // Load an raw 8b value into a register
    let p = r_8b_from_pc(state)?;

    state.w_reg(n1, p)?;
    Ok(8)
}

pub fn ldrr16(state: &mut GBState, rr: u8, x: u16) {
    // Load a raw 16b value into a register
    state.cpu.w16(rr, x);
}

pub fn ldnnsp(state: &mut GBState) -> Result<u64, MemError> {
    // Load SP into an arbitrary position in memory
    let p = r_16b_from_pc(state)?;

    state.mem.w(p, (state.cpu.sp & 0xff) as u8)?;
    state.mem.w(p + 1, (state.cpu.sp >> 8) as u8)?;
    Ok(20)
}

pub fn ldsphl(state: &mut GBState) -> u64 {
    state.cpu.sp = state.cpu.r16(reg::HL);
    8
}

pub fn ldnna(state: &mut GBState, nn: u16) -> Result<(), MemError> {
    // Load A into an arbitrary position in memory
    state.mem.w(nn, state.cpu.r[reg::A as usize])?;
    Ok(())
}

pub fn ldann(state: &mut GBState, nn: u16) -> Result<(), MemError> {
    // Load A from an arbitrary position in memory
    state.cpu.r[reg::A as usize] = state.mem.r(nn)?;
    Ok(())
}

pub fn push(state: &mut GBState, x: u16) -> Result<(), MemError> {
    state.cpu.sp -= 2;

    state.mem.w(state.cpu.sp, (x & 0xff) as u8)?;

    state.mem.w(state.cpu.sp + 1, (x >> 8) as u8)?;

    Ok(())
}

pub fn pop(state: &mut GBState) -> Result<u16, MemError> {
    let res = state.mem.r(state.cpu.sp)? as u16 | ((state.mem.r(state.cpu.sp + 1)? as u16) << 8);

    state.cpu.sp += 2;

    Ok(res)
}

pub fn jr8(state: &mut GBState) -> Result<u64, MemError> {
    // Unconditional relative jump
    let p = r_8b_from_pc(state)?;

    state.cpu.pc = (state.cpu.pc as i16 + p as i8 as i16) as u16;

    Ok(12)
}

pub fn jrcc8(state: &mut GBState, n1: u8) -> Result<u64, MemError> {
    // Conditional relative jump
    let p = r_8b_from_pc(state)?;
    let mut cycles = 8;

    if state.cpu.check_flag(n1 & 0b11) {
        cycles += 4;
        state.cpu.pc = (state.cpu.pc as i16 + p as i8 as i16) as u16;
    }

    Ok(cycles)
}

pub fn jp16(state: &mut GBState) -> Result<u64, MemError> {
    // Unconditional absolute jump
    let p = r_16b_from_pc(state)?;

    state.cpu.pc = p;

    Ok(16)
}

pub fn jphl(state: &mut GBState) -> u64 {
    // Unconditional absolute jump to HL
    state.cpu.pc = state.cpu.r16(reg::HL);

    4
}

pub fn jpcc16(state: &mut GBState, n1: u8) -> Result<u64, MemError> {
    // Conditional absolute jump
    let p = r_16b_from_pc(state)?;
    let mut cycles = 8;

    if state.cpu.check_flag(n1 & 0b11) {
        cycles += 4;
        state.cpu.pc = p;
    }

    Ok(cycles)
}

pub fn call(state: &mut GBState) -> Result<u64, MemError> {
    // Unconditional function call
    let p = r_16b_from_pc(state)?;

    push(state, state.cpu.pc)?;
    state.cpu.pc = p;

    Ok(24)
}

pub fn callcc(state: &mut GBState, n1: u8) -> Result<u64, MemError> {
    // Conditional function call
    let p = r_16b_from_pc(state)?;
    let mut cycles = 12;

    if state.cpu.check_flag(n1 & 0b11) {
        cycles += 12;
        push(state, state.cpu.pc)?;
        state.cpu.pc = p;
    }

    Ok(cycles)
}

pub fn ret(state: &mut GBState) -> Result<u64, MemError> {
    let res = pop(state)?;

    if res == 0 {
        println!("DEBUG: {:?}", state.cpu);
        panic!("RET to start");
    }

    state.cpu.pc = res;

    Ok(16)
}

pub fn retcc(state: &mut GBState, n1: u8) -> Result<u64, MemError> {
    let mut cycles = 8;
    if state.cpu.check_flag(n1 & 0b11) {
        cycles += 12;
        state.cpu.pc = pop(state)?;
    }

    Ok(cycles)
}

pub fn ld00a(state: &mut GBState, n1: u8) -> Result<u64, MemError> {
    // Load register A into or from memory pointed by rr (BC, DE or HL(+/-))
    // LD (rr), A
    // LD A, (rr)
    let ptr_reg = match n1 & 0b110 {
        0b000 => reg::B,
        0b010 => reg::C,
        _ => reg::HL,
    };

    if n1 & 0b001 == 1 {
        state.cpu.r[reg::A as usize] = state.mem.r(state.cpu.r16(ptr_reg))?;
    } else {
        state
            .mem
            .w(state.cpu.r16(ptr_reg), state.cpu.r[reg::A as usize])?;
    }

    if n1 & 0b110 == 0b100 {
        state.cpu.w16(reg::HL, state.cpu.r16(reg::HL) + 1); // (HL+)
    }

    if n1 & 0b110 == 0b110 {
        state.cpu.w16(reg::HL, state.cpu.r16(reg::HL) - 1); // (HL-)
    }

    Ok(8)
}

pub fn inc8(state: &mut GBState, n1: u8) -> Result<u64, MemError> {
    // Increment 8 bit register
    state.w_reg(n1, state.r_reg(n1)? + 1)?;
    state.cpu.r[reg::F as usize] &= !(flag::N | flag::ZF | flag::H);
    if state.r_reg(n1)? == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }

    if state.r_reg(n1)? & 0xf == 0x0 {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    Ok(4)
}

pub fn dec8(state: &mut GBState, n1: u8) -> Result<u64, MemError> {
    // Decrement 8 bit register
    state.w_reg(n1, state.r_reg(n1)? - 1)?;
    state.cpu.r[reg::F as usize] |= flag::N;

    state.cpu.r[reg::F as usize] &= !(flag::ZF | flag::H);
    if state.r_reg(n1)? == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }

    if state.r_reg(n1)? & 0xf == 0xf {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    Ok(4)
}

pub fn inc16(state: &mut GBState, rr: u8) -> u64 {
    // Increment 16 bit register
    state.cpu.w16(rr, state.cpu.r16(rr) + 1);
    8
}

pub fn dec16(state: &mut GBState, rr: u8) -> u64 {
    // Decrement 16 bit register
    state.cpu.w16(rr, state.cpu.r16(rr) - 1);
    8
}

pub fn ccf(state: &mut GBState) {
    // Flip carry flag
    state.cpu.r[reg::F as usize] = (state.cpu.r[reg::F as usize] & 0b10011111) ^ 0b00010000
}

pub fn scf(state: &mut GBState) {
    // Set carry flag
    state.cpu.r[reg::F as usize] = (state.cpu.r[reg::F as usize] & 0b10011111) | 0b00010000
}

pub fn daa(state: &mut GBState) {
    // Decimal Adjust Accumulator
    // Adjust the A register after a addition or substraction to keep valid BCD representation
    let nibble_low = state.cpu.r[reg::A as usize] & 0b1111;
    let sub_flag = (state.cpu.r[reg::F as usize] & flag::N) != 0;
    let half_carry_flag = (state.cpu.r[reg::F as usize] & flag::H) != 0;

    if (half_carry_flag || nibble_low > 9) && !sub_flag {
        state.cpu.r[reg::A as usize] += 0x06;
    }
    if (half_carry_flag || nibble_low > 9) && sub_flag {
        state.cpu.r[reg::A as usize] -= 0x06;
    }

    let nibble_high = state.cpu.r[reg::A as usize] >> 4;

    state.cpu.r[reg::F as usize] &= !(flag::CY | flag::ZF);

    if nibble_high > 9 && !sub_flag {
        state.cpu.r[reg::A as usize] += 0x60;
        state.cpu.r[reg::F as usize] |= flag::CY;
    }
    if nibble_high > 9 && sub_flag {
        state.cpu.r[reg::A as usize] -= 0x60;
        state.cpu.r[reg::F as usize] |= flag::CY;
    }

    if state.cpu.r[reg::A as usize] == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }

    state.cpu.r[reg::F as usize] &= !flag::H;
}

pub fn cpl(state: &mut GBState) {
    // Flip all bits in register A
    state.cpu.r[reg::F as usize] = state.cpu.r[reg::F as usize] | flag::N | flag::H;
    state.cpu.r[reg::A as usize] ^= 0xff;
}

pub fn addsp8(state: &mut GBState) -> Result<u64, MemError> {
    let n = r_8b_from_pc(state)? as i8;

    state.cpu.sp = (state.cpu.sp as i32 + n as i32) as u16;

    state.cpu.r[reg::F as usize] &= !(flag::N | flag::H | flag::CY);

    if (state.cpu.sp & 0xff) as i32 + n as i32 & !0xff != 0 {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    if (state.cpu.sp as i32 + n as i32) & !0xffff != 0 {
        state.cpu.r[reg::F as usize] |= flag::CY;
    }
    Ok(16)
}

pub fn add(state: &mut GBState, x: u8) {
    // ADD a number to A and store the result in A
    let res = x as u16 + state.cpu.r[reg::A as usize] as u16;

    state.cpu.r[reg::F as usize] = 0;

    if (x & 0xf) + (state.cpu.r[reg::A as usize] & 0xf) > 0xf {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    if res > 0xff {
        state.cpu.r[reg::F as usize] |= flag::CY;
    }

    state.cpu.r[reg::A as usize] = res as u8;

    if state.cpu.r[reg::A as usize] == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }
}

pub fn addhlrr(state: &mut GBState, rr: u8) -> u64 {
    let n = state.cpu.r16(rr);
    let hl = state.cpu.r16(reg::HL);

    state.cpu.w16(reg::HL, (hl as i32 + n as i32) as u16);

    state.cpu.r[reg::F as usize] &= !(flag::N | flag::H | flag::CY);

    if (hl & 0xff) as i32 + n as i32 & !0xff != 0 {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    if (hl as i32 + n as i32) & !0xffff != 0 {
        state.cpu.r[reg::F as usize] |= flag::CY;
    }

    8
}

pub fn adc(state: &mut GBState, x: u8) {
    // ADD a number and the carry flag to A and store the result in A
    let carry = (state.cpu.r[reg::F as usize] & flag::CY) >> 4;
    let res = x as u16 + state.cpu.r[reg::A as usize] as u16 + carry as u16;

    state.cpu.r[reg::F as usize] = 0;

    if (x & 0xf) + ((state.cpu.r[reg::A as usize] & 0xf) + carry) > 0xf {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    if res > 0xff {
        state.cpu.r[reg::F as usize] |= flag::CY;
    }

    state.cpu.r[reg::A as usize] = res as u8;

    if state.cpu.r[reg::A as usize] == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }
}

pub fn sub(state: &mut GBState, x: u8) {
    // SUB a number to A and store the result in A
    state.cpu.r[reg::F as usize] = flag::N;

    if (x & 0xf) > (state.cpu.r[reg::A as usize] & 0xf) {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    if x > state.cpu.r[reg::A as usize] {
        state.cpu.r[reg::F as usize] |= flag::CY;
    }

    state.cpu.r[reg::A as usize] = state.cpu.r[reg::A as usize] - x;

    if state.cpu.r[reg::A as usize] == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }
}

pub fn sbc(state: &mut GBState, x: u8) {
    // SUB a number and the carry flag to A and store the result in A
    let carry = (state.cpu.r[reg::F as usize] & flag::CY) >> 4;
    state.cpu.r[reg::F as usize] = flag::N;

    if (x & 0xf) > (state.cpu.r[reg::A as usize] & 0xf) - carry {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    if x as i32 > state.cpu.r[reg::A as usize] as i32 - carry as i32 {
        state.cpu.r[reg::F as usize] |= flag::CY;
    }

    state.cpu.r[reg::A as usize] = state.cpu.r[reg::A as usize] - x - carry;

    if state.cpu.r[reg::A as usize] == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }
}

pub fn and(state: &mut GBState, x: u8) {
    // AND a number to A and store the result in A
    state.cpu.r[reg::A as usize] &= x;

    state.cpu.r[reg::F as usize] = flag::H;

    if state.cpu.r[reg::A as usize] == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }
}

pub fn xor(state: &mut GBState, x: u8) {
    // XOR a number to A and store the result in A
    state.cpu.r[reg::A as usize] ^= x;

    state.cpu.r[reg::F as usize] = 0;

    if state.cpu.r[reg::A as usize] == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }
}

pub fn or(state: &mut GBState, x: u8) {
    // OR a number to A and store the result in A
    state.cpu.r[reg::A as usize] |= x;

    state.cpu.r[reg::F as usize] = 0;

    if state.cpu.r[reg::A as usize] == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }
}

pub fn cp(state: &mut GBState, x: u8) {
    // SUB a number to A and update the flags accordingly without updating A
    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);

    state.cpu.r[reg::F as usize] |= flag::N;

    if x & 0xf > state.cpu.r[reg::A as usize] & 0xf {
        state.cpu.r[reg::F as usize] |= flag::H;
    }

    if x > state.cpu.r[reg::A as usize] {
        state.cpu.r[reg::F as usize] |= flag::CY;
    }

    let res = state.cpu.r[reg::A as usize] - x;

    if res == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }
}

pub fn rlc(state: &mut GBState, r_i: u8) -> Result<(), MemError> {
    // ROTATE LEFT the input register
    let mut n = state.r_reg(r_i)?;
    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
    state.cpu.r[reg::F as usize] |= (n >> 7) << 4;
    n <<= 1;
    n |= (state.cpu.r[reg::F as usize] & flag::CY) >> 4;
    state.w_reg(r_i, n)
}

pub fn rrc(state: &mut GBState, r_i: u8) -> Result<(), MemError> {
    // ROTATE RIGHT the input register
    let mut n = state.r_reg(r_i)?;
    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
    state.cpu.r[reg::F as usize] |= (n & 1) << 4;
    n >>= 1;
    n |= ((state.cpu.r[reg::F as usize] & flag::CY) >> 4) << 7;
    state.w_reg(r_i, n)
}

pub fn rl(state: &mut GBState, r_i: u8) -> Result<(), MemError> {
    // ROTATE LEFT THROUGH CARRY the input register
    // (RLC IS ROTATE AND RL IS ROTATE THROUGH CARRY ! IT DOESN'T MAKE ANY SENSE !!)
    let mut n = state.r_reg(r_i)?;
    let carry = (state.cpu.r[reg::F as usize] & flag::CY) >> 4;

    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
    state.cpu.r[reg::F as usize] |= (n >> 7) << 4;
    n <<= 1;
    n |= carry;
    state.w_reg(r_i, n)
}

pub fn rr(state: &mut GBState, r_i: u8) -> Result<(), MemError> {
    // ROTATE RIGHT THROUGH CARRY the input register
    let mut n = state.r_reg(r_i)?;
    let carry = (state.cpu.r[reg::F as usize] & flag::CY) >> 4;

    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
    state.cpu.r[reg::F as usize] |= (n & 1) << 4;
    n >>= 1;
    n |= carry << 7;
    state.w_reg(r_i, n)
}

pub fn sla(state: &mut GBState, r_i: u8) -> Result<(), MemError> {
    // Shift left Arithmetic (b0=0) the input register
    let mut n = state.r_reg(r_i)?;

    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
    state.cpu.r[reg::F as usize] |= (n >> 7) << 4;
    n <<= 1;

    if n == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }

    state.w_reg(r_i, n)
}

pub fn sra(state: &mut GBState, r_i: u8) -> Result<(), MemError> {
    // Shift right Arithmetic (b7=b7) the input register
    let mut n = state.r_reg(r_i)?;

    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
    state.cpu.r[reg::F as usize] |= (n & 0b1) << 4;
    let b7 = n & 0b10000000;
    n >>= 1;
    n |= b7;

    if n == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }

    state.w_reg(r_i, n)
}

pub fn swap(state: &mut GBState, r_i: u8) -> Result<(), MemError> {
    // Swap the high nibble and low nibble
    let mut n = state.r_reg(r_i)?;

    let nibble_low = n & 0b1111;
    let nibble_high = n >> 4;

    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);

    n = nibble_high | (nibble_low << 4);

    if n == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }

    state.w_reg(r_i, n)
}

pub fn srl(state: &mut GBState, r_i: u8) -> Result<(), MemError> {
    // Shift right Logical (b7=0) the input register
    let mut n = state.r_reg(r_i)?;

    state.cpu.r[reg::F as usize] &= !(flag::H | flag::N | flag::ZF | flag::CY);
    state.cpu.r[reg::F as usize] |= (n & 0b1) << 4;
    n >>= 1;

    if n == 0 {
        state.cpu.r[reg::F as usize] |= flag::ZF;
    }

    state.w_reg(r_i, n)
}

pub fn bit(state: &mut GBState, n1: u8, n2: u8) -> Result<(), MemError> {
    let z = (((state.r_reg(n2)? >> n1) & 1) ^ 1) << 7;

    state.cpu.r[reg::F as usize] &= !(flag::N | flag::ZF);
    state.cpu.r[reg::F as usize] |= flag::H | z;
    Ok(())
}

pub fn set(state: &mut GBState, n1: u8, n2: u8) -> Result<(), MemError> {
    state.w_reg(n2, state.r_reg(n2)? | (1 << n1))
}

pub fn res(state: &mut GBState, n1: u8, n2: u8) -> Result<(), MemError> {
    state.w_reg(n2, state.r_reg(n2)? & !(1 << n1))
}

// I don't remember why I separated op00, op01, op10 and op11 AND I'M NOT GOING TO CHANGE IT
// BECAUSE I LOVE CHAOS

pub fn op00(state: &mut GBState, n1: u8, n2: u8) -> Result<u64, MemError> {
    // Dispatcher for the instructions starting with 0b00 based on their 3 LSB
    match n2 {
        0b000 => match n1 {
            0b000 => Ok(4),
            0b001 => ldnnsp(state),
            0b010 => todo!("STOP"),
            0b011 => jr8(state),
            _ => jrcc8(state, n1),
        },
        0b001 => match n1 {
            0b001 | 0b011 | 0b101 | 0b111 => Ok(addhlrr(state, n1 >> 1)),
            0b000 | 0b010 | 0b100 | 0b110 => {
                let p = r_16b_from_pc(state)?;
                ldrr16(state, n1 >> 1, p);
                Ok(12)
            }
            _ => panic!(),
        },
        0b010 => ld00a(state, n1),
        0b011 => match n1 {
            0b001 | 0b011 | 0b101 | 0b111 => Ok(dec16(state, n1 >> 1)),
            0b000 | 0b010 | 0b100 | 0b110 => Ok(inc16(state, n1 >> 1)),
            _ => panic!(),
        },
        0b100 => inc8(state, n1),
        0b101 => dec8(state, n1),
        0b110 => ldr8(state, n1),
        0b111 => {
            match n1 {
                0b000 => rlc(state, 7)?,
                0b001 => rrc(state, 7)?,
                0b010 => rl(state, 7)?,
                0b011 => rr(state, 7)?,
                0b100 => daa(state),
                0b101 => cpl(state),
                0b110 => scf(state),
                0b111 => ccf(state),
                _ => panic!(),
            };
            Ok(4)
        }
        _ => panic!(),
    }
}

pub fn op01(state: &mut GBState, n1: u8, n2: u8) -> Result<u64, MemError> {
    // Dispatcher for the instructions starting with 0b01 (LD r,r and HALT)
    if n1 == 0b110 && n2 == 0b110 {
        state.mem.halt = true;
        Ok(4)
    } else {
        ldrr(state, n1, n2)?;

        if n1 == 0b110 || n2 == 0b110 {
            Ok(8)
        } else {
            Ok(4)
        }
    }
}

pub fn op10(state: &mut GBState, n1: u8, n2: u8) -> Result<u64, MemError> {
    // Dispatcher for the instructions starting with 0b10 (Arithmetic)
    match n1 {
        0b000 => add(state, state.r_reg(n2)?),
        0b001 => adc(state, state.r_reg(n2)?),
        0b010 => sub(state, state.r_reg(n2)?),
        0b011 => sbc(state, state.r_reg(n2)?),
        0b100 => and(state, state.r_reg(n2)?),
        0b101 => xor(state, state.r_reg(n2)?),
        0b110 => or(state, state.r_reg(n2)?),
        0b111 => cp(state, state.r_reg(n2)?),
        _ => panic!(),
    }

    if n2 == 0b110 {
        Ok(8)
    } else {
        Ok(4)
    }
}

pub fn op11(state: &mut GBState, n1: u8, n2: u8) -> Result<u64, MemError> {
    match n2 {
        0b000 => match n1 {
            0b100 => {
                let n = r_8b_from_pc(state)?;
                ldnna(state, n as u16 | 0xff00)?;
                Ok(12)
            }
            0b101 => addsp8(state),
            0b110 => {
                let n = r_8b_from_pc(state)?;
                ldann(state, n as u16 | 0xff00)?;
                Ok(12)
            }
            0b111 => {
                let n = r_8b_from_pc(state)?;
                ldrr16(state, reg::HL, n as u16 + state.cpu.sp);
                Ok(12)
            }
            _ => retcc(state, n1 & 0b11),
        },
        0b001 => match n1 {
            0b001 => ret(state),
            0b011 => {
                state.mem.ime = true;

                ret(state)
            }
            0b101 => Ok(jphl(state)),
            0b111 => Ok(ldsphl(state)),
            _ => {
                let p = pop(state)?;
                state.cpu.r[(n1 >> 1) as usize * 2 + 1] = (p & 0xff) as u8;
                state.cpu.r[(n1 >> 1) as usize * 2] = (p >> 8) as u8;
                Ok(12)
            }
        },
        0b010 => match n1 {
            0b100 => {
                ldnna(state, state.cpu.r[reg::C as usize] as u16 | 0xff00)?;
                Ok(8)
            }
            0b101 => {
                let nn = r_16b_from_pc(state)?;
                ldnna(state, nn)?;
                Ok(16)
            }
            0b110 => {
                ldann(state, state.cpu.r[reg::C as usize] as u16 | 0xff00)?;
                Ok(8)
            }
            0b111 => {
                let nn = r_16b_from_pc(state)?;
                ldann(state, nn)?;
                Ok(16)
            }
            _ => jpcc16(state, n1 & 0b11),
        },
        0b011 => match n1 {
            0b000 => jp16(state),
            0b001 => op_bitwise(state), // Bitwise operations
            0b010 | 0b011 | 0b100 | 0b101 => unimplemented!(),
            0b110 => {
                state.mem.ime = false;
                Ok(4)
            }
            0b111 => {
                state.mem.ime = true;
                Ok(4)
            }
            _ => panic!(),
        },
        0b100 => callcc(state, n1 & 0b11),
        0b101 => match n1 {
            0b001 => call(state),
            0b011 | 0b101 | 0b111 => unimplemented!(),
            _ => {
                let value = state.cpu.r[(n1 >> 1) as usize * 2 + 1] as u16
                    | ((state.cpu.r[(n1 >> 1) as usize * 2] as u16) << 8);
                push(state, value)?;
                Ok(16)
            }
        },
        0b110 => {
            let p = r_8b_from_pc(state)?;

            match n1 {
                0b000 => add(state, p),
                0b001 => adc(state, p),
                0b010 => sub(state, p),
                0b011 => sbc(state, p),
                0b100 => and(state, p),
                0b101 => xor(state, p),
                0b110 => or(state, p),
                0b111 => cp(state, p),
                _ => panic!(),
            }
            Ok(8)
        }
        0b111 => {
            let p = n1 << 3;

            push(state, state.cpu.pc)?;
            state.cpu.pc = p as u16;
            Ok(16)
        } // RST
        _ => panic!(),
    }
}

pub fn op_bitwise(state: &mut GBState) -> Result<u64, MemError> {
    let p = r_8b_from_pc(state)?;
    let opcode = p >> 6;
    let n1 = p >> 3 & 0b111;
    let n2 = p & 0b111;

    match opcode {
        0b00 => match n1 {
            0b000 => rlc(state, n2),
            0b001 => rrc(state, n2),
            0b010 => rl(state, n2),
            0b011 => rr(state, n2),
            0b100 => sla(state, n2),
            0b101 => sra(state, n2),
            0b110 => swap(state, n2),
            0b111 => srl(state, n2),
            _ => panic!(),
        },
        0b01 => bit(state, n1, n2),
        0b10 => res(state, n1, n2),
        0b11 => set(state, n1, n2),
        _ => panic!(),
    }?;
    if n2 == 0b110 {
        Ok(16)
    } else {
        Ok(8)
    }
}
