use std::collections::HashSet;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Hash, Eq)]
pub enum LogLevel {
    Infos,
    Debug,
    OpcodeDump,
    HaltCycles,
    AudioLatency,
    Error,
}

static LOG_LEVEL: OnceLock<HashSet<LogLevel>> = OnceLock::new();

pub fn set_log_level(verbosity: impl Into<String>) {
    let mut set: HashSet<LogLevel> = HashSet::new();
    for level in verbosity.into().split(",") {
        match level {
            "infos" => {
                set.insert(LogLevel::Infos);
            }
            "debug" => {
                set.insert(LogLevel::Debug);
            }
            "opcode_dump" => {
                set.insert(LogLevel::OpcodeDump);
            }
            "halt_cycles" => {
                set.insert(LogLevel::HaltCycles);
            }
            "errors" => {
                set.insert(LogLevel::Error);
            }
            "audio_latency" => {
                set.insert(LogLevel::AudioLatency);
            }
            "none" => {}
            _ => panic!("Unknown log level \"{}\"", level),
        }
    }
    if let Err(value) = LOG_LEVEL.set(set) {
        panic!("Log level is already set with value {:?}", value);
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn log(level: LogLevel, s: impl Into<String>) {
    if let Some(set) = LOG_LEVEL.get() {
        if set.contains(&level) {
            println!("[{:?}] {}", level, s.into());
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn elog(level: LogLevel, s: impl Into<String>) {
    if let Some(set) = LOG_LEVEL.get() {
        if set.contains(&level) {
            eprintln!("[{:?}] {}", level, s.into());
        }
    }
}

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn console_log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(s: &str);
}

#[cfg(target_family = "wasm")]
pub fn log(level: LogLevel, s: impl Into<String>) {
    if let Some(set) = LOG_LEVEL.get() {
        if set.contains(&level) {
            console_log(&format!("[{:?}] {}", level, s.into()));
        }
    }
}

#[cfg(target_family = "wasm")]
pub fn elog(level: LogLevel, s: impl Into<String>) {
    if let Some(set) = LOG_LEVEL.get() {
        if set.contains(&level) {
            console_error(&format!("[{:?}] {}", level, s.into()));
        }
    }
}
