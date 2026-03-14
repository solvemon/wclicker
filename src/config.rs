use evdev::Key;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClickMode {
    #[default]
    Toggle,
    Hold,
}

impl ClickMode {
    pub fn as_u8(self) -> u8 {
        match self {
            ClickMode::Toggle => 0,
            ClickMode::Hold => 1,
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => ClickMode::Hold,
            _ => ClickMode::Toggle,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub delay_ms: u64,
    pub hotkey: u16,
    #[serde(default)]
    pub mode: ClickMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            delay_ms: 50,
            hotkey: Key::KEY_F8.code(),
            mode: ClickMode::default(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let dir = dirs_or_default();
        dir.join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(contents) = toml::to_string_pretty(self) {
            let _ = fs::write(&path, contents);
        }
    }

    pub fn hotkey_evdev(&self) -> Key {
        Key::new(self.hotkey)
    }
}

fn dirs_or_default() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        })
        .join("wclicker")
}
