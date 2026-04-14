use crate::io::Input;
use crate::logs::{log, LogLevel};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use wasm_bindgen::{JsValue, JsCast};
use web_sys::{window, Document, Event};
use web_sys::js_sys::Function;
use wasm_bindgen::closure::ScopedClosure;

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
    action_reg: Arc<AtomicU8>,
    direction_reg: Arc<AtomicU8>,
    should_save: Arc<AtomicBool>,
    document: Document,
}

impl WebButtonsInput {
    pub fn new(config: WebButtonsInputConfig) -> Self {
        let window = window()
            .expect("Cannot use WebButtonsInput if window doesn't exists");
        let document = window
            .document()
            .expect("Cannot use WebButtonsInput if window.document doesn't exist");

        let action_reg = Arc::new(AtomicU8::new(0xf));
        let direction_reg = Arc::new(AtomicU8::new(0xf));

        let should_save = Arc::new(AtomicBool::new(false));

        Self::register_button_event(config.button_start_id, action_reg.clone(), &document, 3);
        Self::register_button_event(config.button_select_id, action_reg.clone(), &document, 2);
        Self::register_button_event(config.button_b_id, action_reg.clone(), &document, 1);
        Self::register_button_event(config.button_a_id, action_reg.clone(), &document, 0);

        Self::register_button_event(config.button_down_id, direction_reg.clone(), &document, 3);
        Self::register_button_event(config.button_up_id, direction_reg.clone(), &document, 2);
        Self::register_button_event(config.button_left_id, direction_reg.clone(), &document, 1);
        Self::register_button_event(config.button_right_id, direction_reg.clone(), &document, 0);

        let should_save_c = should_save.clone();
        let save_interval: ScopedClosure<'static, dyn FnMut() -> ()> = ScopedClosure::new(move || {
            should_save_c.store(true, Ordering::SeqCst);
        });

        window.set_interval_with_callback_and_timeout_and_arguments_0(&Function::from_closure(save_interval), 5_000);

        Self {
            config,
            action_reg,
            direction_reg,
            should_save,
            document,
        }
    }

    fn register_button_event(
        id: &'static str,
        register: Arc<AtomicU8>,
        document: &Document,
        bit: u8,
    ) {
        let start_button = document.get_element_by_id(id).expect(&format!("Missing element with id: \"{}\"", id));
        let register_c = register.clone();
        let press_event_listener  = move |event: JsValue| {
            let event: Event = Event::unchecked_from_js(event);
            register_c.fetch_and(!(1 << bit), Ordering::SeqCst);
            event.prevent_default();
        };

        let unpress_event_listener  = move |event: JsValue| {
            let event: Event = Event::unchecked_from_js(event);
            register.fetch_or(1 << bit, Ordering::SeqCst);
            event.prevent_default();
        };

        let closure: ScopedClosure<'static, dyn FnMut(JsValue) -> ()> = ScopedClosure::new(press_event_listener.clone());
        start_button.add_event_listener_with_callback("mousedown", &Function::from_closure(closure));

        let closure: ScopedClosure<'static, dyn FnMut(JsValue) -> ()> = ScopedClosure::new(press_event_listener);
        start_button.add_event_listener_with_callback("touchstart", &Function::from_closure(closure));

        let closure: ScopedClosure<'static, dyn FnMut(JsValue) -> ()> = ScopedClosure::new(unpress_event_listener.clone());
        start_button.add_event_listener_with_callback("mouseup", &Function::from_closure(closure));
        let closure: ScopedClosure<'static, dyn FnMut(JsValue) -> ()> = ScopedClosure::new(unpress_event_listener.clone());
        start_button.add_event_listener_with_callback("touchend", &Function::from_closure(closure));

        let closure: ScopedClosure<'static, dyn FnMut(JsValue) -> ()> = ScopedClosure::new(unpress_event_listener);
        start_button.add_event_listener_with_callback("touchcancel", &Function::from_closure(closure));
    }

}

impl Input for WebButtonsInput {
    fn update_events(&mut self, _cycles: u128) -> Option<u128> {
        None
    }

    fn get_action_gamepad_reg(&self) -> u8 {
        self.action_reg.load(Ordering::SeqCst)
    }

    fn get_direction_gamepad_reg(&self) -> u8 {
        self.direction_reg.load(Ordering::SeqCst)
    }

    fn save_state(&mut self) -> bool {
        self.should_save.swap(false, Ordering::SeqCst)
    }
}
