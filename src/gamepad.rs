use crate::display::Display;
use gilrs::{Button, GamepadId, Gilrs};
use minifb::Key;

pub struct Gamepad {
    gilrs: Gilrs,
    gamepad_id: Option<GamepadId>,
}

pub trait Input {
    fn update_events(&mut self);
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
    fn update_events(&mut self) {
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

impl Input for Display {
    fn update_events(&mut self) {}

    fn get_action_gamepad_reg(&self) -> u8 {
        let mut res = 0xf;

        if self.window.is_key_down(Key::A) {
            res &= 0b1110;
        }

        if self.window.is_key_down(Key::B) {
            res &= 0b1101;
        }

        if self.window.is_key_down(Key::Backspace) {
            res &= 0b1011;
        }

        if self.window.is_key_down(Key::Enter) {
            res &= 0b0111;
        }

        res
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        let mut res = 0xf;

        if self.window.is_key_down(Key::Right) {
            res &= 0b1110;
        }

        if self.window.is_key_down(Key::Left) {
            res &= 0b1101;
        }

        if self.window.is_key_down(Key::Up) {
            res &= 0b1011;
        }

        if self.window.is_key_down(Key::Down) {
            res &= 0b0111;
        }

        res
    }
}
