#![allow(unused)]
use super::toggle_switch;
use eframe::egui;

/// top toolbar for development \
/// self-contained toppanel
pub struct DevToolbar {}

impl DevToolbar {
    pub fn show(ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .inner_margin(egui::Margin::same(5))
                    .fill(egui::Color32::LIGHT_GREEN),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // button to show current app positon
                    if ui.button("APP INFO").clicked() {
                        if let Some(viewport) = ctx.input(|i| i.viewport().outer_rect) {
                            println!("--- APP INFO ---");
                            println!("app position: {}", viewport.min); // top-left corner
                            println!("app size: {}", viewport.size());
                            println!("app dark mode: {:?}", ctx.style().visuals.dark_mode);

                            // ctx.memory(|mem| {
                            //     // no luck getting anything useful
                            //     println!("{}", mem.layer_ids().len());
                            // });

                            println!("--- END APP INFO ---");
                        }
                    }
                    ui.separator();

                    // light / dark mode toogler
                    ui.label("DARK MODE");
                    let mut is_dark_mode = ctx.style().visuals.dark_mode;
                    let response = toggle_switch::toggle_ui(ui, &mut is_dark_mode);
                    if response.changed() {
                        if is_dark_mode {
                            ctx.set_visuals(egui::Visuals::dark());
                        } else {
                            ctx.set_visuals(egui::Visuals::light());
                        }
                    }
                    ui.separator();

                    // more
                    // ctx.set_debug_on_hover(true);
                });
                // ui.add_space(5.0); // no use, aesthetic
            });
    }
}
