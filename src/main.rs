use eframe::egui;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

use crate::changed_files::{ChangedFile, ChangedFilesCache};
use crate::diff_viewer::{DiffViewMode, DiffViewer};
use crate::theme::set_rusty_theme;

mod changed_files;
mod diff_viewer;
mod sidebar_item;
mod theme;
mod title_bar;

struct MyApp {
    selected_file_idx: usize,
    selected_changed_file: Option<ChangedFile>,
    changed_files_cache: ChangedFilesCache,
    diff_viewer: DiffViewer,
    diff_view_mode: DiffViewMode,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            selected_file_idx: 0,
            selected_changed_file: None,
            changed_files_cache: ChangedFilesCache::new(),
            diff_viewer: DiffViewer::new(),
            diff_view_mode: DiffViewMode::SideBySide,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        set_rusty_theme(ctx);

        // Top title bar
        egui::TopBottomPanel::top("title_bar").show(ctx, |ui| {
            title_bar::show(ui, "le diff");
        });

        // Track previous selection to detect changes
        let prev_selection = self.selected_file_idx;
        let mut refresh_requested = false;

        // Sidebar (LEFT)
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(220.0)
            .frame(
                egui::Frame::new()
                    // slightly lighter than central panel
                    .fill(egui::Color32::from_rgb(36, 38, 43))
                    .inner_margin(egui::Margin::symmetric(16, 20)),
            )
            .show(ctx, |ui| {
                let (selected_file, refreshed) = changed_files::show(
                    ui,
                    &mut self.changed_files_cache,
                    &mut self.selected_file_idx,
                );
                self.selected_changed_file = selected_file;
                refresh_requested = refreshed;
            });

        // Invalidate diff cache if selection changed or refresh requested
        if prev_selection != self.selected_file_idx || refresh_requested {
            self.diff_viewer.invalidate_cache();
        }

        // Main content
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(30, 32, 36))
                    .inner_margin(egui::Margin::symmetric(24, 20)),
            )
            .show(ctx, |ui| {
                self.diff_viewer.show(
                    ui,
                    self.selected_changed_file.as_ref(),
                    &mut self.diff_view_mode,
                );
            });
    }
}

fn main() -> eframe::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();
    debug!("Starting application in debug mode...");

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Counter App",
        native_options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}
