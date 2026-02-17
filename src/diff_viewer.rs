use eframe::egui::{self, Color32, RichText, ScrollArea};
use similar::{ChangeTag, TextDiff};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use crate::changed_files::{ChangedFile, FileStatus};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DiffViewMode {
    SideBySide,
    Inline,
}

/// A pre-highlighted text span with color
#[derive(Clone, Debug)]
struct HighlightedSpan {
    text: String,
    color: Color32,
}

/// A diff line with pre-computed highlighting
#[derive(Clone, Debug)]
struct RenderedLine {
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
    spans: Vec<HighlightedSpan>,
    change_type: ChangeTag,
}

/// Computed and pre-rendered diff data
struct DiffData {
    path: String,
    inline_lines: Vec<RenderedLine>,
    old_lines: Vec<RenderedLine>,
    new_lines: Vec<RenderedLine>,
}

enum DiffState {
    Empty,
    Loading { path: String },
    Loaded(DiffData),
}

pub struct DiffViewer {
    state: DiffState,
    receiver: Option<Receiver<DiffData>>,
}

impl Default for DiffViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffViewer {
    pub fn new() -> Self {
        Self {
            state: DiffState::Empty,
            receiver: None,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.state = DiffState::Empty;
        self.receiver = None;
    }

    fn ensure_loading(&mut self, file: &ChangedFile, ctx: &egui::Context) {
        match &self.state {
            DiffState::Loaded(data) if data.path == file.path => return,
            DiffState::Loading { path } if path == &file.path => {
                if let Some(ref receiver) = self.receiver {
                    if let Ok(data) = receiver.try_recv() {
                        self.state = DiffState::Loaded(data);
                        self.receiver = None;
                    }
                }
                return;
            }
            _ => {}
        }

        let (sender, receiver): (Sender<DiffData>, Receiver<DiffData>) = channel();
        let path = file.path.clone();
        let status = file.status.clone();
        let ctx = ctx.clone();

        thread::spawn(move || {
            let data = compute_diff_data(&path, &status);
            let _ = sender.send(data);
            ctx.request_repaint();
        });

        self.state = DiffState::Loading {
            path: file.path.clone(),
        };
        self.receiver = Some(receiver);
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        file: Option<&ChangedFile>,
        view_mode: &mut DiffViewMode,
    ) {
        ui.horizontal(|ui| {
            if let Some(f) = file {
                ui.heading(&f.path);
                ui.label(
                    RichText::new(format!("({})", status_label(&f.status))).color(f.status.color()),
                );
            } else {
                ui.heading("No file selected");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .selectable_label(*view_mode == DiffViewMode::Inline, "Inline")
                    .clicked()
                {
                    *view_mode = DiffViewMode::Inline;
                }
                if ui
                    .selectable_label(*view_mode == DiffViewMode::SideBySide, "Side-by-side")
                    .clicked()
                {
                    *view_mode = DiffViewMode::SideBySide;
                }
            });
        });

        ui.separator();

        let Some(file) = file else {
            ui.label("Select a file from the sidebar to view its diff.");
            return;
        };

        self.ensure_loading(file, ui.ctx());

        match &self.state {
            DiffState::Empty | DiffState::Loading { .. } => {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.spinner();
                    ui.label("Loading diff...");
                });
            }
            DiffState::Loaded(data) => match view_mode {
                DiffViewMode::SideBySide => {
                    show_side_by_side(ui, &data.old_lines, &data.new_lines);
                }
                DiffViewMode::Inline => {
                    show_inline(ui, &data.inline_lines);
                }
            },
        }
    }
}

fn show_side_by_side(ui: &mut egui::Ui, old_lines: &[RenderedLine], new_lines: &[RenderedLine]) {
    let available_width = ui.available_width();
    let half_width = (available_width - 20.0) / 2.0;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (old, new) in old_lines.iter().zip(new_lines.iter()) {
                ui.horizontal(|ui| {
                    render_pane_line(ui, old, half_width, true);
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    render_pane_line(ui, new, half_width, false);
                });
            }
        });
}

fn render_pane_line(ui: &mut egui::Ui, line: &RenderedLine, width: f32, is_old: bool) {
    let bg_color = change_tag_to_bg_color(line.change_type);
    let line_num = if is_old {
        line.old_line_num
    } else {
        line.new_line_num
    };
    let line_num_text = line_num
        .map(|n| format!("{:>4} ", n))
        .unwrap_or_else(|| "     ".to_string());

    ui.horizontal(|ui| {
        ui.set_width(width);

        // Background
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Line number
        ui.label(
            RichText::new(&line_num_text)
                .color(Color32::from_rgb(100, 100, 110))
                .monospace(),
        );

        // Pre-rendered spans
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for span in &line.spans {
                ui.label(RichText::new(&span.text).color(span.color).monospace());
            }
        });
    });
}

fn show_inline(ui: &mut egui::Ui, lines: &[RenderedLine]) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for line in lines {
                let bg_color = change_tag_to_bg_color(line.change_type);
                let line_num_text = match (line.old_line_num, line.new_line_num) {
                    (Some(o), Some(n)) => format!("{:>4} {:>4} ", o, n),
                    (Some(o), None) => format!("{:>4}      ", o),
                    (None, Some(n)) => format!("     {:>4} ", n),
                    (None, None) => "          ".to_string(),
                };

                let prefix = match line.change_type {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };

                ui.horizontal(|ui| {
                    // Background
                    let rect = ui.available_rect_before_wrap();
                    let line_rect =
                        egui::Rect::from_min_size(rect.min, egui::vec2(ui.available_width(), 20.0));
                    ui.painter().rect_filled(line_rect, 0.0, bg_color);

                    // Line numbers
                    ui.label(
                        RichText::new(&line_num_text)
                            .color(Color32::from_rgb(100, 100, 110))
                            .monospace(),
                    );

                    // Prefix
                    let prefix_color = match line.change_type {
                        ChangeTag::Delete => FileStatus::Deleted.color(),
                        ChangeTag::Insert => FileStatus::Added.color(),
                        ChangeTag::Equal => Color32::from_rgb(100, 100, 110),
                    };
                    ui.label(RichText::new(prefix).color(prefix_color).monospace());

                    // Pre-rendered spans
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for span in &line.spans {
                            ui.label(RichText::new(&span.text).color(span.color).monospace());
                        }
                    });
                });
            }
        });
}

fn status_label(status: &FileStatus) -> &'static str {
    match status {
        FileStatus::Added => "added",
        FileStatus::Modified => "modified",
        FileStatus::Deleted => "deleted",
        FileStatus::Renamed => "renamed",
    }
}

fn change_tag_to_bg_color(tag: ChangeTag) -> Color32 {
    match tag {
        ChangeTag::Delete => Color32::from_rgba_unmultiplied(220, 80, 80, 20),
        ChangeTag::Insert => Color32::from_rgba_unmultiplied(80, 200, 120, 20),
        ChangeTag::Equal => Color32::TRANSPARENT,
    }
}

// ============================================================================
// Background computation (all heavy work happens here, off the UI thread)
// ============================================================================

fn compute_diff_data(path: &str, status: &FileStatus) -> DiffData {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let (old_content, new_content) = get_file_contents(path, status);
    let diff_lines = compute_diff(&old_content, &new_content);

    // Detect syntax
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let syntax = syntax_set
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
    let theme = &theme_set.themes["base16-ocean.dark"];

    // Pre-render all lines with syntax highlighting
    let inline_lines = render_lines(&diff_lines, syntax, theme, &syntax_set);

    // Split for side-by-side and render
    let (old_diff, new_diff) = split_for_side_by_side(&diff_lines);
    let old_lines = render_lines(&old_diff, syntax, theme, &syntax_set);
    let new_lines = render_lines(&new_diff, syntax, theme, &syntax_set);

    DiffData {
        path: path.to_string(),
        inline_lines,
        old_lines,
        new_lines,
    }
}

fn render_lines(
    lines: &[DiffLineRaw],
    syntax: &syntect::parsing::SyntaxReference,
    theme: &syntect::highlighting::Theme,
    syntax_set: &SyntaxSet,
) -> Vec<RenderedLine> {
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut result = Vec::with_capacity(lines.len());

    for line in lines {
        let regions = highlighter
            .highlight_line(&line.content, syntax_set)
            .unwrap_or_default();

        let spans: Vec<HighlightedSpan> = regions
            .into_iter()
            .map(|(style, text)| HighlightedSpan {
                text: text.to_string(),
                color: Color32::from_rgba_unmultiplied(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                    style.foreground.a,
                ),
            })
            .collect();

        result.push(RenderedLine {
            old_line_num: line.old_line_num,
            new_line_num: line.new_line_num,
            spans,
            change_type: line.change_type,
        });
    }

    result
}

/// Raw diff line before rendering
struct DiffLineRaw {
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
    content: String,
    change_type: ChangeTag,
}

fn get_file_contents(path: &str, status: &FileStatus) -> (String, String) {
    match status {
        FileStatus::Added => {
            let new_content = std::fs::read_to_string(path).unwrap_or_default();
            (String::new(), new_content)
        }
        FileStatus::Deleted => {
            let old_content = get_jj_file_content(path);
            (old_content, String::new())
        }
        FileStatus::Modified | FileStatus::Renamed => {
            let old_content = get_jj_file_content(path);
            let new_content = std::fs::read_to_string(path).unwrap_or_default();
            (old_content, new_content)
        }
    }
}

fn get_jj_file_content(path: &str) -> String {
    let output = Command::new("jj")
        .args(["file", "show", "-r", "@-", path])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => String::new(),
    }
}

fn compute_diff(old: &str, new: &str) -> Vec<DiffLineRaw> {
    let diff = TextDiff::from_lines(old, new);
    let mut lines = Vec::new();
    let mut old_line_num = 1usize;
    let mut new_line_num = 1usize;

    for change in diff.iter_all_changes() {
        let (old_num, new_num) = match change.tag() {
            ChangeTag::Delete => {
                let num = old_line_num;
                old_line_num += 1;
                (Some(num), None)
            }
            ChangeTag::Insert => {
                let num = new_line_num;
                new_line_num += 1;
                (None, Some(num))
            }
            ChangeTag::Equal => {
                let old_num = old_line_num;
                let new_num = new_line_num;
                old_line_num += 1;
                new_line_num += 1;
                (Some(old_num), Some(new_num))
            }
        };

        lines.push(DiffLineRaw {
            old_line_num: old_num,
            new_line_num: new_num,
            content: change.value().trim_end_matches('\n').to_string(),
            change_type: change.tag(),
        });
    }

    lines
}

fn split_for_side_by_side(diff_lines: &[DiffLineRaw]) -> (Vec<DiffLineRaw>, Vec<DiffLineRaw>) {
    let mut old_lines = Vec::new();
    let mut new_lines = Vec::new();

    for line in diff_lines {
        match line.change_type {
            ChangeTag::Equal => {
                old_lines.push(DiffLineRaw {
                    old_line_num: line.old_line_num,
                    new_line_num: line.new_line_num,
                    content: line.content.clone(),
                    change_type: line.change_type,
                });
                new_lines.push(DiffLineRaw {
                    old_line_num: line.old_line_num,
                    new_line_num: line.new_line_num,
                    content: line.content.clone(),
                    change_type: line.change_type,
                });
            }
            ChangeTag::Delete => {
                old_lines.push(DiffLineRaw {
                    old_line_num: line.old_line_num,
                    new_line_num: line.new_line_num,
                    content: line.content.clone(),
                    change_type: line.change_type,
                });
                new_lines.push(DiffLineRaw {
                    old_line_num: None,
                    new_line_num: None,
                    content: String::new(),
                    change_type: ChangeTag::Equal,
                });
            }
            ChangeTag::Insert => {
                old_lines.push(DiffLineRaw {
                    old_line_num: None,
                    new_line_num: None,
                    content: String::new(),
                    change_type: ChangeTag::Equal,
                });
                new_lines.push(DiffLineRaw {
                    old_line_num: line.old_line_num,
                    new_line_num: line.new_line_num,
                    content: line.content.clone(),
                    change_type: line.change_type,
                });
            }
        }
    }

    (old_lines, new_lines)
}
