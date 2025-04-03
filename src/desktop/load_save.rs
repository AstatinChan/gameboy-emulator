use std::fs::File;
use std::io::{Write, Read};
use crate::io::LoadSave;

pub struct FSLoadSave {
    rom_file: String,
    save_file: String,
}

impl FSLoadSave {
    pub fn new(rom_file: impl Into<String>, save_file: impl Into<String>) -> Self {
        Self {
            rom_file: rom_file.into(),
            save_file: save_file.into(),
        }
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
        println!("MBC: {:02x}", boot_rom[0x147]);
        println!("CGB: {:02x}", boot_rom[0x143]);

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

        println!("Save file loaded from \"{}\"!", self.save_file);

        Ok(())
    }

    fn save_external_ram(&self, external_ram: &[u8]) -> Result<(), std::io::Error> {
        let mut f = File::create(&self.save_file)?;

        f.write_all(&external_ram)?;

        println!("Save written to \"{}\"!", self.save_file);

        Ok(())
    }
}
