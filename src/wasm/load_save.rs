use base64::prelude::*;
use std::io::{Cursor, Write, Read};
use web_sys::window;

use crate::state::GBState;
use crate::io::{LoadSave, Audio, Serial};
use crate::logs::{log, LogLevel};

#[derive(Debug)]
pub struct StaticRom;

impl StaticRom {
    pub fn new() -> Self {
        Self
    }
}

impl LoadSave for StaticRom {
    type Error = std::io::Error;

    fn load_rom(&self, rom: &mut [u8]) -> Result<(), std::io::Error> {
        let bytes = include_bytes!(env!("GAME_ROM_ASSET"));
        rom[..bytes.len()].copy_from_slice(bytes);

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

    fn load_external_ram(&self, _external_ram: &mut [u8]) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn save_external_ram(&self, _external_ram: &[u8]) -> Result<(), std::io::Error> {
        unimplemented!();
    }

    fn dump_state<S: Serial, A: Audio>(&self, _: &GBState<S, A>) -> Result<(), std::io::Error> {
        unimplemented!();
    }

    fn save_state<S: Serial, A: Audio>(&self, state: &GBState<S, A>) -> Result<(), std::io::Error> {
        let mut cursor = Cursor::new(vec![]);
        for addr in 0x8000..0xa000 {
            cursor.write_all(&[state.mem.r(addr)])?;
        }

        cursor.write_all(state.mem.wram_00.as_ref())?;
        cursor.write_all(state.mem.wram_01.as_ref())?;

        for addr in 0xff00..0xff80 {
            cursor.write_all(&[state.mem.r(addr)])?;
        }

        cursor.write_all(state.mem.hram.as_ref())?;
        cursor.write_all(&[state.mem.interrupts_register])?;

        cursor.write_all(&state.cpu.r)?;
        cursor.write_all(&state.cpu.pc.to_le_bytes())?;
        cursor.write_all(&state.cpu.sp.to_le_bytes())?;
        cursor.write_all(&[state.mem.boot_rom_on.into(), state.mem.ime.into()])?;
        let state_b64 = BASE64_STANDARD.encode(cursor.into_inner().as_slice());

        let local_storage = window()
            .expect("Cannot get localStorage if window doesn't exists")
            .local_storage()
            .expect("Window.local_storage() failed")
            .expect("Window.local_storage() returned None");

        log(LogLevel::Infos, "Saved To LocalStorage");
        local_storage.set_item("gameboy_state", &state_b64);

        Ok(())
    }

    fn load_state<S: Serial, A: Audio>(
        &self,
        state: &mut GBState<S, A>,
    ) -> Result<(), std::io::Error> {
        let local_storage = window()
            .expect("Cannot get localStorage if window doesn't exists")
            .local_storage()
            .expect("Window.local_storage() failed")
            .expect("Window.local_storage() returned None");

        log(LogLevel::Infos, "Load State From LocalStorage");
        if let Ok(Some(state_b64)) = local_storage.get_item("gameboy_state") {
            if let Ok(state_vec) = BASE64_STANDARD.decode(state_b64) {
                let mut cursor = Cursor::new(state_vec);
                let mut vram = Box::new([0; 0x2000]);
                cursor.read_exact(vram.as_mut())?;
                for i in 0x0000..0x2000 {
                    state.mem.w(0x8000 + i, vram[i as usize]);
                }

                cursor.read_exact(state.mem.wram_00.as_mut())?;
                cursor.read_exact(state.mem.wram_01.as_mut())?;

                let mut io = [0; 0x80];
                cursor.read_exact(io.as_mut())?;
                for i in 0x00..0x80 {
                    state.mem.w(0xff00 + i, io[i as usize]);
                }

                cursor.read_exact(state.mem.hram.as_mut())?;

                let mut reg8 = [0; 1];
                cursor.read_exact(reg8.as_mut())?;
                state.mem.interrupts_register = reg8[0];

                cursor.read_exact(&mut state.cpu.r)?;

                let mut reg16 = [0; 2];
                cursor.read_exact(&mut reg16)?;
                state.cpu.pc = u16::from_le_bytes(reg16);
                cursor.read_exact(&mut reg16)?;
                state.cpu.sp = u16::from_le_bytes(reg16);

                cursor.read_exact(reg8.as_mut())?;
                state.mem.boot_rom_on = reg8[0] != 0;
                cursor.read_exact(reg8.as_mut())?;
                state.mem.ime = reg8[0] != 0;
                log(LogLevel::Infos, "State loaded !");
            } else {
                log(LogLevel::Error, "Decoding State from LocalStorage failed");
            }
        } else {
            log(LogLevel::Error, "Loading from LocalStorage failed");
        }
        Ok(())
    }
}
