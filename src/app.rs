use crate::config::Config;
use crate::state::SharedState;
use egui::{
    Align, Button, Color32, FontId, Frame, Layout, Margin, RichText, Rounding, Stroke, TextEdit,
    Vec2,
};
use std::sync::{
    atomic::Ordering,
    Arc,
};

const ACCENT: Color32 = Color32::from_rgb(100, 160, 255);
const GREEN: Color32 = Color32::from_rgb(80, 200, 120);
const RED: Color32 = Color32::from_rgb(220, 80, 80);
const SURFACE: Color32 = Color32::from_rgb(35, 35, 42);
const PANEL_BG: Color32 = Color32::from_rgb(28, 28, 34);

pub struct WclickerApp {
    state: Arc<SharedState>,
    missing_groups: Vec<String>,
    delay_input: String,
}

impl WclickerApp {
    pub fn new(state: Arc<SharedState>, missing_groups: Vec<String>) -> Self {
        let delay = state.delay_ms.load(Ordering::Relaxed);
        Self {
            state,
            missing_groups,
            delay_input: delay.to_string(),
        }
    }

    fn save_config(&self) {
        let cfg = Config {
            delay_ms: self.state.delay_ms.load(Ordering::Relaxed),
            hotkey: self.state.hotkey.lock().unwrap().code(),
            mode: Default::default(),
        };
        cfg.save();
    }
}

impl eframe::App for WclickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply dark theme
        ctx.set_visuals(egui::Visuals::dark());

        // Check if a rebind completed
        {
            let new_key = self.state.new_key.lock().unwrap().take();
            if let Some(key) = new_key {
                *self.state.hotkey.lock().unwrap() = key;
                self.save_config();
            }
        }

        egui::CentralPanel::default()
            .frame(Frame::none().fill(PANEL_BG).inner_margin(Margin::same(16.0)))
            .show(ctx, |ui| {
                // --- Permission error screen ---
                if !self.missing_groups.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("Missing permissions")
                                .color(Color32::YELLOW)
                                .size(18.0)
                                .strong(),
                        );
                        ui.add_space(12.0);
                        for err in &self.missing_groups {
                            ui.label(RichText::new(err).color(RED).monospace());
                            ui.add_space(4.0);
                        }
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Log out and back in after fixing.")
                                .color(Color32::GRAY),
                        );
                    });
                    return;
                }

                // --- Normal UI ---
                let is_clicking = self.state.clicking.load(Ordering::Relaxed);

                // Status banner
                let (status_color, status_text) = if is_clicking {
                    (GREEN, "CLICKING")
                } else {
                    (RED, "STOPPED")
                };

                Frame::none()
                    .fill(SURFACE)
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui: &mut egui::Ui| {
                        ui.horizontal(|ui| {
                            // Colored dot
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::splat(10.0),
                                egui::Sense::hover(),
                            );
                            ui.painter()
                                .circle_filled(rect.center(), 5.0, status_color);

                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(status_text)
                                    .color(status_color)
                                    .font(FontId::monospace(14.0))
                                    .strong(),
                            );
                        });
                    });

                ui.add_space(12.0);

                // Settings section
                Frame::none()
                    .fill(SURFACE)
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui: &mut egui::Ui| {
                        // Hotkey row
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Hotkey")
                                    .color(Color32::LIGHT_GRAY)
                                    .size(13.0),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let is_rebinding = self.state.rebinding.load(Ordering::Relaxed);
                                let (label, color) = if is_rebinding {
                                    ("press a key...".to_string(), ACCENT)
                                } else {
                                    let hk = self.state.hotkey.lock().unwrap();
                                    (format!("{:?}", *hk), Color32::WHITE)
                                };
                                let btn = Button::new(
                                    RichText::new(&label).color(color).monospace().size(13.0),
                                )
                                .fill(PANEL_BG)
                                .stroke(Stroke::new(1.0, ACCENT.linear_multiply(0.4)))
                                .rounding(Rounding::same(4.0))
                                .min_size(Vec2::new(100.0, 0.0));

                                if ui.add(btn).clicked() && !is_rebinding {
                                    self.state.rebinding.store(true, Ordering::Relaxed);
                                }
                            });
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Delay row
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Delay")
                                    .color(Color32::LIGHT_GRAY)
                                    .size(13.0),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new("ms")
                                        .color(Color32::GRAY)
                                        .size(13.0),
                                );
                                let response = ui.add(
                                    TextEdit::singleline(&mut self.delay_input)
                                        .desired_width(60.0)
                                        .font(FontId::monospace(13.0)),
                                );
                                if response.lost_focus() {
                                    if let Ok(v) = self.delay_input.parse::<u64>() {
                                        if v > 0 {
                                            self.state.delay_ms.store(v, Ordering::Relaxed);
                                            self.save_config();
                                        }
                                    }
                                    self.delay_input =
                                        self.state.delay_ms.load(Ordering::Relaxed).to_string();
                                }
                            });
                        });
                    });

                ui.add_space(12.0);

                // Toggle button
                ui.vertical_centered(|ui| {
                    let (btn_label, btn_color) = if is_clicking {
                        ("Stop", RED)
                    } else {
                        ("Start", GREEN)
                    };
                    let btn = Button::new(
                        RichText::new(btn_label)
                            .size(16.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(btn_color.linear_multiply(0.8))
                    .stroke(Stroke::NONE)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(ui.available_width(), 36.0));

                    if ui.add(btn).clicked() {
                        self.state.clicking.fetch_xor(true, Ordering::AcqRel);
                    }
                });
            });

        // Repaint continuously so status reflects hotkey changes promptly
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}
