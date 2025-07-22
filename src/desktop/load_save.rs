use crate::io::{Audio, LoadSave, Serial};
use crate::logs::{elog, log, LogLevel};
use crate::state::GBState;
use std::fs::File;
use std::io::{Read, Write};

#[derive(Debug)]
pub struct FSLoadSave {
    rom_file: String,
    save_file: String,
    state_file: Option<String>,
}

impl FSLoadSave {
    pub fn new(rom_file: impl Into<String>, save_file: impl Into<String>) -> Self {
        Self {
            rom_file: rom_file.into(),
            save_file: save_file.into(),
            state_file: None,
        }
    }

    pub fn state_file(mut self, state_file: impl Into<String>) -> Self {
        self.state_file = Some(state_file.into());
        self
    }
}

impl LoadSave for FSLoadSave {
    type Error = std::io::Error;

    fn load_rom(&self, rom: &mut [u8]) -> Result<(), std::io::Error> {
        let mut f = File::open(&self.rom_file)?;

        f.read(rom)?;

        return Ok(());
    }

    fn load_bootrom(&self, boot_rom: &mut [u8]) -> Result<(), std::io::Error> {
        log(LogLevel::Debug, format!("MBC: {:02x}", boot_rom[0x147]));
        log(LogLevel::Debug, format!("CGB: {:02x}", boot_rom[0x143]));

        if boot_rom[0x143] == 0x80 || boot_rom[0x143] == 0xc0 {
            unimplemented!("CGB Boot rom is not implemented");
            // let bytes = include_bytes!("../assets/cgb_boot.bin");

            // self.boot_rom[..0x900].copy_from_slice(bytes);
            // self.cgb_mode = true;
            // self.display.cgb_mode = true;
        } else {
            let bytes = include_bytes!("../../assets/dmg_boot.bin");

            boot_rom[..0x100].copy_from_slice(bytes);
        }

        Ok(())
    }

    fn load_external_ram(&self, external_ram: &mut [u8]) -> Result<(), std::io::Error> {
        let mut f = File::open(&self.save_file)?;

        f.read(external_ram)?;

        log(
            LogLevel::Infos,
            format!("Save file loaded from \"{}\"!", self.save_file),
        );

        Ok(())
    }

    fn save_external_ram(&self, external_ram: &[u8]) -> Result<(), std::io::Error> {
        let mut f = File::create(&self.save_file)?;

        f.write_all(&external_ram)?;

        log(
            LogLevel::Infos,
            format!("Save written to \"{}\"!", self.save_file),
        );

        Ok(())
    }

    fn dump_state<S: Serial, A: Audio>(&self, state: &GBState<S, A>) -> Result<(), std::io::Error> {
        {
            let mut vram_dump_file = File::create(format!("{}.vram.dump", self.rom_file))?;

            for addr in 0x8000..0xa000 {
                vram_dump_file.write_all(format!("{:02X} ", state.mem.r(addr)).as_bytes())?;
            }
        }

        {
            let mut wram_dump_file = File::create(format!("{}.wram.dump", self.rom_file))?;

            for addr in 0xc000..0xe000 {
                wram_dump_file.write_all(format!("{:02X} ", state.mem.r(addr)).as_bytes())?;
            }
        }

        {
            let mut io_dump_file = File::create(format!("{}.io.dump", self.rom_file))?;

            for addr in 0xff00..0xff80 {
                io_dump_file.write_all(format!("{:02X} ", state.mem.r(addr)).as_bytes())?;
            }
        }

        {
            let mut hram_dump_file = File::create(format!("{}.hram.dump", self.rom_file))?;

            for addr in 0xff80..=0xffff {
                hram_dump_file.write_all(format!("{:02X} ", state.mem.r(addr)).as_bytes())?;
            }
        }

        {
            let mut cpu_dump_file = File::create(format!("{}.cpu.dump", self.rom_file))?;

            for i in 0..8 {
                cpu_dump_file.write_all(format!("{:02X} ", state.cpu.r[i]).as_bytes())?;
            }

            cpu_dump_file.write_all(format!("{:04X} ", state.cpu.pc).as_bytes())?;

            cpu_dump_file.write_all(format!("{:04X} ", state.cpu.sp).as_bytes())?;
        }

        Ok(())
    }

    fn save_state<S: Serial, A: Audio>(&self, state: &GBState<S, A>) -> Result<(), std::io::Error> {
        if let Some(state_file) = &self.state_file {
            let mut state_file = File::create(state_file)?;
            for addr in 0x8000..0xa000 {
                state_file.write_all(&[state.mem.r(addr)])?;
            }

            state_file.write_all(state.mem.wram_00.as_ref())?;
            state_file.write_all(state.mem.wram_01.as_ref())?;

            for addr in 0xff00..0xff80 {
                state_file.write_all(&[state.mem.r(addr)])?;
            }

            state_file.write_all(state.mem.hram.as_ref())?;
            state_file.write_all(&[state.mem.interrupts_register])?;

            state_file.write_all(&state.cpu.r)?;
            state_file.write_all(&state.cpu.pc.to_le_bytes())?;
            state_file.write_all(&state.cpu.sp.to_le_bytes())?;
            state_file.write_all(&[state.mem.boot_rom_on.into(), state.mem.ime.into()])?;
        } else {
            elog(
                LogLevel::Error,
                format!("Tried to save state without state_file specified"),
            );
        }
        Ok(())
    }

    fn load_state<S: Serial, A: Audio>(
        &self,
        state: &mut GBState<S, A>,
    ) -> Result<(), std::io::Error> {
        if let Some(state_file) = &self.state_file {
            let mut state_file = File::open(state_file)?;

            let mut vram = Box::new([0; 0x2000]);
            state_file.read_exact(vram.as_mut())?;
            for i in 0x0000..0x2000 {
                state.mem.w(0x8000 + i, vram[i as usize]);
            }

            state_file.read_exact(state.mem.wram_00.as_mut())?;
            state_file.read_exact(state.mem.wram_01.as_mut())?;

            let mut io = [0; 0x80];
            state_file.read_exact(io.as_mut())?;
            for i in 0x00..0x80 {
                state.mem.w(0xff00 + i, io[i as usize]);
            }

            state_file.read_exact(state.mem.hram.as_mut())?;

            let mut reg8 = [0; 1];
            state_file.read_exact(reg8.as_mut())?;
            state.mem.interrupts_register = reg8[0];

            state_file.read_exact(&mut state.cpu.r)?;

            let mut reg16 = [0; 2];
            state_file.read_exact(&mut reg16)?;
            state.cpu.pc = u16::from_le_bytes(reg16);
            state_file.read_exact(&mut reg16)?;
            state.cpu.sp = u16::from_le_bytes(reg16);

            state_file.read_exact(reg8.as_mut())?;
            state.mem.boot_rom_on = reg8[0] != 0;
            state_file.read_exact(reg8.as_mut())?;
            state.mem.ime = reg8[0] != 0;
        }
        Ok(())
    }
}
