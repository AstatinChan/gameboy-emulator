use crate::state;
use gilrs::{Button, GamepadId, Gilrs};
use state::GBState;
use std::fs::File;
use std::io::{Write, Read, ErrorKind};
use minifb::Key;

pub struct Gamepad {
    gilrs: Gilrs,
    gamepad_id: Option<GamepadId>,
}

pub trait Input {
    fn update_events(&mut self, cycles: u128, state: &GBState);
    fn get_action_gamepad_reg(&self) -> u8;
    fn get_direction_gamepad_reg(&self) -> u8;
}

impl Gamepad {
    pub fn new() -> Self {
        let mut gilrs = Gilrs::new().unwrap();

        let gamepad_id = if let Some((gamepad_id, _gamepad)) = gilrs.gamepads().next() {
            println!("Found Gamepad id: {:?}", gamepad_id);
            Some(gamepad_id)
        } else {
            println!("No gamepad found");
            None
        };

        Self { gilrs, gamepad_id }
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
    fn update_events(&mut self, _cycles: u128, _state: &GBState) {
        while let Some(_) = self.gilrs.next_event() {}
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
}

pub struct Keyboard {
    action_reg: u8,
    direction_reg: u8,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            action_reg: 0,
            direction_reg: 0
        }
    }
}

impl Input for Keyboard {
    fn update_events(&mut self, _cycles: u128, state: &GBState) {
        let mut res = 0xf;

        if state.mem.display.window.is_key_down(Key::A) {
            res &= 0b1110;
        }

        if state.mem.display.window.is_key_down(Key::B) {
            res &= 0b1101;
        }

        if state.mem.display.window.is_key_down(Key::Backspace) {
            res &= 0b1011;
        }

        if state.mem.display.window.is_key_down(Key::Enter) {
            res &= 0b0111;
        }

        self.action_reg = res;

        let mut res = 0xf;

        if state.mem.display.window.is_key_down(Key::Right) {
            res &= 0b1110;
        }

        if state.mem.display.window.is_key_down(Key::Left) {
            res &= 0b1101;
        }

        if state.mem.display.window.is_key_down(Key::Up) {
            res &= 0b1011;
        }

        if state.mem.display.window.is_key_down(Key::Down) {
            res &= 0b0111;
        }

        self.direction_reg = res;
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        self.action_reg
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        self.direction_reg
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
    fn update_events(&mut self, cycles: u128, state: &GBState) {
        self.input.update_events(cycles, state);

        let new_action_reg = self.input.get_action_gamepad_reg();
        let new_direction_reg = self.input.get_direction_gamepad_reg();

        if self.action_reg != new_action_reg || self.direction_reg != new_direction_reg {
            println!("input update on cycle {} ! 0x{:02x} 0x{:02x}", cycles, new_action_reg, new_direction_reg);
            if let Err(err) = self.record_file.write_all(&cycles.to_le_bytes()) {
                eprintln!("Failed to write to record file: {}", err);
            };
            if let Err(err) = self.record_file.write_all(&[new_action_reg, new_direction_reg]) {
                eprintln!("Failed to write to record file: {}", err);
            }
            if let Err(err) = self.record_file.flush() {
                eprintln!("Failed to flush record file writes: {}", err);
            }
        }

        self.action_reg = new_action_reg;
        self.direction_reg = new_direction_reg;
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        self.action_reg
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        self.direction_reg
    }
}

pub struct GamepadReplay {
    record_file: File,
    action_reg: u8,
    direction_reg: u8,
    next_cycle_update: Option<u128>,
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
        }
    }
}

impl Input for GamepadReplay {
    fn update_events(&mut self, cycles: u128, _state: &GBState) {
        if let Some(next_cycle_update) = self.next_cycle_update {
            if cycles > next_cycle_update {
                let mut inputs: [u8; 2] = [0; 2];

                self.record_file.read_exact(&mut inputs).expect("Unexpected EOF after cycle but before input");

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
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        self.action_reg
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        self.direction_reg
    }
}

