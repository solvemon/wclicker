mod app;
mod clicker;
mod config;
mod groups;
mod hotkey;
mod state;

use std::sync::Arc;

fn main() {
    let missing = groups::check_device_access();
    let cfg = config::Config::load();
    let state = Arc::new(state::SharedState::from_config(&cfg));

    if missing.is_empty() {
        hotkey::spawn_listeners(Arc::clone(&state));

        let clicker_state = Arc::clone(&state);
        std::thread::spawn(move || clicker::run(clicker_state));
    }

    let app = app::WclickerApp::new(state, missing);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 320.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native("wclicker", options, Box::new(|_cc| Ok(Box::new(app))))
        .expect("failed to run eframe");
}
