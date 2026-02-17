use eframe::egui::{self, Sense};
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl FileStatus {
    fn symbol(&self) -> &'static str {
        match self {
            FileStatus::Added => "+",
            FileStatus::Modified => "●",
            FileStatus::Deleted => "✕",
            FileStatus::Renamed => "→",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            FileStatus::Added => egui::Color32::from_rgb(80, 200, 120), // Green
            FileStatus::Modified => egui::Color32::from_rgb(140, 200, 140), // Light green
            FileStatus::Deleted => egui::Color32::from_rgb(220, 80, 80), // Red
            FileStatus::Renamed => egui::Color32::from_rgb(220, 180, 80), // Yellow
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
}

/// Cache for changed files list - only fetches on demand
pub struct ChangedFilesCache {
    files: Vec<ChangedFile>,
    loaded: bool,
}

impl Default for ChangedFilesCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangedFilesCache {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            loaded: false,
        }
    }

    /// Get files, fetching only on first call
    pub fn get_files(&mut self) -> &[ChangedFile] {
        if !self.loaded {
            self.refresh();
        }
        &self.files
    }

    /// Manually refresh the file list
    pub fn refresh(&mut self) {
        self.files = fetch_changed_files();
        self.loaded = true;
    }
}

/// Renders the changed files sidebar and returns the selected file
/// Returns (selected_file, refresh_requested)
pub fn show(
    ui: &mut egui::Ui,
    cache: &mut ChangedFilesCache,
    selected: &mut usize,
) -> (Option<ChangedFile>, bool) {
    let mut refresh_requested = false;

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label("Changed Files");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("↻").on_hover_text("Refresh file list").clicked() {
                    refresh_requested = true;
                }
            });
        });
        ui.separator();

        let files = cache.get_files();

        for (i, file) in files.iter().enumerate() {
            let response = changed_file_item(ui, file, *selected == i);

            if response.clicked() {
                *selected = i;
            }
        }
    });

    // Handle refresh after UI
    if refresh_requested {
        cache.refresh();
    }

    let files = cache.get_files();

    // Clamp selection to valid range
    if !files.is_empty() && *selected >= files.len() {
        *selected = files.len() - 1;
    }

    (files.get(*selected).cloned(), refresh_requested)
}

fn changed_file_item(ui: &mut egui::Ui, file: &ChangedFile, selected: bool) -> egui::Response {
    let height = 28.0;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().visuals.clone();

        let bg_color = if selected {
            egui::Color32::from_rgb(55, 60, 70)
        } else if response.hovered() {
            egui::Color32::from_rgb(45, 48, 54)
        } else {
            egui::Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, 6.0, bg_color);

        // Draw status symbol with color
        let status_color = file.status.color();
        ui.painter().text(
            rect.left_center() + egui::vec2(12.0, 0.0),
            egui::Align2::LEFT_CENTER,
            file.status.symbol(),
            egui::TextStyle::Body.resolve(ui.style()),
            status_color,
        );

        // Draw file path
        ui.painter().text(
            rect.left_center() + egui::vec2(28.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &file.path,
            egui::TextStyle::Body.resolve(ui.style()),
            visuals.text_color(),
        );
    }

    response
}

fn fetch_changed_files() -> Vec<ChangedFile> {
    let output = Command::new("jj").args(["st"]).output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_jj_status(&stdout)
}

fn parse_jj_status(output: &str) -> Vec<ChangedFile> {
    let mut files = Vec::new();
    let mut in_changes_section = false;

    for line in output.lines() {
        // Start parsing after "Working copy changes:"
        if line.starts_with("Working copy changes:") {
            in_changes_section = true;
            continue;
        }

        // Stop parsing at "Working copy" line (the commit info)
        if line.starts_with("Working copy ") && !line.starts_with("Working copy changes:") {
            break;
        }

        if !in_changes_section {
            continue;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse status and path (format: "A path/to/file")
        if let Some((status_char, path)) = line.split_once(' ') {
            let status = match status_char {
                "A" => FileStatus::Added,
                "M" => FileStatus::Modified,
                "D" => FileStatus::Deleted,
                "R" => FileStatus::Renamed,
                _ => continue,
            };

            files.push(ChangedFile {
                path: path.to_string(),
                status,
            });
        }
    }

    files
}
