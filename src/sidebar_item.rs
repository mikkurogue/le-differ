use eframe::egui::{self, Sense};

pub fn sidebar_item(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let height = 28.0;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().visuals.clone();

        let bg_color = if selected {
            egui::Color32::from_rgb(55, 60, 70) // selected surface
        } else if response.hovered() {
            egui::Color32::from_rgb(45, 48, 54) // hover surface
        } else {
            egui::Color32::TRANSPARENT
        };

        ui.painter().rect_filled(
            rect, 6.0, // rounding
            bg_color,
        );

        ui.painter().text(
            rect.left_center() + egui::vec2(12.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::TextStyle::Body.resolve(ui.style()),
            visuals.text_color(),
        );
    }

    response
}
