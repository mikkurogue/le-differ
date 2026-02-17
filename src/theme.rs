use eframe::egui;

pub fn set_rusty_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Name("Title".into()),
            egui::FontId::new(20.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(17.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::new(14.0, egui::FontFamily::Monospace),
        ),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(12);

    ctx.set_style(style);

    let mut visuals = egui::Visuals::dark();

    // Base Layers
    visuals.panel_fill = egui::Color32::from_rgb(26, 28, 32); // deepest background
    visuals.window_fill = egui::Color32::from_rgb(30, 32, 36); // content surface
    visuals.faint_bg_color = egui::Color32::from_rgb(38, 40, 45);

    // Text
    visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 215));

    // Rust Accent
    let rust = egui::Color32::from_rgb(190, 80, 20);
    let rust_hover = egui::Color32::from_rgb(210, 95, 30);
    let rust_active = egui::Color32::from_rgb(160, 60, 10);

    visuals.selection.bg_fill = rust;
    visuals.selection.stroke = egui::Stroke::new(1.0, rust_hover);

    // Widgets
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(42, 45, 50);
    visuals.widgets.hovered.bg_fill = rust_hover;
    visuals.widgets.active.bg_fill = rust_active;

    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 70));

    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, rust);

    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, rust_hover);

    // Rounded modern feel
    let rounding = 4.0.into();

    visuals.widgets.noninteractive.corner_radius = rounding;

    visuals.widgets.inactive.corner_radius = rounding;
    visuals.widgets.hovered.corner_radius = rounding;
    visuals.widgets.active.corner_radius = rounding;

    ctx.set_visuals(visuals);
}
