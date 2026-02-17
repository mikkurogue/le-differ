use eframe::egui;
use tracing::debug;

pub fn show(ui: &mut egui::Ui, title: &str) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(34, 36, 40))
        .inner_margin(egui::Margin::symmetric(20, 14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).text_style(egui::TextStyle::Heading));

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.menu_button("☰", |ui| {
                        ui.set_min_width(180.0);
                        ui.label("Menu item 1");
                        ui.label("Menu item 2");
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::LEFT), |ui| {
                    if ui.button("X").clicked() {
                        debug!("Close app button clicked... std::process::exit(0)");
                        std::process::exit(0);
                    }
                });
            });
        });
}
