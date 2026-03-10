use egui::{Color32, RichText};
use evdev::Key;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

pub struct WclickerApp {
    // Shared state
    pub clicking: Arc<AtomicBool>,
    pub delay_ms: Arc<AtomicU64>,
    pub hotkey: Arc<Mutex<Key>>,
    pub rebinding: Arc<AtomicBool>,
    pub new_key: Arc<Mutex<Option<Key>>>,

    // Local UI state
    pub missing_groups: Vec<String>,
    delay_input: String,
}

impl WclickerApp {
    pub fn new(
        clicking: Arc<AtomicBool>,
        delay_ms: Arc<AtomicU64>,
        hotkey: Arc<Mutex<Key>>,
        rebinding: Arc<AtomicBool>,
        new_key: Arc<Mutex<Option<Key>>>,
        missing_groups: Vec<String>,
    ) -> Self {
        let delay = delay_ms.load(Ordering::Relaxed);
        Self {
            clicking,
            delay_ms,
            hotkey,
            rebinding,
            new_key,
            missing_groups,
            delay_input: delay.to_string(),
        }
    }
}

impl eframe::App for WclickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if a rebind completed
        {
            let mut nk = self.new_key.lock().unwrap();
            if let Some(key) = nk.take() {
                *self.hotkey.lock().unwrap() = key;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // --- Group check error screen ---
            if !self.missing_groups.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("⚠ Missing required groups:")
                            .color(Color32::YELLOW)
                            .size(16.0),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(self.missing_groups.join(", "))
                            .color(Color32::RED)
                            .monospace(),
                    );
                    ui.add_space(16.0);
                    ui.label("Fix:");
                    ui.add_space(4.0);
                    let cmd = format!(
                        "sudo usermod -aG {} $USER",
                        self.missing_groups.join(",")
                    );
                    ui.label(RichText::new(&cmd).monospace().color(Color32::LIGHT_BLUE));
                    ui.add_space(8.0);
                    ui.label("Then log out and back in.");
                });
                return;
            }

            // --- Normal UI ---
            let is_clicking = self.clicking.load(Ordering::Relaxed);

            ui.add_space(8.0);

            // Status
            ui.horizontal(|ui| {
                ui.label("Status:");
                if is_clicking {
                    ui.label(RichText::new("● CLICKING").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("● STOPPED").color(Color32::RED));
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Hotkey
            ui.horizontal(|ui| {
                ui.label("Hotkey:");
                let is_rebinding = self.rebinding.load(Ordering::Relaxed);
                let hotkey_label = if is_rebinding {
                    "[ press a key... ]".to_string()
                } else {
                    let hk = self.hotkey.lock().unwrap();
                    format!("[ {:?} ]", *hk)
                };
                if ui.button(&hotkey_label).clicked() && !is_rebinding {
                    self.rebinding.store(true, Ordering::Relaxed);
                }
            });

            ui.add_space(4.0);

            // Delay
            ui.horizontal(|ui| {
                ui.label("Delay:");
                let response = ui.text_edit_singleline(&mut self.delay_input);
                ui.label("ms");
                if response.lost_focus() {
                    if let Ok(v) = self.delay_input.parse::<u64>() {
                        if v > 0 {
                            self.delay_ms.store(v, Ordering::Relaxed);
                        }
                    }
                    // Sync display back to actual value
                    self.delay_input =
                        self.delay_ms.load(Ordering::Relaxed).to_string();
                }
            });

            ui.add_space(12.0);

            // Toggle button
            ui.vertical_centered(|ui| {
                let btn_label = if is_clicking { "Stop" } else { "Start" };
                if ui.button(btn_label).clicked() {
                    self.clicking.fetch_xor(true, Ordering::AcqRel);
                }
            });
        });

        // Repaint continuously so status reflects hotkey changes promptly
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}
