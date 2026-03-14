use evdev::Key;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8},
    Mutex,
};

use crate::config::Config;

/// All cross-thread state shared between the GUI, hotkey listeners, and clicker.
pub struct SharedState {
    pub clicking: AtomicBool,
    pub delay_ms: AtomicU64,
    pub hotkey: Mutex<Key>,
    pub rebinding: AtomicBool,
    pub new_key: Mutex<Option<Key>>,
    pub mode: AtomicU8,
}

impl SharedState {
    pub fn from_config(cfg: &Config) -> Self {
        Self {
            clicking: AtomicBool::new(false),
            delay_ms: AtomicU64::new(cfg.delay_ms),
            hotkey: Mutex::new(cfg.hotkey_evdev()),
            rebinding: AtomicBool::new(false),
            new_key: Mutex::new(None),
            mode: AtomicU8::new(cfg.mode.as_u8()),
        }
    }
}
