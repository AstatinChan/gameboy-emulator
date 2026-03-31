use crate::io::Input;
use web_sys::{window, Document};

pub struct WebButtonsInputConfig {
    pub button_a_id: &'static str,
    pub button_b_id: &'static str,
    pub button_start_id: &'static str,
    pub button_select_id: &'static str,
    pub button_up_id: &'static str,
    pub button_down_id: &'static str,
    pub button_left_id: &'static str,
    pub button_right_id: &'static str,
}

pub struct WebButtonsInput {
    config: WebButtonsInputConfig,
    action_reg: u8,
    direction_reg: u8,
    document: Document,
}

impl WebButtonsInput {
    pub fn new(config: WebButtonsInputConfig) -> Self {
        Self {
            config,
            action_reg: 0,
            direction_reg: 0,
            document: window()
                .expect("Cannot use WebButtonsInput if window doesn't exists")
                .document()
                .expect("Cannot use WebButtonsInput if window.document doesn't exist"),
        }
    }
}

impl Input for WebButtonsInput {
    fn update_events(&mut self, _cycles: u128) -> Option<u128> {
        let mut res = 0xf;

        if self
            .document
            .query_selector(&format!("button#{}:active", self.config.button_a_id))
            .expect("Query Selector failed")
            .is_some()
        {
            res &= 0b1110;
        }

        if self
            .document
            .query_selector(&format!("button#{}:active", self.config.button_b_id))
            .expect("Query Selector failed")
            .is_some()
        {
            res &= 0b1101;
        }

        if self
            .document
            .query_selector(&format!("button#{}:active", self.config.button_select_id))
            .expect("Query Selector failed")
            .is_some()
        {
            res &= 0b1011;
        }

        if self
            .document
            .query_selector(&format!("button#{}:active", self.config.button_start_id))
            .expect("Query Selector failed")
            .is_some()
        {
            res &= 0b0111;
        }

        self.action_reg = res;

        let mut res = 0xf;

        if self
            .document
            .query_selector(&format!("button#{}:active", self.config.button_right_id))
            .expect("Query Selector failed")
            .is_some()
        {
            res &= 0b1110;
        }

        if self
            .document
            .query_selector(&format!("button#{}:active", self.config.button_left_id))
            .expect("Query Selector failed")
            .is_some()
        {
            res &= 0b1101;
        }

        if self
            .document
            .query_selector(&format!("button#{}:active", self.config.button_up_id))
            .expect("Query Selector failed")
            .is_some()
        {
            res &= 0b1011;
        }

        if self
            .document
            .query_selector(&format!("button#{}:active", self.config.button_down_id))
            .expect("Query Selector failed")
            .is_some()
        {
            res &= 0b0111;
        }

        self.direction_reg = res;

        None
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        self.action_reg
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        self.direction_reg
    }

    fn save_state(&mut self) -> bool {
        // Unimplemented
        false
    }
}
