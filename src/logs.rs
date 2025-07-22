use std::collections::HashSet;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Hash, Eq)]
pub enum LogLevel {
    Infos,
    Debug,
    OpcodeDump,
    HaltCycles,
    Error,
}

static LOG_LEVEL: OnceLock<HashSet<LogLevel>> = OnceLock::new();

pub fn set_log_level(verbosity: String) {
    let mut set: HashSet<LogLevel> = HashSet::new();
    for level in verbosity.split(",") {
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
            "none" => {}
            _ => panic!("Unknown log level \"{}\"", level),
        }
    }
    if let Err(value) = LOG_LEVEL.set(set) {
        panic!("Log level is already set with value {:?}", value);
    }
}

pub fn log(level: LogLevel, s: String) {
    if let Some(set) = LOG_LEVEL.get() {
        if set.contains(&level) {
            println!("[{:?}] {}", level, s);
        }
    }
}

pub fn elog(level: LogLevel, s: String) {
    if let Some(set) = LOG_LEVEL.get() {
        if set.contains(&level) {
            eprintln!("[{:?}] {}", level, s);
        }
    }
}
