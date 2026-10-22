use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    time::Instant,
};

use eframe::egui;
use serde::{Deserialize, Serialize};

use pka_lab_inspector::commands;
use pka_lab_inspector::inspector;
use pka_lab_inspector::model::{Analysis, Checkpoint, Status};
use pka_lab_inspector::reports;

/// Available tabs in the inspector UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Tab {
    #[default]
    Overview,
    Checkpoints,
    NextActions,
    DeviceCommands,
    RawXml,
    Diagnostics,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Overview,
        Tab::Checkpoints,
        Tab::NextActions,
        Tab::DeviceCommands,
        Tab::RawXml,
        Tab::Diagnostics,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Checkpoints => "Checkpoints",
            Tab::NextActions => "Next Actions",
            Tab::DeviceCommands => "Device Commands",
            Tab::RawXml => "Raw XML",
            Tab::Diagnostics => "Diagnostics",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Tab::Overview => "Ctrl+1",
            Tab::Checkpoints => "Ctrl+2",
            Tab::NextActions => "Ctrl+3",
            Tab::DeviceCommands => "Ctrl+4",
            Tab::RawXml => "Ctrl+5",
            Tab::Diagnostics => "Ctrl+6",
        }
    }
}

/// Status filter options for checkpoint filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StatusFilter {
    #[default]
    All,
    Passed,
    Failed,
    Unverified,
}

/// User preferences saved across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub dark_theme: bool,
    pub last_activity_path: Option<String>,
    pub last_progress_path: Option<String>,
    pub default_export_dir: Option<String>,
    pub active_tab: Tab,
    pub compact_view: bool,
    pub show_only_manual_unchecked: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            dark_theme: true,
            last_activity_path: None,
            last_progress_path: None,
            default_export_dir: None,
            active_tab: Tab::Overview,
            compact_view: false,
            show_only_manual_unchecked: false,
        }
    }
}

/// Manual checklist items and notes for a specific activity, keyed by activity path or name.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityChecklist {
    pub checked_checkpoints: HashMap<String, bool>,
    pub notes: HashMap<String, String>,
}

/// Messages sent from background threads to the UI.
pub enum LoadMessage {
    ActivityLoaded {
        path: PathBuf,
        result: Result<Analysis, String>,
    },
    ProgressLoaded {
        path: PathBuf,
        result: Result<Analysis, String>,
    },
}

/// Main Inspector Application State.
pub struct InspectorApp {
    pub analysis: Option<Analysis>,
    pub original_analysis: Option<Analysis>,
    pub progress_analysis: Option<Analysis>,
    pub activity_path: Option<PathBuf>,
    pub progress_path: Option<PathBuf>,

    // Search and filters
    pub search_query: String,
    pub selected_device: Option<String>,
    pub selected_category: Option<String>,
    pub status_filter: StatusFilter,
    pub current_tab: Tab,
    pub expanded_checkpoint_ids: HashMap<String, bool>,

    // Device commands view state
    pub command_device_filter: Option<String>,

    // Raw XML search
    pub xml_search_query: String,

    // Status / feedback
    pub is_loading: bool,
    pub loading_description: Option<String>,
    pub status_message: Option<String>,
    pub status_timestamp: Option<Instant>,

    // Preferences and checklists
    pub preferences: Preferences,
    pub manual_checklists: HashMap<String, ActivityChecklist>,

    // Background thread channels
    pub tx: Sender<LoadMessage>,
    pub rx: Receiver<LoadMessage>,
}

impl InspectorApp {
    pub fn new(cc: Option<&eframe::CreationContext>, initial_path: Option<PathBuf>) -> Self {
        let (tx, rx) = channel();

        let mut app = Self {
            analysis: None,
            original_analysis: None,
            progress_analysis: None,
            activity_path: None,
            progress_path: None,

            search_query: String::new(),
            selected_device: None,
            selected_category: None,
            status_filter: StatusFilter::All,
            current_tab: Tab::Overview,
            expanded_checkpoint_ids: HashMap::new(),

            command_device_filter: None,
            xml_search_query: String::new(),

            is_loading: false,
            loading_description: None,
            status_message: Some("Ready".into()),
            status_timestamp: Some(Instant::now()),

            preferences: Preferences::default(),
            manual_checklists: HashMap::new(),

            tx,
            rx,
        };

        // Load persisted state if available
        if let Some(ctx_ref) = cc {
            if let Some(storage) = ctx_ref.storage {
                if let Some(prefs_str) = storage.get_string("preferences") {
                    if let Ok(prefs) = serde_json::from_str::<Preferences>(&prefs_str) {
                        app.preferences = prefs;
                        app.current_tab = app.preferences.active_tab;
                    }
                }
                if let Some(checklists_str) = storage.get_string("manual_checklists") {
                    if let Ok(checklists) =
                        serde_json::from_str::<HashMap<String, ActivityChecklist>>(&checklists_str)
                    {
                        app.manual_checklists = checklists;
                    }
                }
            }

            // Configure native dark visuals
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = egui::CornerRadius::same(6);
            ctx_ref.egui_ctx.set_visuals(visuals);
        }

        // If an initial file was specified, load it
        if let Some(path) = initial_path {
            app.load_activity_background(path, cc.map(|c| c.egui_ctx.clone()));
        }

        app
    }

    /// Key used to isolate manual checklist data per activity.
    pub fn activity_key(&self) -> String {
        if let Some(ref path) = self.activity_path {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string())
        } else {
            "default".to_string()
        }
    }

    /// Get current activity's manual checklist.
    pub fn current_checklist(&self) -> &ActivityChecklist {
        static DEFAULT_CHECKLIST: std::sync::LazyLock<ActivityChecklist> =
            std::sync::LazyLock::new(ActivityChecklist::default);
        let key = self.activity_key();
        self.manual_checklists.get(&key).unwrap_or(&DEFAULT_CHECKLIST)
    }

    /// Mutably get or create current activity's manual checklist.
    pub fn current_checklist_mut(&mut self) -> &mut ActivityChecklist {
        let key = self.activity_key();
        self.manual_checklists.entry(key).or_default()
    }

    /// Set manual check status for a checkpoint in the current activity.
    pub fn set_manual_check(&mut self, checkpoint_id: &str, checked: bool) {
        let checklist = self.current_checklist_mut();
        if checked {
            checklist.checked_checkpoints.insert(checkpoint_id.to_string(), true);
        } else {
            checklist.checked_checkpoints.remove(checkpoint_id);
        }
    }

    /// Check if a checkpoint is marked checked in the current activity checklist.
    #[allow(dead_code)]
    pub fn is_manually_checked(&self, checkpoint_id: &str) -> bool {
        self.current_checklist()
            .checked_checkpoints
            .get(checkpoint_id)
            .copied()
            .unwrap_or(false)
    }

    /// Total manual checks completed for the current activity.
    pub fn manual_check_count(&self) -> (usize, usize) {
        let total = self.analysis.as_ref().map(|a| a.checkpoints.len()).unwrap_or(0);
        let completed = if let Some(ref a) = self.analysis {
            let cl = self.current_checklist();
            a.checkpoints
                .iter()
                .filter(|c| cl.checked_checkpoints.get(&c.id).copied().unwrap_or(false))
                .count()
        } else {
            0
        };
        (completed, total)
    }

    /// Set status message with timestamp.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_timestamp = Some(Instant::now());
    }

    /// Load activity file in a background thread.
    pub fn load_activity_background(&mut self, path: PathBuf, ctx: Option<egui::Context>) {
        self.is_loading = true;
        self.loading_description = Some(format!("Loading activity: {}", path.display()));
        self.set_status(format!("Loading {}...", path.display()));

        let tx = self.tx.clone();
        let target_path = path.clone();
        std::thread::spawn(move || {
            let res = inspector::load(&target_path).map_err(|e| format!("{:#}", e));
            let _ = tx.send(LoadMessage::ActivityLoaded {
                path: target_path,
                result: res,
            });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    /// Load progress file in a background thread.
    pub fn load_progress_background(&mut self, path: PathBuf, ctx: Option<egui::Context>) {
        self.is_loading = true;
        self.loading_description = Some(format!("Loading progress: {}", path.display()));
        self.set_status(format!("Loading progress {}...", path.display()));

        let tx = self.tx.clone();
        let target_path = path.clone();
        std::thread::spawn(move || {
            let res = inspector::load(&target_path).map_err(|e| format!("{:#}", e));
            let _ = tx.send(LoadMessage::ProgressLoaded {
                path: target_path,
                result: res,
            });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    /// Reload the current activity file.
    pub fn reload_activity(&mut self, ctx: Option<egui::Context>) {
        if let Some(path) = self.activity_path.clone() {
            self.load_activity_background(path, ctx);
        }
    }

    /// Drain background messages and update state.
    pub fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            self.is_loading = false;
            self.loading_description = None;
            match msg {
                LoadMessage::ActivityLoaded { path, result } => match result {
                    Ok(analysis) => {
                        let count = analysis.checkpoints.len();
                        self.activity_path = Some(path.clone());
                        self.preferences.last_activity_path = Some(path.to_string_lossy().to_string());
                        self.original_analysis = Some(analysis.clone());
                        let mut act = analysis;

                        // If a progress file was already loaded, compare against it immediately
                        if let Some(ref prog) = self.progress_analysis {
                            inspector::compare(&mut act, prog);
                            self.set_status(format!(
                                "Loaded {} ({} checkpoints) and compared with progress.",
                                path.display(),
                                count
                            ));
                        } else {
                            self.set_status(format!(
                                "Successfully loaded {} ({} checkpoints).",
                                path.display(),
                                count
                            ));
                        }
                        self.analysis = Some(act);
                    }
                    Err(e) => {
                        self.set_status(format!("Failed to load activity: {e}"));
                    }
                },
                LoadMessage::ProgressLoaded { path, result } => match result {
                    Ok(prog) => {
                        self.progress_path = Some(path.clone());
                        self.preferences.last_progress_path = Some(path.to_string_lossy().to_string());
                        self.progress_analysis = Some(prog.clone());

                        if let Some(ref mut act) = self.analysis {
                            inspector::compare(act, &prog);
                            let summary = act.summary();
                            self.set_status(format!(
                                "Progress compared: {} passed, {} failed, {} unverified.",
                                summary.passed, summary.failed, summary.unverified
                            ));
                        } else {
                            self.set_status(format!(
                                "Loaded progress {}. Load an activity to compare.",
                                path.display()
                            ));
                        }
                    }
                    Err(e) => {
                        self.set_status(format!("Failed to load progress: {e}"));
                    }
                },
            }
        }
    }

    /// Reset comparison and revert back to original activity analysis.
    pub fn clear_progress(&mut self) {
        if let Some(orig) = &self.original_analysis {
            self.analysis = Some(orig.clone());
        }
        self.progress_analysis = None;
        self.progress_path = None;
        self.set_status("Progress cleared. Showing original activity assessment.");
    }

    /// Get unique devices from current checkpoints.
    pub fn unique_devices(&self) -> Vec<String> {
        let mut devices = Vec::new();
        if let Some(ref a) = self.analysis {
            for c in &a.checkpoints {
                if !c.device.is_empty() && !devices.contains(&c.device) {
                    devices.push(c.device.clone());
                }
            }
        }
        devices.sort();
        devices
    }

    /// Get unique categories from current checkpoints.
    pub fn unique_categories(&self) -> Vec<String> {
        let mut categories = Vec::new();
        if let Some(ref a) = self.analysis {
            for c in &a.checkpoints {
                if !c.category.is_empty() && !categories.contains(&c.category) {
                    categories.push(c.category.clone());
                }
            }
        }
        categories.sort();
        categories
    }

    /// Filter checkpoint indices matching all active filters.
    pub fn filtered_checkpoint_indices(&self) -> Vec<usize> {
        let Some(ref a) = self.analysis else {
            return Vec::new();
        };

        let search = self.search_query.trim().to_lowercase();
        let manual_filter = self.preferences.show_only_manual_unchecked;
        let checklist = self.current_checklist();

        a.checkpoints
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                // Status filter
                match self.status_filter {
                    StatusFilter::All => {}
                    StatusFilter::Passed => {
                        if c.status != Status::Passed {
                            return false;
                        }
                    }
                    StatusFilter::Failed => {
                        if c.status != Status::Failed {
                            return false;
                        }
                    }
                    StatusFilter::Unverified => {
                        if c.status != Status::Unverified {
                            return false;
                        }
                    }
                }

                // Device filter
                if let Some(ref dev) = self.selected_device {
                    if &c.device != dev {
                        return false;
                    }
                }

                // Category filter
                if let Some(ref cat) = self.selected_category {
                    if &c.category != cat {
                        return false;
                    }
                }

                // Manual unchecked filter
                if manual_filter && checklist.checked_checkpoints.get(&c.id).copied().unwrap_or(false) {
                    return false;
                }

                // Search query
                if !search.is_empty() {
                    let matches_id = c.id.to_lowercase().contains(&search);
                    let matches_dev = c.device.to_lowercase().contains(&search);
                    let matches_iface = c.interface.to_lowercase().contains(&search);
                    let matches_prop = c.property.to_lowercase().contains(&search);
                    let matches_exp = c.expected.to_lowercase().contains(&search);
                    let matches_curr = c.current.as_ref().is_some_and(|s| s.to_lowercase().contains(&search));
                    let matches_expl = c.explanation.to_lowercase().contains(&search);
                    let matches_cat = c.category.to_lowercase().contains(&search);

                    if !(matches_id
                        || matches_dev
                        || matches_iface
                        || matches_prop
                        || matches_exp
                        || matches_curr
                        || matches_expl
                        || matches_cat)
                    {
                        return false;
                    }
                }

                true
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Copy text to system clipboard via egui context.
    pub fn copy_text_with_feedback(&mut self, ctx: &egui::Context, text: String, label: &str) {
        ctx.copy_text(text);
        self.set_status(format!("Copied {label} to clipboard."));
    }

    /// Handle keyboard shortcuts.
    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            let ctrl = i.modifiers.command; // Ctrl on Windows/Linux, Cmd on Mac
            let shift = i.modifiers.shift;

            // Ctrl+O: Open Activity
            if ctrl && !shift && i.key_pressed(egui::Key::O) {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Packet Tracer (*.pka, *.pkt)", &["pka", "pkt"])
                    .add_filter("All Files (*.*)", &["*"])
                    .pick_file()
                {
                    self.load_activity_background(path, Some(ctx.clone()));
                }
            }

            // Ctrl+P or Ctrl+Shift+O: Open Progress
            if (ctrl && i.key_pressed(egui::Key::P)) || (ctrl && shift && i.key_pressed(egui::Key::O)) {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Packet Tracer (*.pka, *.pkt)", &["pka", "pkt"])
                    .add_filter("All Files (*.*)", &["*"])
                    .pick_file()
                {
                    self.load_progress_background(path, Some(ctx.clone()));
                }
            }

            // Ctrl+R: Reload Activity
            if ctrl && i.key_pressed(egui::Key::R) {
                self.reload_activity(Some(ctx.clone()));
            }

            // Tabs Ctrl+1 through Ctrl+6
            if ctrl && i.key_pressed(egui::Key::Num1) {
                self.current_tab = Tab::Overview;
            }
            if ctrl && i.key_pressed(egui::Key::Num2) {
                self.current_tab = Tab::Checkpoints;
            }
            if ctrl && i.key_pressed(egui::Key::Num3) {
                self.current_tab = Tab::NextActions;
            }
            if ctrl && i.key_pressed(egui::Key::Num4) {
                self.current_tab = Tab::DeviceCommands;
            }
            if ctrl && i.key_pressed(egui::Key::Num5) {
                self.current_tab = Tab::RawXml;
            }
            if ctrl && i.key_pressed(egui::Key::Num6) {
                self.current_tab = Tab::Diagnostics;
            }

            // Escape: Clear search
            if i.key_pressed(egui::Key::Escape) {
                self.search_query.clear();
                self.selected_device = None;
                self.selected_category = None;
                self.status_filter = StatusFilter::All;
            }
        });
    }

    /// Render Top Panel (Toolbar, File Actions, Quick Summary).
    pub fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading("🔍 Packet Tracer Inspector");
                    ui.separator();

                    // Open Activity button
                    if ui.button("📂 Open Activity (Ctrl+O)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Packet Tracer (*.pka, *.pkt)", &["pka", "pkt"])
                            .add_filter("All Files (*.*)", &["*"])
                            .pick_file()
                        {
                            self.load_activity_background(path, Some(ctx.clone()));
                        }
                    }

                    // Reload button
                    let reload_enabled = self.activity_path.is_some() && !self.is_loading;
                    if ui
                        .add_enabled(reload_enabled, egui::Button::new("🔄 Reload (Ctrl+R)"))
                        .clicked()
                    {
                        self.reload_activity(Some(ctx.clone()));
                    }

                    ui.separator();

                    // Open Progress button
                    let progress_enabled = self.analysis.is_some() && !self.is_loading;
                    if ui
                        .add_enabled(progress_enabled, egui::Button::new("📥 Open Progress (Ctrl+P)"))
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Packet Tracer (*.pka, *.pkt)", &["pka", "pkt"])
                            .add_filter("All Files (*.*)", &["*"])
                            .pick_file()
                        {
                            self.load_progress_background(path, Some(ctx.clone()));
                        }
                    }

                    // Clear progress button
                    if self.progress_analysis.is_some() {
                        if ui.button("❌ Clear Progress").clicked() {
                            self.clear_progress();
                        }
                    }

                    ui.separator();

                    // Export CSV
                    let export_enabled = self.analysis.is_some();
                    if ui.add_enabled(export_enabled, egui::Button::new("📊 Export CSV")).clicked() {
                        if let Some(ref a) = self.analysis {
                            match reports::csv(a) {
                                Ok(data) => {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("CSV (*.csv)", &["csv"])
                                        .set_file_name("activity_report.csv")
                                        .save_file()
                                    {
                                        if let Err(e) = std::fs::write(&path, data) {
                                            self.set_status(format!("Error saving CSV: {e}"));
                                        } else {
                                            self.set_status(format!("Saved CSV to {}", path.display()));
                                        }
                                    }
                                }
                                Err(e) => self.set_status(format!("Failed to generate CSV: {e}")),
                            }
                        }
                    }

                    // Export Markdown
                    if ui.add_enabled(export_enabled, egui::Button::new("📝 Export Markdown")).clicked() {
                        if let Some(ref a) = self.analysis {
                            let data = reports::markdown(a);
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Markdown (*.md)", &["md"])
                                .set_file_name("activity_report.md")
                                .save_file()
                            {
                                if let Err(e) = std::fs::write(&path, data) {
                                    self.set_status(format!("Error saving Markdown: {e}"));
                                } else {
                                    self.set_status(format!("Saved Markdown to {}", path.display()));
                                }
                            }
                        }
                    }

                    // Export CLI Script
                    if ui.add_enabled(export_enabled, egui::Button::new("💻 Export CLI Script")).clicked() {
                        if let Some(ref a) = self.analysis {
                            let script = commands::device_script(&a.checkpoints, None);
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("CLI Script (*.txt, *.ios)", &["txt", "ios", "cli"])
                                .set_file_name("device_commands.txt")
                                .save_file()
                            {
                                if let Err(e) = std::fs::write(&path, script) {
                                    self.set_status(format!("Error saving CLI script: {e}"));
                                } else {
                                    self.set_status(format!("Saved CLI script to {}", path.display()));
                                }
                            }
                        }
                    }

                    // Right-side summary metrics
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(ref a) = self.analysis {
                            let s = a.summary();
                            ui.label(format!("Score: {:.1}/{:.1}", s.verified_score, s.total_score));
                            ui.separator();

                            // Unverified badge
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 179, 0),
                                format!("{} Unverified", s.unverified),
                            );

                            // Failed badge
                            ui.colored_label(
                                egui::Color32::from_rgb(244, 67, 54),
                                format!("{} Failed", s.failed),
                            );

                            // Passed badge
                            ui.colored_label(
                                egui::Color32::from_rgb(76, 175, 80),
                                format!("{} Passed", s.passed),
                            );

                            ui.label(format!("Total: {}", s.total));
                        } else {
                            ui.label("No file loaded");
                        }
                    });
                });
            });
    }

    /// Render Left Panel (Filters, Navigation, Activity Checklist).
    pub fn render_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side_panel")
            .resizable(true)
            .default_width(280.0)
            .width_range(220.0..=450.0)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::same(12)))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.strong("🔎 Search & Filters");
                    ui.add_space(4.0);

                    // Search box
                    ui.horizontal(|ui| {
                        let _search_edit = ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("Search checkpoints, devices... (Esc)"),
                        );
                        if !self.search_query.is_empty() && ui.small_button("✕").clicked() {
                            self.search_query.clear();
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Status filter
                    ui.strong("Status Filter");
                    let total_passed = self.analysis.as_ref().map(|a| a.summary().passed).unwrap_or(0);
                    let total_failed = self.analysis.as_ref().map(|a| a.summary().failed).unwrap_or(0);
                    let total_unverified = self.analysis.as_ref().map(|a| a.summary().unverified).unwrap_or(0);
                    let total_all = self.analysis.as_ref().map(|a| a.checkpoints.len()).unwrap_or(0);

                    ui.radio_value(&mut self.status_filter, StatusFilter::All, format!("All ({total_all})"));
                    ui.radio_value(
                        &mut self.status_filter,
                        StatusFilter::Passed,
                        format!("Passed ({total_passed})"),
                    );
                    ui.radio_value(
                        &mut self.status_filter,
                        StatusFilter::Failed,
                        format!("Failed ({total_failed})"),
                    );
                    ui.radio_value(
                        &mut self.status_filter,
                        StatusFilter::Unverified,
                        format!("Unverified ({total_unverified})"),
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Device Filter
                    ui.strong("Device Filter");
                    let devices = self.unique_devices();
                    let current_device_label = self
                        .selected_device
                        .as_deref()
                        .unwrap_or("All Devices");

                    egui::ComboBox::from_id_salt("device_filter_combo")
                        .selected_text(current_device_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(self.selected_device.is_none(), "All Devices")
                                .clicked()
                            {
                                self.selected_device = None;
                            }
                            for dev in &devices {
                                let is_selected = self.selected_device.as_deref() == Some(dev);
                                if ui.selectable_label(is_selected, dev).clicked() {
                                    self.selected_device = Some(dev.clone());
                                }
                            }
                        });

                    ui.add_space(8.0);

                    // Category Filter
                    ui.strong("Category Filter");
                    let categories = self.unique_categories();
                    let current_category_label = self
                        .selected_category
                        .as_deref()
                        .unwrap_or("All Categories");

                    egui::ComboBox::from_id_salt("category_filter_combo")
                        .selected_text(current_category_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(self.selected_category.is_none(), "All Categories")
                                .clicked()
                            {
                                self.selected_category = None;
                            }
                            for cat in &categories {
                                let is_selected = self.selected_category.as_deref() == Some(cat);
                                if ui.selectable_label(is_selected, cat).clicked() {
                                    self.selected_category = Some(cat.clone());
                                }
                            }
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Manual Checklist Section (Activity Keyed)
                    ui.strong("📋 Activity Checklist");
                    let (completed_checks, total_checks) = self.manual_check_count();
                    ui.label(format!("{completed_checks} / {total_checks} completed"));

                    ui.checkbox(
                        &mut self.preferences.show_only_manual_unchecked,
                        "Show only unchecked",
                    );

                    if ui.small_button("Clear Checklist for Activity").clicked() {
                        let key = self.activity_key();
                        self.manual_checklists.remove(&key);
                        self.set_status("Activity checklist reset.");
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Preferences
                    ui.strong("⚙ View Options");
                    ui.checkbox(&mut self.preferences.compact_view, "Compact rows");

                    ui.add_space(12.0);
                    // Active files info
                    ui.separator();
                    ui.add_space(4.0);
                    ui.strong("📁 Loaded Files");
                    if let Some(ref path) = self.activity_path {
                        ui.label(format!("Activity:\n{}", path.file_name().unwrap_or_default().to_string_lossy()));
                    } else {
                        ui.label("Activity: None");
                    }

                    if let Some(ref path) = self.progress_path {
                        ui.add_space(4.0);
                        ui.label(format!("Progress:\n{}", path.file_name().unwrap_or_default().to_string_lossy()));
                    }
                });
            });
    }

    /// Render Status Bar (Bottom Panel).
    pub fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        let filtered_count = self.filtered_checkpoint_indices().len();
        let total_count = self.analysis.as_ref().map(|a| a.checkpoints.len()).unwrap_or(0);

        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::symmetric(12, 6)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.is_loading {
                        ui.spinner();
                        if let Some(ref desc) = self.loading_description {
                            ui.label(desc);
                        } else {
                            ui.label("Loading...");
                        }
                    } else if let Some(ref msg) = self.status_message {
                        ui.label(msg);
                    } else {
                        ui.label("Ready");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if total_count > 0 {
                            ui.label(format!("Showing {filtered_count} of {total_count} checkpoints"));
                        }
                    });
                });
            });
    }

    /// Render Central Panel with Tabs.
    pub fn render_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab bar
            ui.horizontal(|ui| {
                for tab in Tab::ALL {
                    let is_active = self.current_tab == tab;
                    let text = format!("{} ({})", tab.title(), tab.shortcut());
                    if ui.selectable_label(is_active, text).clicked() {
                        self.current_tab = tab;
                        self.preferences.active_tab = tab;
                    }
                }
            });
            ui.separator();

            // When no analysis is loaded
            if self.analysis.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.heading("Welcome to Packet Tracer Inspector");
                    ui.add_space(8.0);
                    ui.label("Load a Cisco Packet Tracer activity (.pka) or network file (.pkt) to inspect assessment items, generate device CLI commands, and compare student progress.");
                    ui.add_space(16.0);
                    if ui.button("📂 Open Activity File (Ctrl+O)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Packet Tracer (*.pka, *.pkt)", &["pka", "pkt"])
                            .add_filter("All Files (*.*)", &["*"])
                            .pick_file()
                        {
                            self.load_activity_background(path, Some(ctx.clone()));
                        }
                    }
                });
                return;
            }

            // Render current tab content
            match self.current_tab {
                Tab::Overview => self.render_overview_tab(ui),
                Tab::Checkpoints => self.render_checkpoints_tab(ui, ctx),
                Tab::NextActions => self.render_next_actions_tab(ui, ctx),
                Tab::DeviceCommands => self.render_device_commands_tab(ui, ctx),
                Tab::RawXml => self.render_raw_xml_tab(ui, ctx),
                Tab::Diagnostics => self.render_diagnostics_tab(ui, ctx),
            }
        });
    }

    /// Tab 1: Overview
    pub fn render_overview_tab(&mut self, ui: &mut egui::Ui) {
        let Some(ref a) = self.analysis else { return };
        let s = a.summary();
        let timings = a.timings.clone();

        let mut device_stats: BTreeMap<String, (usize, usize, usize, usize, f64)> = BTreeMap::new();
        for c in &a.checkpoints {
            let entry = device_stats.entry(c.device.clone()).or_insert((0, 0, 0, 0, 0.0));
            entry.0 += 1; // total
            match c.status {
                Status::Passed => entry.1 += 1,
                Status::Failed => entry.2 += 1,
                Status::Unverified => entry.3 += 1,
            }
            entry.4 += c.weight;
        }

        let (checked, total) = self.manual_check_count();
        let mut select_device: Option<String> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading("Assessment Overview");
            ui.add_space(8.0);

            // Metrics cards in a row
            ui.horizontal(|ui| {
                // Card 1: Checkpoints
                egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Checkpoints");
                        ui.heading(format!("{}", s.total));
                        ui.label(format!("Passed: {} | Failed: {} | Unverified: {}", s.passed, s.failed, s.unverified));
                    });
                });

                // Card 2: Score
                egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Verified Score");
                        ui.heading(format!("{:.1} / {:.1} pts", s.verified_score, s.total_score));
                        let pct = if s.total_score > 0.0 { s.verified_score / s.total_score * 100.0 } else { 0.0 };
                        ui.label(format!("{:.1}% score achieved", pct));
                    });
                });

                // Card 3: Verification Coverage
                egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Verification Coverage");
                        ui.heading(format!("{:.1}%", s.coverage * 100.0));
                        ui.label(format!("{} verified of {}", s.passed + s.failed, s.total));
                    });
                });

                // Card 4: Manual Checklist
                egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Manual Checklist");
                        ui.heading(format!("{checked} / {total}"));
                        let pct = if total > 0 { checked as f64 / total as f64 * 100.0 } else { 0.0 };
                        ui.label(format!("{:.1}% verified by you", pct));
                    });
                });
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Device summary breakdown table
            ui.heading("Per-Device Breakdown");
            ui.add_space(6.0);

            egui::Grid::new("device_overview_grid")
                .striped(true)
                .min_col_width(120.0)
                .show(ui, |ui| {
                    ui.strong("Device");
                    ui.strong("Total");
                    ui.strong("Passed");
                    ui.strong("Failed");
                    ui.strong("Unverified");
                    ui.strong("Points");
                    ui.strong("Action");
                    ui.end_row();

                    for (dev, (tot, pass, fail, unver, pts)) in &device_stats {
                        let dev_name = if dev.is_empty() { "(General / Unassigned)" } else { dev.as_str() };
                        ui.label(dev_name);
                        ui.label(tot.to_string());
                        ui.colored_label(egui::Color32::from_rgb(76, 175, 80), pass.to_string());
                        ui.colored_label(egui::Color32::from_rgb(244, 67, 54), fail.to_string());
                        ui.colored_label(egui::Color32::from_rgb(255, 179, 0), unver.to_string());
                        ui.label(format!("{pts:.1}"));

                        if ui.small_button("View Checkpoints").clicked() {
                            select_device = Some(dev.clone());
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Performance and parse timings
            ui.heading("Timings & Profiling");
            ui.add_space(6.0);
            egui::Grid::new("timings_grid").striped(true).show(ui, |ui| {
                ui.strong("Phase");
                ui.strong("Elapsed Time");
                ui.end_row();

                for (phase, duration) in &timings {
                    ui.label(phase);
                    ui.label(format!("{:.3} s ({:.1} ms)", duration, duration * 1000.0));
                    ui.end_row();
                }
            });
        });

        if let Some(dev) = select_device {
            self.selected_device = Some(dev);
            self.current_tab = Tab::Checkpoints;
        }
    }

    /// Tab 2: Checkpoints with virtualized row rendering
    pub fn render_checkpoints_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let filtered_indices = self.filtered_checkpoint_indices();
        let total_filtered = filtered_indices.len();
        let compact = self.preferences.compact_view;

        let mut copy_filtered_csv = false;
        let mut toggle_check: Option<(String, bool)> = None;
        let mut toggle_expand: Option<String> = None;
        let mut to_copy: Option<(String, &'static str)> = None;

        ui.horizontal(|ui| {
            ui.label(format!("{total_filtered} checkpoints matching filters"));
            if ui.button("Copy CSV of Filtered").clicked() {
                copy_filtered_csv = true;
            }
        });
        ui.add_space(4.0);

        if copy_filtered_csv {
            if let Some(ref a) = self.analysis {
                let sub_analysis = Analysis {
                    checkpoints: filtered_indices.iter().map(|&i| a.checkpoints[i].clone()).collect(),
                    diagnostics: Vec::new(),
                    xml: String::new(),
                    timings: Vec::new(),
                };
                if let Ok(csv_text) = reports::csv(&sub_analysis) {
                    to_copy = Some((csv_text, "filtered CSV"));
                }
            }
        }

        // Virtualized scroll area
        let row_height = if compact { 44.0 } else { 74.0 };

        if let Some(ref a) = self.analysis {
            let checklist = self.current_checklist();
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show_rows(ui, row_height, total_filtered, |ui, row_range| {
                    for &idx in &filtered_indices[row_range] {
                        let c = &a.checkpoints[idx];
                        let is_expanded = self.expanded_checkpoint_ids.get(&c.id).copied().unwrap_or(false);
                        let mut is_checked = checklist.checked_checkpoints.get(&c.id).copied().unwrap_or(false);

                        egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Manual checklist checkbox
                                    if ui.checkbox(&mut is_checked, "").changed() {
                                        toggle_check = Some((c.id.clone(), is_checked));
                                    }

                                    // Status pill
                                    let (status_text, status_color) = match c.status {
                                        Status::Passed => ("PASSED", egui::Color32::from_rgb(76, 175, 80)),
                                        Status::Failed => ("FAILED", egui::Color32::from_rgb(244, 67, 54)),
                                        Status::Unverified => ("UNVERIFIED", egui::Color32::from_rgb(255, 179, 0)),
                                    };
                                    ui.colored_label(status_color, format!("[{status_text}]"));

                                    // Device & Interface
                                    let dev_iface = if c.interface.is_empty() {
                                        c.device.clone()
                                    } else {
                                        format!("{} ({})", c.device, c.interface)
                                    };
                                    ui.strong(dev_iface);
                                    ui.label("•");

                                    // Property
                                    ui.label(&c.property);
                                    ui.label("•");

                                    // Expected vs Current
                                    ui.label(format!("Expected: {}", c.expected));
                                    if let Some(ref curr) = c.current {
                                        ui.label(format!("| Current: {curr}"));
                                    }

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        // Expand/collapse button
                                        let expand_label = if is_expanded { "▲ Less" } else { "▼ More" };
                                        if ui.small_button(expand_label).clicked() {
                                            toggle_expand = Some(c.id.clone());
                                        }

                                        // Copy CLI button
                                        if ui.small_button("📋 Copy CLI").clicked() {
                                            let cmd = commands::generate(c);
                                            let text_to_copy = if cmd.supported && !cmd.gui {
                                                cmd.text
                                            } else if cmd.gui {
                                                format!("GUI step required: {}", cmd.text)
                                            } else {
                                                format!("! {}", cmd.explanation)
                                            };
                                            to_copy = Some((text_to_copy, "checkpoint CLI command"));
                                        }

                                        ui.label(format!("{:.1} pts", c.weight));
                                    });
                                });

                                // Expanded detail view
                                if is_expanded {
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);

                                    egui::Grid::new(format!("detail_grid_{}", c.id))
                                        .num_columns(2)
                                        .spacing([12.0, 4.0])
                                        .show(ui, |ui| {
                                            ui.strong("ID:");
                                            ui.label(&c.id);
                                            ui.end_row();

                                            ui.strong("Category:");
                                            ui.label(&c.category);
                                            ui.end_row();

                                            ui.strong("Path:");
                                            ui.label(&c.path);
                                            ui.end_row();

                                            ui.strong("Explanation:");
                                            ui.label(&c.explanation);
                                            ui.end_row();

                                            if !c.evidence.is_empty() {
                                                ui.strong("Evidence:");
                                                ui.vertical(|ui| {
                                                    for ev in &c.evidence {
                                                        ui.label(format!("• {ev}"));
                                                    }
                                                });
                                                ui.end_row();
                                            }

                                            // Command preview
                                            let cmd = commands::generate(c);
                                            ui.strong("Remediation CLI:");
                                            ui.vertical(|ui| {
                                                if cmd.supported && !cmd.gui {
                                                    ui.monospace(&cmd.text);
                                                } else if cmd.gui {
                                                    ui.colored_label(egui::Color32::LIGHT_BLUE, format!("GUI: {}", cmd.text));
                                                } else {
                                                    ui.colored_label(egui::Color32::LIGHT_RED, format!("Unsupported: {}", cmd.explanation));
                                                }
                                            });
                                            ui.end_row();
                                        });
                                }
                            });
                    }
                });
        }

        // Apply mutations
        if let Some((id, checked)) = toggle_check {
            self.set_manual_check(&id, checked);
        }
        if let Some(id) = toggle_expand {
            let current = self.expanded_checkpoint_ids.get(&id).copied().unwrap_or(false);
            self.expanded_checkpoint_ids.insert(id, !current);
        }
        if let Some((text, label)) = to_copy {
            self.copy_text_with_feedback(ctx, text, label);
        }
    }

    /// Tab 3: Next Actions (Remediation plan for incomplete items).
    pub fn render_next_actions_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(ref a) = self.analysis else { return };

        let mut incomplete: Vec<Checkpoint> = a
            .checkpoints
            .iter()
            .filter(|c| c.status != Status::Passed)
            .cloned()
            .collect();

        if incomplete.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.heading("🎉 All Checkpoints Passed!");
                ui.label("No pending remediation actions remain for this activity.");
            });
            return;
        }

        // Sort by weight descending so highest-value fixes appear first
        incomplete.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

        let mut toggle_check: Option<(String, bool)> = None;
        let mut to_copy: Option<(String, &'static str)> = None;

        ui.horizontal(|ui| {
            ui.heading(format!("Next Actions ({} remaining)", incomplete.len()));
            ui.separator();
            if ui.button("Copy All Remediation Commands").clicked() {
                let script = commands::device_script(&incomplete, None);
                to_copy = Some((script, "remediation script"));
            }
        });
        ui.add_space(8.0);

        let checklist = self.current_checklist();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, c) in incomplete.iter().enumerate() {
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut checked = checklist.checked_checkpoints.get(&c.id).copied().unwrap_or(false);
                            if ui.checkbox(&mut checked, "").changed() {
                                toggle_check = Some((c.id.clone(), checked));
                            }

                            ui.strong(format!("#{}. {} - {}", idx + 1, c.device, c.property));
                            if !c.interface.is_empty() {
                                ui.label(format!("(Interface: {})", c.interface));
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.colored_label(
                                    if c.status == Status::Failed {
                                        egui::Color32::from_rgb(244, 67, 54)
                                    } else {
                                        egui::Color32::from_rgb(255, 179, 0)
                                    },
                                    format!("{:?}", c.status),
                                );
                                ui.strong(format!("{:.1} pts", c.weight));
                            });
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Expected: ");
                            ui.monospace(&c.expected);
                            if let Some(ref curr) = c.current {
                                ui.label(" | Actual: ");
                                ui.monospace(curr);
                            }
                        });

                        ui.add_space(4.0);
                        let cmd = commands::generate(c);
                        if cmd.supported && !cmd.gui {
                            ui.horizontal(|ui| {
                                ui.monospace(&cmd.text);
                                if ui.small_button("📋 Copy Command").clicked() {
                                    to_copy = Some((cmd.text.clone(), "CLI command"));
                                }
                            });
                        } else if cmd.gui {
                            ui.colored_label(egui::Color32::LIGHT_BLUE, format!("Manual GUI Action: {}", cmd.text));
                        } else {
                            ui.colored_label(
                                egui::Color32::LIGHT_RED,
                                format!("No automatic CLI command available: {}", cmd.explanation),
                            );
                        }
                    });
                ui.add_space(6.0);
            }
        });

        if let Some((id, checked)) = toggle_check {
            self.set_manual_check(&id, checked);
        }
        if let Some((text, label)) = to_copy {
            self.copy_text_with_feedback(ctx, text, label);
        }
    }

    /// Tab 4: Device Commands Script Generation.
    pub fn render_device_commands_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(ref a) = self.analysis else { return };

        let devices = self.unique_devices();
        let current_filter = self.command_device_filter.clone().unwrap_or_else(|| "All Devices".into());
        let target_device = self.command_device_filter.as_deref();
        let script = commands::device_script(&a.checkpoints, target_device);

        let mut select_dev: Option<Option<String>> = None;
        let mut do_copy: Option<(String, &'static str)> = None;
        let mut status_update: Option<String> = None;

        let active_filter = self.command_device_filter.clone();

        ui.horizontal(|ui| {
            ui.heading("Device Configuration Scripts");
            ui.separator();

            // Device selector
            egui::ComboBox::from_id_salt("cmd_device_selector")
                .selected_text(&current_filter)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(active_filter.is_none(), "All Devices").clicked() {
                        select_dev = Some(None);
                    }
                    for dev in &devices {
                        let is_sel = active_filter.as_deref() == Some(dev.as_str());
                        if ui.selectable_label(is_sel, dev).clicked() {
                            select_dev = Some(Some(dev.clone()));
                        }
                    }
                });

            ui.separator();

            // Copy button
            if ui.button("📋 Copy Script to Clipboard").clicked() {
                do_copy = Some((script.clone(), "device script"));
            }

            // Export to file button
            if ui.button("💾 Save Script to File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CLI Script (*.txt, *.ios)", &["txt", "ios", "cli"])
                    .set_file_name("device_script.txt")
                    .save_file()
                {
                    if let Err(e) = std::fs::write(&path, &script) {
                        status_update = Some(format!("Error saving script: {e}"));
                    } else {
                        status_update = Some(format!("Script saved to {}", path.display()));
                    }
                }
            }
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut script.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(24),
            );
        });

        if let Some(dev) = select_dev {
            self.command_device_filter = dev;
        }
        if let Some((text, label)) = do_copy {
            self.copy_text_with_feedback(ctx, text, label);
        }
        if let Some(msg) = status_update {
            self.set_status(msg);
        }
    }

    /// Tab 5: Raw XML Viewer.
    pub fn render_raw_xml_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(ref a) = self.analysis else { return };
        let xml_len = a.xml.len();
        let xml_chars = a.xml.chars().count();
        let matches = if !self.xml_search_query.is_empty() {
            a.xml.matches(&self.xml_search_query).count()
        } else {
            0
        };

        let mut do_copy = false;

        ui.horizontal(|ui| {
            ui.heading("Raw Packet Tracer XML");
            ui.separator();

            // Search query within XML
            ui.add(
                egui::TextEdit::singleline(&mut self.xml_search_query)
                    .hint_text("Find in XML..."),
            );

            if !self.xml_search_query.is_empty() {
                ui.label(format!("{matches} matches"));
                if ui.small_button("✕").clicked() {
                    self.xml_search_query.clear();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📋 Copy XML").clicked() {
                    do_copy = true;
                }
                ui.label(format!("Size: {:.2} KB ({} chars)", xml_len as f64 / 1024.0, xml_chars));
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut a.xml.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(26),
            );
        });

        if do_copy {
            self.copy_text_with_feedback(ctx, a.xml.clone(), "raw XML");
        }
    }

    /// Tab 6: Diagnostics & Parser Logs.
    pub fn render_diagnostics_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(ref a) = self.analysis else { return };
        let diagnostics = a.diagnostics.clone();
        let mut do_copy = false;

        ui.horizontal(|ui| {
            ui.heading(format!("Diagnostics ({} items)", diagnostics.len()));
            ui.separator();
            if ui.button("📋 Copy Diagnostics").clicked() {
                do_copy = true;
            }
        });

        ui.add_space(8.0);

        if diagnostics.is_empty() {
            ui.label("No warnings or diagnostic issues recorded.");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, diag) in diagnostics.iter().enumerate() {
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(255, 179, 0), "⚠");
                                ui.strong(format!("#{}:", idx + 1));
                                ui.label(diag);
                            });
                        });
                    ui.add_space(4.0);
                }
            });
        }

        if do_copy {
            let text = diagnostics.join("\n");
            self.copy_text_with_feedback(ctx, text, "diagnostics log");
        }
    }
}

impl eframe::App for InspectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain incoming background thread messages
        self.drain_messages();

        // Process global keyboard shortcuts
        self.handle_shortcuts(ctx);

        // Render UI panels
        self.render_top_panel(ctx);
        self.render_side_panel(ctx);
        self.render_bottom_panel(ctx);
        self.render_central_panel(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(prefs_str) = serde_json::to_string(&self.preferences) {
            storage.set_string("preferences", prefs_str);
        }
        if let Ok(checklists_str) = serde_json::to_string(&self.manual_checklists) {
            storage.set_string("manual_checklists", checklists_str);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_analysis() -> Analysis {
        Analysis {
            checkpoints: vec![
                Checkpoint {
                    id: "chk1".into(),
                    device: "Router0".into(),
                    device_type: "Router".into(),
                    interface: "GigabitEthernet0/0".into(),
                    category: "Addressing".into(),
                    property: "IP Address".into(),
                    expected: "192.168.1.1 255.255.255.0".into(),
                    current: Some("192.168.1.1".into()),
                    weight: 10.0,
                    path: "network/r0/g00".into(),
                    status: Status::Passed,
                    explanation: "IP configured correctly".into(),
                    evidence: vec!["assessment:passed".into()],
                },
                Checkpoint {
                    id: "chk2".into(),
                    device: "Switch0".into(),
                    device_type: "Switch".into(),
                    interface: "FastEthernet0/1".into(),
                    category: "VLAN".into(),
                    property: "Switchport Mode".into(),
                    expected: "trunk".into(),
                    current: Some("access".into()),
                    weight: 15.0,
                    path: "network/sw0/fa01".into(),
                    status: Status::Failed,
                    explanation: "Port is access instead of trunk".into(),
                    evidence: vec![],
                },
                Checkpoint {
                    id: "chk3".into(),
                    device: "PC0".into(),
                    device_type: "PC".into(),
                    interface: "FastEthernet0".into(),
                    category: "Addressing".into(),
                    property: "Default Gateway".into(),
                    expected: "192.168.1.1".into(),
                    current: None,
                    weight: 5.0,
                    path: "network/pc0/fa0".into(),
                    status: Status::Unverified,
                    explanation: "No observations".into(),
                    evidence: vec![],
                },
            ],
            diagnostics: vec!["Test diagnostic message".into()],
            xml: "<PACKETTRACER5><NETWORK/></PACKETTRACER5>".into(),
            timings: vec![("load".into(), 0.05), ("parse".into(), 0.02)],
        }
    }

    #[test]
    fn test_initial_app_state() {
        let app = InspectorApp::new(None, None);
        assert!(app.analysis.is_none());
        assert_eq!(app.current_tab, Tab::Overview);
        assert_eq!(app.status_filter, StatusFilter::All);
        assert!(!app.is_loading);
        assert_eq!(app.activity_key(), "default");
    }

    #[test]
    fn test_tab_metadata_and_shortcuts() {
        for tab in Tab::ALL {
            assert!(!tab.title().is_empty());
            assert!(tab.shortcut().starts_with("Ctrl+"));
        }
    }

    #[test]
    fn test_filtering_logic() {
        let mut app = InspectorApp::new(None, None);
        app.analysis = Some(make_test_analysis());

        // Default: all 3 match
        assert_eq!(app.filtered_checkpoint_indices().len(), 3);

        // Status filter: Passed
        app.status_filter = StatusFilter::Passed;
        assert_eq!(app.filtered_checkpoint_indices().len(), 1);
        assert_eq!(app.filtered_checkpoint_indices()[0], 0);

        // Status filter: Failed
        app.status_filter = StatusFilter::Failed;
        assert_eq!(app.filtered_checkpoint_indices().len(), 1);
        assert_eq!(app.filtered_checkpoint_indices()[0], 1);

        // Status filter: Unverified
        app.status_filter = StatusFilter::Unverified;
        assert_eq!(app.filtered_checkpoint_indices().len(), 1);
        assert_eq!(app.filtered_checkpoint_indices()[0], 2);

        // Reset status filter, test device filter
        app.status_filter = StatusFilter::All;
        app.selected_device = Some("Switch0".into());
        assert_eq!(app.filtered_checkpoint_indices().len(), 1);
        assert_eq!(app.filtered_checkpoint_indices()[0], 1);

        // Reset device, test search query
        app.selected_device = None;
        app.search_query = "trunk".into();
        assert_eq!(app.filtered_checkpoint_indices().len(), 1);
        assert_eq!(app.filtered_checkpoint_indices()[0], 1);

        app.search_query = "nonexistent".into();
        assert_eq!(app.filtered_checkpoint_indices().len(), 0);
    }

    #[test]
    fn test_manual_checklist_keyed_by_activity() {
        let mut app = InspectorApp::new(None, None);
        app.analysis = Some(make_test_analysis());
        app.activity_path = Some(PathBuf::from("C:/labs/lab1.pka"));

        assert_eq!(app.activity_key(), "lab1.pka");
        assert_eq!(app.manual_check_count(), (0, 3));

        // Mark chk1 as checked
        app.set_manual_check("chk1", true);
        assert!(app.is_manually_checked("chk1"));
        assert!(!app.is_manually_checked("chk2"));
        assert_eq!(app.manual_check_count(), (1, 3));

        // Switch activity to lab2.pka
        app.activity_path = Some(PathBuf::from("C:/labs/lab2.pka"));
        assert_eq!(app.activity_key(), "lab2.pka");
        // In lab2, chk1 should NOT be checked (isolated checklists!)
        assert!(!app.is_manually_checked("chk1"));
        assert_eq!(app.manual_check_count(), (0, 3));

        // Switch back to lab1.pka
        app.activity_path = Some(PathBuf::from("C:/labs/lab1.pka"));
        assert_eq!(app.activity_key(), "lab1.pka");
        assert!(app.is_manually_checked("chk1"));
        assert_eq!(app.manual_check_count(), (1, 3));
    }

    #[test]
    fn test_preferences_serialization() {
        let mut prefs = Preferences::default();
        prefs.active_tab = Tab::NextActions;
        prefs.compact_view = true;
        prefs.last_activity_path = Some("C:/test.pka".into());

        let serialized = serde_json::to_string(&prefs).expect("Serialize preferences");
        let deserialized: Preferences =
            serde_json::from_str(&serialized).expect("Deserialize preferences");

        assert_eq!(deserialized.active_tab, Tab::NextActions);
        assert!(deserialized.compact_view);
        assert_eq!(deserialized.last_activity_path, Some("C:/test.pka".into()));
    }

    #[test]
    fn test_summary_and_reports_integration() {
        let analysis = make_test_analysis();
        let summary = analysis.summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.unverified, 1);
        assert_eq!(summary.total_score, 30.0);
        assert_eq!(summary.verified_score, 10.0);

        let csv = reports::csv(&analysis).expect("CSV export");
        assert!(csv.contains("chk1"));
        assert!(csv.contains("Router0"));

        let md = reports::markdown(&analysis);
        assert!(md.contains("# PKA Lab Inspector report"));
        assert!(md.contains("Router0"));

        let cli = commands::device_script(&analysis.checkpoints, None);
        assert!(!cli.is_empty());
    }

    #[test]
    fn test_clear_progress_reverts_analysis() {
        let mut app = InspectorApp::new(None, None);
        let orig = make_test_analysis();
        app.analysis = Some(orig.clone());
        app.original_analysis = Some(orig);

        // Mutate current analysis as if compared
        if let Some(ref mut a) = app.analysis {
            a.checkpoints[0].status = Status::Failed;
        }
        assert_eq!(app.analysis.as_ref().unwrap().checkpoints[0].status, Status::Failed);

        app.clear_progress();
        assert_eq!(app.analysis.as_ref().unwrap().checkpoints[0].status, Status::Passed);
        assert!(app.progress_analysis.is_none());
        assert!(app.progress_path.is_none());
    }

    #[test]
    fn test_drain_messages_activity_loaded() {
        let mut app = InspectorApp::new(None, None);
        let path = PathBuf::from("lab.pka");
        let analysis = make_test_analysis();

        app.tx
            .send(LoadMessage::ActivityLoaded {
                path: path.clone(),
                result: Ok(analysis),
            })
            .unwrap();

        app.drain_messages();
        assert!(app.analysis.is_some());
        assert_eq!(app.activity_path, Some(path));
        assert_eq!(app.analysis.as_ref().unwrap().checkpoints.len(), 3);
        assert!(app.status_message.as_ref().unwrap().contains("Successfully loaded"));
    }

    #[test]
    fn test_unique_devices_and_categories() {
        let mut app = InspectorApp::new(None, None);
        app.analysis = Some(make_test_analysis());

        let devices = app.unique_devices();
        assert_eq!(devices, vec!["PC0", "Router0", "Switch0"]);

        let categories = app.unique_categories();
        assert_eq!(categories, vec!["Addressing", "VLAN"]);
    }

    #[test]
    fn test_manual_check_filter() {
        let mut app = InspectorApp::new(None, None);
        app.analysis = Some(make_test_analysis());
        app.activity_path = Some(PathBuf::from("lab1.pka"));

        // All 3 initially
        assert_eq!(app.filtered_checkpoint_indices().len(), 3);

        // Mark chk1 as checked
        app.set_manual_check("chk1", true);

        // Turn on "Show only unchecked"
        app.preferences.show_only_manual_unchecked = true;
        let indices = app.filtered_checkpoint_indices();
        assert_eq!(indices.len(), 2);
        assert_eq!(indices, vec![1, 2]); // chk2 and chk3 remain
    }

    #[test]
    fn test_command_generation_preview() {
        let analysis = make_test_analysis();
        let cmd0 = commands::generate(&analysis.checkpoints[0]);
        // Router0 IP Address
        assert!(cmd0.supported);
        assert!(!cmd0.gui);
        assert!(cmd0.text.contains("ip address 192.168.1.1 255.255.255.0"));

        let cmd1 = commands::generate(&analysis.checkpoints[1]);
        // Switch0 switchport mode trunk
        assert!(cmd1.supported);
        assert!(!cmd1.gui);
        assert!(cmd1.text.contains("switchport mode trunk"));
    }
}
