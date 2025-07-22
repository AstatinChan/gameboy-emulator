use std::fs::File;
use std::io::{ErrorKind, Read, Write};

use crate::desktop::window::Keys;
use crate::io::Input;
use gilrs::{Button, GamepadId, Gilrs};
use winit::keyboard::KeyCode;

pub struct Gamepad {
    gilrs: Gilrs,
    gamepad_id: Option<GamepadId>,
    last_save_state: bool,
}

impl Gamepad {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().unwrap();

        let gamepad_id = if let Some((gamepad_id, _gamepad)) = gilrs.gamepads().next() {
            println!("Found Gamepad id: {:?}", gamepad_id);
            Some(gamepad_id)
        } else {
            println!("No gamepad found");
            None
        };

        Self {
            gilrs,
            gamepad_id,
            last_save_state: false,
        }
    }

    pub fn check_special_actions(&self, is_debug: &mut bool) {
        if let Some(gamepad_id) = self.gamepad_id {
            if let Some(gamepad) = self.gilrs.connected_gamepad(gamepad_id) {
                *is_debug = gamepad.is_pressed(Button::West);
            }
        }
    }
}

impl Input for Gamepad {
    fn update_events(&mut self, _cycles: u128) -> Option<u128> {
        if let Some(gamepad_id) = self.gamepad_id {
            if self.gilrs.connected_gamepad(gamepad_id).is_none() {
                println!("Gamepad (id = {:?}) disconnected", gamepad_id);
                self.gamepad_id = None;
            }
        } else {
            if let Some((gamepad_id, _gamepad)) = self.gilrs.gamepads().next() {
                println!("Found Gamepad id: {:?}", gamepad_id);
                self.gamepad_id = Some(gamepad_id);
            }
        }

        while let Some(_) = self.gilrs.next_event() {}
        None
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        let mut res = 0xf;

        if let Some(gamepad_id) = self.gamepad_id {
            if let Some(gamepad) = self.gilrs.connected_gamepad(gamepad_id) {
                if gamepad.is_pressed(Button::East) {
                    res &= 0b1110;
                }

                if gamepad.is_pressed(Button::South) {
                    res &= 0b1101;
                }

                if gamepad.is_pressed(Button::Select) {
                    res &= 0b1011;
                }

                if gamepad.is_pressed(Button::Start) {
                    res &= 0b0111;
                }
            }
        }

        res
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        let mut res = 0xf;

        if let Some(gamepad_id) = self.gamepad_id {
            if let Some(gamepad) = self.gilrs.connected_gamepad(gamepad_id) {
                if gamepad.is_pressed(Button::DPadRight) {
                    res &= 0b1110;
                }

                if gamepad.is_pressed(Button::DPadLeft) {
                    res &= 0b1101;
                }

                if gamepad.is_pressed(Button::DPadUp) {
                    res &= 0b1011;
                }

                if gamepad.is_pressed(Button::DPadDown) {
                    res &= 0b0111;
                }
            }
        }

        res
    }

    fn save_state(&mut self) -> bool {
        let mut ret = false;

        if let Some(gamepad_id) = self.gamepad_id {
            if let Some(gamepad) = self.gilrs.connected_gamepad(gamepad_id) {
                let pressed = gamepad.is_pressed(Button::North);
                ret = pressed && !self.last_save_state;
                self.last_save_state = pressed;
            }
        }
        ret
    }
}

pub struct Keyboard {
    keys: Keys,
    action_reg: u8,
    direction_reg: u8,
}

impl Keyboard {
    pub fn new(keys: Keys) -> Self {
        Self {
            keys,
            action_reg: 0,
            direction_reg: 0,
        }
    }
}

impl Input for Keyboard {
    fn update_events(&mut self, _cycles: u128) -> Option<u128> {
        if let Ok(keys) = self.keys.lock() {
            let mut res = 0xf;

            if (*keys).contains(&KeyCode::KeyA) {
                res &= 0b1110;
            }

            if (*keys).contains(&KeyCode::KeyB) {
                res &= 0b1101;
            }

            if (*keys).contains(&KeyCode::Backspace) {
                res &= 0b1011;
            }

            if (*keys).contains(&KeyCode::Enter) {
                res &= 0b0111;
            }

            self.action_reg = res;

            let mut res = 0xf;

            if (*keys).contains(&KeyCode::ArrowRight) {
                res &= 0b1110;
            }

            if (*keys).contains(&KeyCode::ArrowLeft) {
                res &= 0b1101;
            }

            if (*keys).contains(&KeyCode::ArrowUp) {
                res &= 0b1011;
            }

            if (*keys).contains(&KeyCode::ArrowDown) {
                res &= 0b0111;
            }

            self.direction_reg = res;
        }

        None
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        self.action_reg
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        self.direction_reg
    }

    fn save_state(&mut self) -> bool {
        false
    }
}

pub struct GamepadRecorder {
    input: Box<dyn Input>,
    record_file: File,
    action_reg: u8,
    direction_reg: u8,
}

impl GamepadRecorder {
    pub fn new(input: Box<dyn Input>, record_file: String) -> Self {
        Self {
            input,
            record_file: File::create(record_file).expect("Couldn't create gamepad record file"),
            action_reg: 0xff,
            direction_reg: 0xff,
        }
    }
}

impl Input for GamepadRecorder {
    fn update_events(&mut self, cycles: u128) -> Option<u128> {
        self.input.update_events(cycles);

        let new_action_reg = self.input.get_action_gamepad_reg();
        let new_direction_reg = self.input.get_direction_gamepad_reg();

        if self.action_reg != new_action_reg || self.direction_reg != new_direction_reg {
            println!(
                "input update on cycle {} ! 0x{:02x} 0x{:02x}",
                cycles, new_action_reg, new_direction_reg
            );
            if let Err(err) = self.record_file.write_all(&cycles.to_le_bytes()) {
                eprintln!("Failed to write to record file: {}", err);
            };
            if let Err(err) = self
                .record_file
                .write_all(&[new_action_reg, new_direction_reg])
            {
                eprintln!("Failed to write to record file: {}", err);
            }
            if let Err(err) = self.record_file.flush() {
                eprintln!("Failed to flush record file writes: {}", err);
            }
        }

        self.action_reg = new_action_reg;
        self.direction_reg = new_direction_reg;
        None
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        self.action_reg
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        self.direction_reg
    }

    fn save_state(&mut self) -> bool {
        false
    }
}

pub struct GamepadReplay {
    record_file: File,
    action_reg: u8,
    direction_reg: u8,
    next_cycle_update: Option<u128>,
    last_save_state: bool,
}

impl GamepadReplay {
    pub fn new(record_file: String) -> Self {
        let mut file = File::open(record_file).expect("Couldn't open gamepad record file");

        let mut cycles_le: [u8; 16] = [0; 16];

        let next_cycle_update = match file.read_exact(&mut cycles_le) {
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => None,
            Err(err) => panic!("{}", err),
            Ok(_) => Some(u128::from_le_bytes(cycles_le)),
        };

        Self {
            record_file: file,
            action_reg: 0xff,
            direction_reg: 0xff,
            next_cycle_update,
            last_save_state: false,
        }
    }
}

impl Input for GamepadReplay {
    fn update_events(&mut self, cycles: u128) -> Option<u128> {
        if let Some(next_cycle_update) = self.next_cycle_update {
            if cycles >= next_cycle_update {
                let mut inputs: [u8; 2] = [0; 2];

                self.record_file
                    .read_exact(&mut inputs)
                    .expect("Unexpected EOF after cycle but before input");

                self.action_reg = inputs[0];
                self.direction_reg = inputs[1];

                let mut cycles_le: [u8; 16] = [0; 16];

                self.next_cycle_update = match self.record_file.read_exact(&mut cycles_le) {
                    Err(err) if err.kind() == ErrorKind::UnexpectedEof => None,
                    Err(err) => panic!("{}", err),
                    Ok(_) => Some(u128::from_le_bytes(cycles_le)),
                };
            }
        }

        return self.next_cycle_update;
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        self.action_reg
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        self.direction_reg
    }

    fn save_state(&mut self) -> bool {
        if self.last_save_state {
            return false;
        }
        if self.next_cycle_update == None {
            println!("SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE SAVE");

            self.last_save_state = true;
            return true;
        }
        false
    }
}
