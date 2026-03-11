mod app;
mod clicker;
mod config;
mod groups;
mod hotkey;

use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc, Mutex,
};

fn main() {
    let missing = groups::check_device_access();
    let cfg = config::Config::load();

    // Shared state
    let clicking = Arc::new(AtomicBool::new(false));
    let delay_ms = Arc::new(AtomicU64::new(cfg.delay_ms));
    let hotkey = Arc::new(Mutex::new(cfg.hotkey_evdev()));
    let rebinding = Arc::new(AtomicBool::new(false));
    let new_key: Arc<Mutex<Option<evdev::Key>>> = Arc::new(Mutex::new(None));

    if missing.is_empty() {
        // Spawn background threads only if we have permissions
        hotkey::spawn_listeners(
            Arc::clone(&clicking),
            Arc::clone(&hotkey),
            Arc::clone(&rebinding),
            Arc::clone(&new_key),
        );

        let clicker_clicking = Arc::clone(&clicking);
        let clicker_delay = Arc::clone(&delay_ms);
        std::thread::spawn(move || clicker::run(clicker_clicking, clicker_delay));
    }

    let app = app::WclickerApp::new(
        clicking,
        delay_ms,
        hotkey,
        rebinding,
        new_key,
        missing,
    );

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 280.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native("wclicker", options, Box::new(|_cc| Ok(Box::new(app))))
        .expect("failed to run eframe");
}
