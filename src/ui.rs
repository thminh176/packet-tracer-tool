use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{Receiver, Sender, channel},
    },
    time::{Duration, Instant},
};

use eframe::egui;
use serde::{Deserialize, Serialize};

use pka_lab_inspector::cisco::{
    self, Capability, CommandRequest, CommandSpec, DeviceProfile, SupportState,
};
use pka_lab_inspector::commands;
use pka_lab_inspector::inspector;
use pka_lab_inspector::localization::Localizer;
use pka_lab_inspector::model::{Analysis, Checkpoint, Status};
use pka_lab_inspector::network_values;
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
    CommandLibrary,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Overview,
        Tab::Checkpoints,
        Tab::NextActions,
        Tab::DeviceCommands,
        Tab::RawXml,
        Tab::Diagnostics,
        Tab::CommandLibrary,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Overview => "nav.overview",
            Tab::Checkpoints => "nav.checkpoints",
            Tab::NextActions => "nav.next_actions",
            Tab::DeviceCommands => "nav.device_commands",
            Tab::RawXml => "nav.raw_xml",
            Tab::Diagnostics => "nav.diagnostics",
            Tab::CommandLibrary => "nav.command_library",
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
            Tab::CommandLibrary => "Ctrl+7",
        }
    }
}

/// Filter device categories in the Cisco Command Library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CiscoDeviceFilter {
    #[default]
    All,
    Router,
    Switch,
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
#[serde(default)]
pub struct Preferences {
    pub language: String,
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
            language: "vi".into(),
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

/// Results carry a generation so canceled or superseded jobs cannot replace current state.
pub enum LoadMessage {
    ActivityLoaded {
        token: u64,
        path: PathBuf,
        result: Result<Analysis, String>,
    },
    ProgressLoaded {
        token: u64,
        path: PathBuf,
        result: Result<(Analysis, Option<Analysis>), String>,
    },
}

/// Recognized network protocols that receive prominent visual badges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolBadge {
    Ipv4StaticRoute,
    Ipv6StaticRoute,
    OspfNetwork,
}

impl ProtocolBadge {
    pub fn badge_text(&self) -> &'static str {
        match self {
            Self::Ipv4StaticRoute => "[IPv4 Static Route]",
            Self::Ipv6StaticRoute => "[IPv6 Static Route]",
            Self::OspfNetwork => "[OSPF Network]",
        }
    }

    pub fn catalog_key(&self) -> &'static str {
        match self {
            Self::Ipv4StaticRoute => "detail.badge_ipv4_static",
            Self::Ipv6StaticRoute => "detail.badge_ipv6_static",
            Self::OspfNetwork => "detail.badge_ospf_network",
        }
    }
}

/// Detect whether a checkpoint represents an IPv4 static route, IPv6 static route, or OSPF network.
pub fn detect_protocol(c: &Checkpoint) -> Option<ProtocolBadge> {
    if commands::is_ospf_network_property(&c.property, &c.path) {
        return Some(ProtocolBadge::OspfNetwork);
    }
    if commands::is_ipv6_static_route_property(&c.property, &c.path) {
        return Some(ProtocolBadge::Ipv6StaticRoute);
    }

    if commands::is_static_route_property(&c.property, &c.path) {
        return Some(ProtocolBadge::Ipv4StaticRoute);
    }

    None
}

/// Carry explicit checkpoint parameters into the library without filling unknowns.
fn checkpoint_library_request(c: &Checkpoint) -> Option<cisco::CommandRequest> {
    if cisco::mapping::identity_conflict(c)
        || cisco::mapping::parameter_conflict(c, &c.expected)
        || c.evidence.iter().any(|e| {
            e.starts_with("assessment:unsupported")
                || e.starts_with("assessment:ambiguous")
                || e.starts_with("assessment:inventory")
                || e.starts_with("commands:context-conflict=")
        })
        || commands::generate(c)
            .explanation
            .to_ascii_lowercase()
            .contains("conflict")
    {
        return None;
    }
    if !commands::is_ospf_network_property(&c.property, &c.path) {
        let request = cisco::mapping::request(c, &c.expected)?;
        let plan = cisco::generate(&request);
        return (matches!(
            request.family.as_str(),
            "router.interface.dot1q"
                | "router.serial.clock"
                | "router.serial.ppp"
                | "basic.cdp_interface"
                | "router.ospf.router_id"
                | "router.ospf.passive"
                | "router.ospf.reference_bandwidth"
                | "router.eigrp.router_id"
                | "router.eigrp.passive"
                | "basic.ntp_source"
                | "basic.ntp_server"
                | "switch.lldp.interface"
                | "switch.lldp.timers"
        ) && !plan.missing_parameters.is_empty())
        .then_some(request);
    }
    let o = network_values::ParsedOspfNetwork::parse(&c.expected).ok()?;
    let prefix = network_values::wildcard_to_prefix(o.wildcard_mask)?;
    let mut parameters = BTreeMap::from([("network".into(), format!("{}/{prefix}", o.network))]);
    if let Some(area) = o.area {
        parameters.insert("area".into(), area);
    }
    if let Some(process) = o.process_id {
        parameters.insert("process_id".into(), process.to_string());
    }
    Some(cisco::CommandRequest {
        family: "router.ospf.network".into(),
        device: c.device.clone(),
        profile: cisco::mapping::profile(c),
        parameters,
    })
}

/// Structured network values extracted from a static route or OSPF statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDetail {
    pub badge: ProtocolBadge,
    pub raw_value: String,
    pub normalized_cidr: String,
    pub subnet_mask: Option<String>,
    pub wildcard_mask: Option<String>,
    pub next_hop: Option<String>,
    pub exit_interface: Option<String>,
    pub distance: Option<u8>,
    pub area: Option<String>,
    pub process_id: Option<u32>,
}

/// Extract detailed network parameters (raw, CIDR, subnet mask, wildcard) from a checkpoint.
pub fn extract_network_detail(c: &Checkpoint) -> Option<NetworkDetail> {
    let badge = detect_protocol(c)?;
    match badge {
        ProtocolBadge::Ipv4StaticRoute => {
            if let Ok(route) = network_values::ParsedStaticRoute::parse(&c.expected) {
                Some(NetworkDetail {
                    badge,
                    raw_value: c.expected.clone(),
                    normalized_cidr: route.destination.cidr(),
                    subnet_mask: Some(route.destination.subnet_mask.to_string()),
                    wildcard_mask: Some(route.destination.wildcard_mask.to_string()),
                    next_hop: route.next_hop.map(|ip| ip.to_string()),
                    exit_interface: route.exit_interface,
                    distance: route.distance,
                    area: None,
                    process_id: None,
                })
            } else if let Some(net) = network_values::NormalizedNetwork::parse(&c.expected) {
                Some(NetworkDetail {
                    badge,
                    raw_value: c.expected.clone(),
                    normalized_cidr: net.cidr(),
                    subnet_mask: Some(net.subnet_mask.to_string()),
                    wildcard_mask: Some(net.wildcard_mask.to_string()),
                    next_hop: None,
                    exit_interface: None,
                    distance: None,
                    area: None,
                    process_id: None,
                })
            } else {
                Some(NetworkDetail {
                    badge,
                    raw_value: c.expected.clone(),
                    normalized_cidr: c.expected.clone(),
                    subnet_mask: None,
                    wildcard_mask: None,
                    next_hop: None,
                    exit_interface: None,
                    distance: None,
                    area: None,
                    process_id: None,
                })
            }
        }
        ProtocolBadge::Ipv6StaticRoute => {
            if let Some(route) = network_values::ParsedIpv6StaticRoute::parse(&c.expected) {
                Some(NetworkDetail {
                    badge,
                    raw_value: c.expected.clone(),
                    normalized_cidr: format!("{}/{}", route.destination, route.prefix),
                    subnet_mask: Some(format!("/{}", route.prefix)),
                    wildcard_mask: None,
                    next_hop: route.next_hop.map(|ip| ip.to_string()),
                    exit_interface: route.exit_interface,
                    distance: route.distance,
                    area: None,
                    process_id: None,
                })
            } else {
                Some(NetworkDetail {
                    badge,
                    raw_value: c.expected.clone(),
                    normalized_cidr: c.expected.clone(),
                    subnet_mask: None,
                    wildcard_mask: None,
                    next_hop: None,
                    exit_interface: None,
                    distance: None,
                    area: None,
                    process_id: None,
                })
            }
        }
        ProtocolBadge::OspfNetwork => {
            if let Ok(ospf) = network_values::ParsedOspfNetwork::parse(&c.expected) {
                let prefix = network_values::wildcard_to_prefix(ospf.wildcard_mask);
                let cidr = prefix
                    .map(|p| format!("{}/{}", ospf.network, p))
                    .unwrap_or_else(|| ospf.network.to_string());
                let subnet =
                    network_values::wildcard_to_subnet(ospf.wildcard_mask).map(|m| m.to_string());
                Some(NetworkDetail {
                    badge,
                    raw_value: c.expected.clone(),
                    normalized_cidr: cidr,
                    subnet_mask: subnet,
                    wildcard_mask: Some(ospf.wildcard_mask.to_string()),
                    next_hop: None,
                    exit_interface: None,
                    distance: None,
                    area: ospf.area,
                    process_id: ospf.process_id,
                })
            } else if let Some(net) = network_values::NormalizedNetwork::parse(&c.expected) {
                Some(NetworkDetail {
                    badge,
                    raw_value: c.expected.clone(),
                    normalized_cidr: net.cidr(),
                    subnet_mask: Some(net.subnet_mask.to_string()),
                    wildcard_mask: Some(net.wildcard_mask.to_string()),
                    next_hop: None,
                    exit_interface: None,
                    distance: None,
                    area: None,
                    process_id: None,
                })
            } else {
                Some(NetworkDetail {
                    badge,
                    raw_value: c.expected.clone(),
                    normalized_cidr: c.expected.clone(),
                    subnet_mask: None,
                    wildcard_mask: None,
                    next_hop: None,
                    exit_interface: None,
                    distance: None,
                    area: None,
                    process_id: None,
                })
            }
        }
    }
}

/// Render a styled protocol badge pill widget.
pub fn render_protocol_badge(ui: &mut egui::Ui, badge: ProtocolBadge, text: &str) {
    let (bg_color, fg_color) = match badge {
        ProtocolBadge::Ipv4StaticRoute => (
            egui::Color32::from_rgb(27, 94, 32),
            egui::Color32::from_rgb(200, 230, 201),
        ),
        ProtocolBadge::Ipv6StaticRoute => (
            egui::Color32::from_rgb(13, 71, 161),
            egui::Color32::from_rgb(187, 222, 251),
        ),
        ProtocolBadge::OspfNetwork => (
            egui::Color32::from_rgb(230, 81, 0),
            egui::Color32::from_rgb(255, 224, 178),
        ),
    };

    egui::Frame::NONE
        .fill(bg_color)
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(fg_color).strong());
        });
}

/// Render a styled badge representing a Cisco device profile support state.
/// CRITICAL: Never labels PT verified unless support is actual `SupportState::VerifiedPacketTracer`.
pub fn render_cisco_support_badge(
    ui: &mut egui::Ui,
    support: SupportState,
    catalog: &Localizer,
    language: &str,
) {
    let tr = |key: &str| catalog.text(language, key);
    let (bg, fg, label_key) = match support {
        SupportState::VerifiedPacketTracer => (
            egui::Color32::from_rgb(27, 94, 32),
            egui::Color32::from_rgb(200, 230, 201),
            "cisco.support.verified_pt",
        ),
        SupportState::DocumentedNotTested => (
            egui::Color32::from_rgb(13, 71, 161),
            egui::Color32::from_rgb(187, 222, 251),
            "cisco.support.documented_not_tested",
        ),
        SupportState::Unsupported => (
            egui::Color32::from_rgb(183, 28, 28),
            egui::Color32::from_rgb(255, 205, 210),
            "cisco.support.unsupported",
        ),
        SupportState::Unknown => (
            egui::Color32::from_rgb(80, 80, 80),
            egui::Color32::from_rgb(238, 238, 238),
            "cisco.support.unknown",
        ),
    };

    egui::Frame::NONE
        .fill(bg)
        .corner_radius(egui::CornerRadius::same(3))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(tr(label_key))
                    .color(fg)
                    .size(11.0)
                    .strong(),
            );
        });
}

/// Main Inspector Application State.
pub struct InspectorApp {
    pub localizer: Arc<Localizer>,
    request_token: u64,
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
    xml_chunks: Vec<(usize, usize)>,
    xml_cache_identity: (usize, usize),

    // Status / feedback
    pub is_loading: bool,
    pub loading_description: Option<String>,
    pub status_message: Option<String>,
    pub status_timestamp: Option<Instant>,
    pub copy_notification: Option<(String, Instant)>,
    pub show_settings: bool,

    // Preferences and checklists
    pub preferences: Preferences,
    pub manual_checklists: HashMap<String, ActivityChecklist>,

    // Cisco Command Library state
    pub library_search_query: String,
    pub library_device_filter: CiscoDeviceFilter,
    pub library_topic_filter: Option<String>,
    pub library_profile: DeviceProfile,
    pub library_selected_id: Option<String>,
    pub library_target_device: String,
    pub library_parameters: BTreeMap<String, String>,

    // Cisco Command Library layout diagnostics
    pub library_layout_height: f32,
    pub library_left_width: f32,
    pub library_right_width: f32,

    // Background thread channels
    pub tx: Sender<LoadMessage>,
    pub rx: Receiver<LoadMessage>,
}

impl InspectorApp {
    fn select_library_family(&mut self, id: &str) {
        if self.library_selected_id.as_deref() != Some(id) {
            self.library_parameters.clear();
            self.library_selected_id = Some(id.to_owned());
        }
    }
    pub fn new(cc: Option<&eframe::CreationContext>, initial_path: Option<PathBuf>) -> Self {
        let (tx, rx) = channel();

        let mut app = Self {
            localizer: Arc::new(Localizer::new()),
            request_token: 0,
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
            xml_chunks: Vec::new(),
            xml_cache_identity: (0, 0),

            is_loading: false,
            loading_description: None,
            status_message: None,
            status_timestamp: Some(Instant::now()),
            copy_notification: None,
            show_settings: false,

            preferences: Preferences::default(),
            manual_checklists: HashMap::new(),

            library_search_query: String::new(),
            library_device_filter: CiscoDeviceFilter::All,
            library_topic_filter: None,
            library_profile: DeviceProfile::Unknown,
            library_selected_id: None,
            library_target_device: "Router0".into(),
            library_parameters: BTreeMap::new(),
            library_layout_height: 0.0,
            library_left_width: 0.0,
            library_right_width: 0.0,

            tx,
            rx,
        };

        // Load persisted state if available
        if let Some(ctx_ref) = cc {
            if let Some(storage) = ctx_ref.storage {
                if let Some(prefs_str) = storage.get_string("preferences")
                    && let Ok(prefs) = serde_json::from_str::<Preferences>(&prefs_str)
                {
                    app.preferences = prefs;
                    app.current_tab = app.preferences.active_tab;
                }
                if let Some(checklists_str) = storage.get_string("manual_checklists")
                    && let Ok(checklists) =
                        serde_json::from_str::<HashMap<String, ActivityChecklist>>(&checklists_str)
                {
                    app.manual_checklists = checklists;
                }
            }

            // Configure native dark visuals
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = egui::CornerRadius::same(6);
            ctx_ref.egui_ctx.set_visuals(visuals);
            // Use the Windows system font for complete Vietnamese glyphs.
            // It is read from the OS, never copied into the distribution.
            if let Some(windows) = std::env::var_os("WINDIR")
                && let Ok(bytes) = std::fs::read(PathBuf::from(windows).join("Fonts/segoeui.ttf"))
            {
                let mut fonts = egui::FontDefinitions::default();
                fonts.font_data.insert(
                    "system-ui".into(),
                    Arc::new(egui::FontData::from_owned(bytes)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "system-ui".into());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("system-ui".into());
                ctx_ref.egui_ctx.set_fonts(fonts);
            }
        }

        app.set_status(
            app.localizer
                .text(&app.preferences.language, "status.ready"),
        );

        // If an initial file was specified, load it
        if let Some(path) = initial_path {
            app.load_activity_background(path, cc.map(|c| c.egui_ctx.clone()));
        }

        app
    }

    /// Key used to isolate manual checklist data per activity.
    pub fn activity_key(&self) -> String {
        if let Some(ref path) = self.activity_path {
            canonical_identity(path).to_string_lossy().into_owned()
        } else {
            "default".to_string()
        }
    }

    /// Get current activity's manual checklist.
    pub fn current_checklist(&self) -> &ActivityChecklist {
        static DEFAULT_CHECKLIST: std::sync::LazyLock<ActivityChecklist> =
            std::sync::LazyLock::new(ActivityChecklist::default);
        let key = self.activity_key();
        self.manual_checklists
            .get(&key)
            .unwrap_or(&DEFAULT_CHECKLIST)
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
            checklist
                .checked_checkpoints
                .insert(checkpoint_id.to_string(), true);
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
        let total = self
            .analysis
            .as_ref()
            .map(|a| a.checkpoints.len())
            .unwrap_or(0);
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

    /// Only one load/comparison job runs at a time; clear invalidates its generation.
    pub fn load_activity_background(&mut self, path: PathBuf, ctx: Option<egui::Context>) {
        if self.is_loading {
            return;
        }
        self.request_token = self.request_token.wrapping_add(1);
        let token = self.request_token;
        self.is_loading = true;
        self.loading_description = Some(
            self.localizer
                .text(&self.preferences.language, "status.loading"),
        );
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = inspector::load(&path).map_err(|e| format!("{e:#}"));
            let _ = tx.send(LoadMessage::ActivityLoaded {
                token,
                path,
                result,
            });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    pub fn load_progress_background(&mut self, path: PathBuf, ctx: Option<egui::Context>) {
        if self.is_loading || self.original_analysis.is_none() {
            return;
        }
        self.request_token = self.request_token.wrapping_add(1);
        let token = self.request_token;
        self.is_loading = true;
        self.loading_description = Some(
            self.localizer
                .text(&self.preferences.language, "status.loading"),
        );
        let mut activity = self.original_analysis.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = inspector::load(&path)
                .map(|progress| {
                    if let Some(ref mut activity) = activity {
                        inspector::compare(activity, &progress);
                    }
                    (progress, activity)
                })
                .map_err(|e| format!("{e:#}"));
            let _ = tx.send(LoadMessage::ProgressLoaded {
                token,
                path,
                result,
            });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    pub fn reload_activity(&mut self, ctx: Option<egui::Context>) {
        if let Some(path) = self.activity_path.clone() {
            self.load_activity_background(path, ctx);
        }
    }

    pub fn drain_messages(&mut self) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        while let Ok(msg) = self.rx.try_recv() {
            let token = match &msg {
                LoadMessage::ActivityLoaded { token, .. }
                | LoadMessage::ProgressLoaded { token, .. } => *token,
            };
            if token != self.request_token {
                continue;
            }
            self.is_loading = false;
            self.loading_description = None;
            match msg {
                LoadMessage::ActivityLoaded { path, result, .. } => match result {
                    Ok(analysis) => {
                        let count = analysis.checkpoints.len();
                        self.activity_path = Some(path.clone());
                        self.preferences.last_activity_path =
                            Some(path.to_string_lossy().into_owned());
                        self.original_analysis = Some(analysis.clone());
                        self.analysis = Some(analysis);
                        self.progress_analysis = None;
                        self.progress_path = None;
                        self.preferences.last_progress_path = None;
                        self.set_status(tf(
                            "ui.successfully_loaded_value_value2_checkpoints",
                            &[
                                ("value", &format!("{}", path.display())),
                                ("value2", &format!("{}", count)),
                            ],
                        ));
                    }
                    Err(error) => self.set_status(tf(
                        "ui.failed_to_load_activity_error",
                        &[("error", &error.to_string())],
                    )),
                },
                LoadMessage::ProgressLoaded { path, result, .. } => match result {
                    Ok((progress, compared)) => {
                        self.progress_path = Some(path.clone());
                        self.preferences.last_progress_path =
                            Some(path.to_string_lossy().into_owned());
                        self.progress_analysis = Some(progress);
                        if let Some(compared) = compared {
                            self.analysis = Some(compared);
                        }
                        self.set_status(
                            self.localizer
                                .text(&self.preferences.language, "status.progress_compared"),
                        );
                    }
                    Err(error) => self.set_status(tf(
                        "ui.failed_to_load_progress_error",
                        &[("error", &error.to_string())],
                    )),
                },
            }
        }
    }

    pub fn clear_progress(&mut self) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        self.request_token = self.request_token.wrapping_add(1);
        self.is_loading = false;
        self.loading_description = None;
        self.analysis = self.original_analysis.clone();
        self.progress_analysis = None;
        self.progress_path = None;
        self.preferences.last_progress_path = None;
        self.set_status(tr(
            "ui.progress_cleared_showing_original_activity_assessment",
        ));
    }

    /// Guard both loaded sources and Packet Tracer destinations before touching disk.
    fn export_data(&self, path: &Path, data: impl AsRef<[u8]>) -> std::io::Result<()> {
        let protected = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pka") || e.eq_ignore_ascii_case("pkt"))
            || [&self.activity_path, &self.progress_path]
                .into_iter()
                .flatten()
                .any(|source| canonical_identity(source) == canonical_identity(path));
        if protected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                self.localizer
                    .text(&self.preferences.language, "error.source_export"),
            ));
        }
        atomic_export(path, data.as_ref())
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
                if let Some(ref dev) = self.selected_device
                    && &c.device != dev
                {
                    return false;
                }

                // Category filter
                if let Some(ref cat) = self.selected_category
                    && &c.category != cat
                {
                    return false;
                }

                // Manual unchecked filter
                if manual_filter
                    && checklist
                        .checked_checkpoints
                        .get(&c.id)
                        .copied()
                        .unwrap_or(false)
                {
                    return false;
                }

                // Search query
                if !search.is_empty() {
                    let matches_id = c.id.to_lowercase().contains(&search);
                    let matches_dev = c.device.to_lowercase().contains(&search);
                    let matches_iface = c.interface.to_lowercase().contains(&search);
                    let matches_prop = c.property.to_lowercase().contains(&search);
                    let matches_exp = c.expected.to_lowercase().contains(&search);
                    let matches_curr = c
                        .current
                        .as_ref()
                        .is_some_and(|s| s.to_lowercase().contains(&search));
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

    /// Copy text to system clipboard via egui context with immediate visual feedback.
    pub fn copy_text_with_feedback(&mut self, ctx: &egui::Context, text: String, label: &str) {
        ctx.copy_text(text);
        let base_msg = self
            .localizer
            .text(&self.preferences.language, "action.copied");
        let full_msg = if !label.trim().is_empty() {
            format!("{base_msg}: {label}")
        } else {
            base_msg
        };
        self.copy_notification = Some((full_msg.clone(), Instant::now()));
        self.set_status(full_msg);
        ctx.request_repaint();
    }

    /// Render immediate toast notification overlay for user actions like Copy.
    pub fn render_notification(&self, ctx: &egui::Context) {
        if let Some((ref msg, ts)) = self.copy_notification {
            let elapsed = ts.elapsed();
            if elapsed < Duration::from_secs(3) {
                let opacity = if elapsed < Duration::from_millis(2200) {
                    1.0
                } else {
                    (1.0 - (elapsed.as_secs_f32() - 2.2) / 0.8).clamp(0.0, 1.0)
                };
                let alpha = (opacity * 255.0) as u8;

                egui::Area::new(egui::Id::new("copy_notification_toast"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -45.0))
                    .order(egui::Order::Foreground)
                    .interactable(false)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(egui::Color32::from_rgba_premultiplied(20, 55, 25, alpha))
                            .stroke(egui::Stroke::new(
                                1.5_f32,
                                egui::Color32::from_rgba_premultiplied(76, 175, 80, alpha),
                            ))
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(14, 8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("✔")
                                            .color(egui::Color32::from_rgba_premultiplied(
                                                129, 199, 132, alpha,
                                            ))
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(msg)
                                            .color(egui::Color32::from_rgba_premultiplied(
                                                255, 255, 255, alpha,
                                            ))
                                            .strong(),
                                    );
                                });
                            });
                    });
                ctx.request_repaint();
            }
        }
    }

    /// Handle keyboard shortcuts.
    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        ctx.input(|i| {
            let ctrl = i.modifiers.command; // Ctrl on Windows/Linux, Cmd on Mac
            let shift = i.modifiers.shift;

            // Ctrl+O: Open Activity
            if ctrl
                && !shift
                && i.key_pressed(egui::Key::O)
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr("ui.packet_tracer_pka_pkt"), &["pka", "pkt"])
                    .add_filter(tr("ui.all_files"), &["*"])
                    .pick_file()
            {
                self.load_activity_background(path, Some(ctx.clone()));
            }

            // Ctrl+P or Ctrl+Shift+O: Open Progress
            if ((ctrl && i.key_pressed(egui::Key::P))
                || (ctrl && shift && i.key_pressed(egui::Key::O)))
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr("ui.packet_tracer_pka_pkt"), &["pka", "pkt"])
                    .add_filter(tr("ui.all_files"), &["*"])
                    .pick_file()
            {
                self.load_progress_background(path, Some(ctx.clone()));
            }

            // Ctrl+R: Reload Activity
            if ctrl && i.key_pressed(egui::Key::R) {
                self.reload_activity(Some(ctx.clone()));
            }

            // Tabs Ctrl+1 through Ctrl+7
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
            if ctrl && i.key_pressed(egui::Key::Num7) {
                self.current_tab = Tab::CommandLibrary;
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
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        egui::TopBottomPanel::top("top_panel")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading(tr("app.title"));
                    ui.label(tr("menu.language"));
                    egui::ComboBox::from_id_salt("language_selector")
                        .selected_text(
                            self.localizer
                                .languages()
                                .into_iter()
                                .find(|(code, _)| {
                                    code == &self.preferences.language
                                        || (code == "vi-VN" && self.preferences.language == "vi")
                                        || (code == "en-US" && self.preferences.language == "en")
                                })
                                .map(|(_, name)| name)
                                .unwrap_or_else(|| self.preferences.language.clone()),
                        )
                        .show_ui(ui, |ui| {
                            for (code, name) in self.localizer.languages() {
                                let is_selected = code == self.preferences.language
                                    || (code == "vi-VN" && self.preferences.language == "vi")
                                    || (code == "en-US" && self.preferences.language == "en");
                                if ui.selectable_label(is_selected, &name).clicked() {
                                    self.preferences.language = code;
                                    self.status_message = None;
                                }
                            }
                        });
                    if ui.small_button(tr("menu.reload_languages")).clicked() {
                        self.localizer = Arc::new(Localizer::new());
                    }
                    if ui.button(tr("menu.settings")).clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    ui.separator();

                    // Open Activity button
                    if ui.button(tr("menu.open_file")).clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter(tr("ui.packet_tracer_pka_pkt"), &["pka", "pkt"])
                            .add_filter(tr("ui.all_files"), &["*"])
                            .pick_file()
                    {
                        self.load_activity_background(path, Some(ctx.clone()));
                    }

                    // Reload button
                    let reload_enabled = self.activity_path.is_some() && !self.is_loading;
                    if ui
                        .add_enabled(reload_enabled, egui::Button::new(tr("menu.reload")))
                        .clicked()
                    {
                        self.reload_activity(Some(ctx.clone()));
                    }

                    ui.separator();

                    // Open Progress button
                    let progress_enabled = self.analysis.is_some() && !self.is_loading;
                    if ui
                        .add_enabled(
                            progress_enabled,
                            egui::Button::new(tr("menu.open_progress")),
                        )
                        .clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter(tr("ui.packet_tracer_pka_pkt"), &["pka", "pkt"])
                            .add_filter(tr("ui.all_files"), &["*"])
                            .pick_file()
                    {
                        self.load_progress_background(path, Some(ctx.clone()));
                    }

                    // Clear progress button
                    if self.progress_analysis.is_some()
                        && ui.button(tr("ui.clear_progress")).clicked()
                    {
                        self.clear_progress();
                    }

                    ui.separator();

                    // Export CSV
                    let export_enabled = self.analysis.is_some();
                    if ui
                        .add_enabled(export_enabled, egui::Button::new(tr("menu.export_csv")))
                        .clicked()
                        && let Some(ref a) = self.analysis
                    {
                        match reports::csv(a) {
                            Ok(data) => {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(tr("ui.csv_csv"), &["csv"])
                                    .set_file_name("activity_report.csv")
                                    .save_file()
                                {
                                    if let Err(e) = self.export_data(&path, data) {
                                        self.set_status(tf(
                                            "ui.error_saving_csv_e",
                                            &[("e", &format!("{}", e))],
                                        ));
                                    } else {
                                        self.set_status(tf(
                                            "ui.saved_csv_to_value",
                                            &[("value", &format!("{}", path.display()))],
                                        ));
                                    }
                                }
                            }
                            Err(e) => self.set_status(tf(
                                "ui.failed_to_generate_csv_e",
                                &[("e", &format!("{}", e))],
                            )),
                        }
                    }

                    // Export Markdown
                    if ui
                        .add_enabled(
                            export_enabled,
                            egui::Button::new(tr("menu.export_markdown")),
                        )
                        .clicked()
                        && let Some(ref a) = self.analysis
                    {
                        let data = reports::markdown(a);
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(tr("ui.markdown_md"), &["md"])
                            .set_file_name("activity_report.md")
                            .save_file()
                        {
                            if let Err(e) = self.export_data(&path, data) {
                                self.set_status(tf(
                                    "ui.error_saving_markdown_e",
                                    &[("e", &format!("{}", e))],
                                ));
                            } else {
                                self.set_status(tf(
                                    "ui.saved_markdown_to_value",
                                    &[("value", &format!("{}", path.display()))],
                                ));
                            }
                        }
                    }

                    // Export CLI Script
                    if ui
                        .add_enabled(export_enabled, egui::Button::new(tr("menu.export_cli")))
                        .clicked()
                        && let Some(ref a) = self.analysis
                    {
                        let script = commands::device_script(&a.checkpoints, None);
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(tr("ui.cli_script_txt_ios"), &["txt", "ios", "cli"])
                            .set_file_name("device_commands.txt")
                            .save_file()
                        {
                            if let Err(e) = self.export_data(&path, script) {
                                self.set_status(tf(
                                    "ui.error_saving_cli_script_e",
                                    &[("e", &format!("{}", e))],
                                ));
                            } else {
                                self.set_status(tf(
                                    "ui.saved_cli_script_to_value",
                                    &[("value", &format!("{}", path.display()))],
                                ));
                            }
                        }
                    }
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    // Summary has its own row, including at narrow window sizes.
                    ui.horizontal_wrapped(|ui| {
                        if let Some(ref a) = self.analysis {
                            let s = a.summary();
                            ui.label(tf(
                                "ui.score_value_value2",
                                &[
                                    ("value", &format!("{:.1}", s.verified_score)),
                                    ("value2", &format!("{:.1}", s.total_score)),
                                ],
                            ));
                            ui.separator();

                            // Unverified badge
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 179, 0),
                                tf(
                                    "ui.value_unverified",
                                    &[("value", &format!("{}", s.unverified))],
                                ),
                            );

                            // Failed badge
                            ui.colored_label(
                                egui::Color32::from_rgb(244, 67, 54),
                                tf("ui.value_failed", &[("value", &format!("{}", s.failed))]),
                            );

                            // Passed badge
                            ui.colored_label(
                                egui::Color32::from_rgb(76, 175, 80),
                                tf("ui.value_passed", &[("value", &format!("{}", s.passed))]),
                            );

                            ui.label(tf("ui.total_value", &[("value", &format!("{}", s.total))]));
                        } else {
                            ui.label(tr("status.no_file"));
                        }
                    });
                });
            });
    }

    /// Render Left Panel (Filters, Navigation, Activity Checklist).
    pub fn render_side_panel(&mut self, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        egui::SidePanel::left("side_panel")
            .resizable(true)
            .default_width(280.0)
            .width_range(220.0..=450.0)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::same(12)))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.strong(tr("ui.search_filters"));
                    ui.add_space(4.0);

                    // Search box
                    ui.horizontal(|ui| {
                        let _search_edit = ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text(tr("filter.search")),
                        );
                        if !self.search_query.is_empty() && ui.small_button("✕").clicked() {
                            self.search_query.clear();
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Status filter
                    ui.strong(tr("ui.status_filter"));
                    let total_passed = self
                        .analysis
                        .as_ref()
                        .map(|a| a.summary().passed)
                        .unwrap_or(0);
                    let total_failed = self
                        .analysis
                        .as_ref()
                        .map(|a| a.summary().failed)
                        .unwrap_or(0);
                    let total_unverified = self
                        .analysis
                        .as_ref()
                        .map(|a| a.summary().unverified)
                        .unwrap_or(0);
                    let total_all = self
                        .analysis
                        .as_ref()
                        .map(|a| a.checkpoints.len())
                        .unwrap_or(0);

                    ui.radio_value(
                        &mut self.status_filter,
                        StatusFilter::All,
                        tf(
                            "ui.all_total_all",
                            &[("total_all", &format!("{}", total_all))],
                        ),
                    );
                    ui.radio_value(
                        &mut self.status_filter,
                        StatusFilter::Passed,
                        tf(
                            "ui.passed_total_passed",
                            &[("total_passed", &format!("{}", total_passed))],
                        ),
                    );
                    ui.radio_value(
                        &mut self.status_filter,
                        StatusFilter::Failed,
                        tf(
                            "ui.failed_total_failed",
                            &[("total_failed", &format!("{}", total_failed))],
                        ),
                    );
                    ui.radio_value(
                        &mut self.status_filter,
                        StatusFilter::Unverified,
                        tf(
                            "ui.unverified_total_unverified",
                            &[("total_unverified", &format!("{}", total_unverified))],
                        ),
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Device Filter
                    ui.strong(tr("ui.device_filter"));
                    let devices = self.unique_devices();
                    let current_device_label = self
                        .selected_device
                        .clone()
                        .unwrap_or_else(|| tr("filter.all_devices"));

                    egui::ComboBox::from_id_salt("device_filter_combo")
                        .selected_text(current_device_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    self.selected_device.is_none(),
                                    tr("filter.all_devices"),
                                )
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
                    ui.strong(tr("ui.category_filter"));
                    let categories = self.unique_categories();
                    let current_category_label = self
                        .selected_category
                        .clone()
                        .unwrap_or_else(|| tr("filter.all_categories"));

                    egui::ComboBox::from_id_salt("category_filter_combo")
                        .selected_text(current_category_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    self.selected_category.is_none(),
                                    tr("ui.all_categories"),
                                )
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
                    ui.strong(tr("ui.activity_checklist"));
                    let (completed_checks, total_checks) = self.manual_check_count();
                    ui.label(tf(
                        "ui.completed_checks_total_checks_completed",
                        &[
                            ("completed_checks", &format!("{}", completed_checks)),
                            ("total_checks", &format!("{}", total_checks)),
                        ],
                    ));

                    ui.checkbox(
                        &mut self.preferences.show_only_manual_unchecked,
                        tr("ui.show_only_unchecked"),
                    );

                    if ui
                        .small_button(tr("ui.clear_checklist_for_activity"))
                        .clicked()
                    {
                        let key = self.activity_key();
                        self.manual_checklists.remove(&key);
                        self.set_status(tr("ui.activity_checklist_reset"));
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Preferences
                    ui.strong(tr("ui.view_options"));
                    ui.checkbox(&mut self.preferences.compact_view, tr("ui.compact_rows"));

                    ui.add_space(12.0);
                    // Active files info
                    ui.separator();
                    ui.add_space(4.0);
                    ui.strong(tr("ui.loaded_files"));
                    if let Some(ref path) = self.activity_path {
                        ui.label(tf(
                            "ui.activity_value",
                            &[(
                                "value",
                                &format!(
                                    "{}",
                                    path.file_name().unwrap_or_default().to_string_lossy()
                                ),
                            )],
                        ));
                    } else {
                        ui.label(tr("ui.activity_none"));
                    }

                    if let Some(ref path) = self.progress_path {
                        ui.add_space(4.0);
                        ui.label(tf(
                            "ui.progress_value",
                            &[(
                                "value",
                                &format!(
                                    "{}",
                                    path.file_name().unwrap_or_default().to_string_lossy()
                                ),
                            )],
                        ));
                    }
                });
            });
    }

    /// Render Status Bar (Bottom Panel).
    pub fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        let filtered_count = self.filtered_checkpoint_indices().len();
        let total_count = self
            .analysis
            .as_ref()
            .map(|a| a.checkpoints.len())
            .unwrap_or(0);

        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.is_loading {
                        ui.spinner();
                        ui.label(tr("status.loading"));
                    } else if let Some(ref msg) = self.status_message {
                        let is_recent_copy = self
                            .status_timestamp
                            .map(|ts| ts.elapsed() < Duration::from_secs(3))
                            .unwrap_or(false)
                            && msg.contains(&tr("action.copied"));
                        if is_recent_copy {
                            ui.label(
                                egui::RichText::new(format!("✔ {msg}"))
                                    .color(egui::Color32::LIGHT_GREEN)
                                    .strong(),
                            );
                        } else {
                            ui.label(msg);
                        }
                    } else {
                        ui.label(tr("status.ready"));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if total_count > 0 {
                            ui.label(tf(
                                "ui.showing_filtered_count_of_total_count_checkpoints",
                                &[
                                    ("filtered_count", &format!("{}", filtered_count)),
                                    ("total_count", &format!("{}", total_count)),
                                ],
                            ));
                        }
                    });
                });
            });
    }

    /// Render Central Panel with Tabs.
    pub fn render_central_panel(&mut self, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab bar
            ui.horizontal_wrapped(|ui| {
                for tab in Tab::ALL {
                    let is_active = self.current_tab == tab;
                    let text = format!("{} ({})", tr(tab.title()), tab.shortcut());
                    if ui.selectable_label(is_active, text).clicked() {
                        self.current_tab = tab;
                        self.preferences.active_tab = tab;
                    }
                }
            });
            ui.separator();

            // When Cisco Command Library is selected, render it directly even without an activity loaded
            if self.current_tab == Tab::CommandLibrary {
                self.render_command_library_tab(ui, ctx);
                return;
            }

            // When no analysis is loaded
            if self.analysis.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.heading(tr("empty.welcome"));
                    ui.add_space(8.0);
                    ui.label(tr("empty.description"));
                    ui.add_space(16.0);
                    if ui.button(tr("menu.open_file")).clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter(tr("ui.packet_tracer_pka_pkt"), &["pka", "pkt"])
                            .add_filter(tr("ui.all_files"), &["*"])
                            .pick_file()
                    {
                        self.load_activity_background(path, Some(ctx.clone()));
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
                Tab::CommandLibrary => self.render_command_library_tab(ui, ctx),
            }
        });
    }

    /// Tab 1: Overview
    pub fn render_overview_tab(&mut self, ui: &mut egui::Ui) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        let Some(ref a) = self.analysis else { return };
        let s = a.summary();
        let timings = a.timings.clone();

        let mut device_stats: BTreeMap<String, (usize, usize, usize, usize, f64)> = BTreeMap::new();
        for c in &a.checkpoints {
            let entry = device_stats
                .entry(c.device.clone())
                .or_insert((0, 0, 0, 0, 0.0));
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
            ui.heading(tr("ui.assessment_overview"));
            ui.add_space(8.0);
            let coverage = &a.command_coverage;
            ui.label(tf(
                "cisco.coverage_summary",
                &[
                    ("recognized", &coverage.recognized.to_string()),
                    ("commands", &coverage.command_ready.to_string()),
                    ("gui", &coverage.gui_ready.to_string()),
                    ("verified", &coverage.validation_verified.to_string()),
                    ("total", &s.total.to_string()),
                ],
            ));
            ui.label(
                egui::RichText::new(tr("cisco.coverage_notice"))
                    .small()
                    .weak(),
            );

            // Metrics cards wrap for narrow windows and longer Vietnamese labels.
            ui.horizontal_wrapped(|ui| {
                // Card 1: Checkpoints
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(tr("nav.checkpoints"));
                            ui.heading(format!("{}", s.total));
                            ui.label(tf(
                                "ui.passed_value_failed_value2_unverified_value3",
                                &[
                                    ("value", &format!("{}", s.passed)),
                                    ("value2", &format!("{}", s.failed)),
                                    ("value3", &format!("{}", s.unverified)),
                                ],
                            ));
                        });
                    });

                // Card 2: Score
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(tr("score.verified"));
                            ui.heading(tf(
                                "ui.value_value2_pts",
                                &[
                                    ("value", &format!("{:.1}", s.verified_score)),
                                    ("value2", &format!("{:.1}", s.total_score)),
                                ],
                            ));
                            let pct = if s.total_score > 0.0 {
                                s.verified_score / s.total_score * 100.0
                            } else {
                                0.0
                            };
                            ui.label(tf(
                                "ui.value_score_achieved",
                                &[("value", &format!("{:.1}", pct))],
                            ));
                        });
                    });

                // Card 3: Verification Coverage
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(tr("score.coverage"));
                            ui.heading(format!("{:.1}%", s.coverage * 100.0));
                            ui.label(tf(
                                "ui.value_verified_of_value2",
                                &[
                                    ("value", &format!("{}", s.passed + s.failed)),
                                    ("value2", &format!("{}", s.total)),
                                ],
                            ));
                        });
                    });

                // Card 4: Manual Checklist
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(tr("score.manual"));
                            ui.heading(format!("{checked} / {total}"));
                            let pct = if total > 0 {
                                checked as f64 / total as f64 * 100.0
                            } else {
                                0.0
                            };
                            ui.label(tf(
                                "ui.value_verified_by_you",
                                &[("value", &format!("{:.1}", pct))],
                            ));
                        });
                    });
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Device summary breakdown table
            ui.heading(tr("ui.per_device_breakdown"));
            ui.add_space(6.0);

            egui::Grid::new("device_overview_grid")
                .striped(true)
                .min_col_width(120.0)
                .show(ui, |ui| {
                    ui.strong(tr("table.device"));
                    ui.strong(tr("ui.total"));
                    ui.strong(tr("status.passed"));
                    ui.strong(tr("status.failed"));
                    ui.strong(tr("status.unverified"));
                    ui.strong(tr("table.points"));
                    ui.strong(tr("ui.action"));
                    ui.end_row();

                    for (dev, (tot, pass, fail, unver, pts)) in &device_stats {
                        let dev_name = if dev.is_empty() {
                            tr("table.unassigned")
                        } else {
                            dev.clone()
                        };
                        ui.label(dev_name);
                        ui.label(tot.to_string());
                        ui.colored_label(egui::Color32::from_rgb(76, 175, 80), pass.to_string());
                        ui.colored_label(egui::Color32::from_rgb(244, 67, 54), fail.to_string());
                        ui.colored_label(egui::Color32::from_rgb(255, 179, 0), unver.to_string());
                        ui.label(format!("{pts:.1}"));

                        if ui.small_button(tr("ui.view_checkpoints")).clicked() {
                            select_device = Some(dev.clone());
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Performance and parse timings
            ui.heading(tr("info.timings"));
            ui.add_space(6.0);
            egui::Grid::new("timings_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(tr("ui.phase"));
                    ui.strong(tr("ui.elapsed_time"));
                    ui.end_row();

                    for (phase, duration) in &timings {
                        ui.label(self.localizer.message(&self.preferences.language, phase));
                        ui.label(tf(
                            "ui.value_s_value2_ms",
                            &[
                                ("value", &format!("{:.3}", duration / 1000.0)),
                                ("value2", &format!("{:.1}", *duration)),
                            ],
                        ));
                        ui.end_row();
                    }
                });
        });

        if let Some(dev) = select_device {
            self.selected_device = Some(dev);
            self.current_tab = Tab::Checkpoints;
        }
    }

    /// Virtualized checkpoint table. Selection is rendered in the detail panel.
    pub fn render_checkpoints_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let lang = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&lang, key);
        let indices = self.filtered_checkpoint_indices();
        let mut selected = None;
        let mut manual = None;
        let mut copied: Option<(String, String)> = None;
        ui.horizontal(|ui| {
            ui.label(catalog.format(
                &lang,
                "ui.total_filtered_checkpoints_matching_filters",
                &[("total_filtered", &indices.len().to_string())],
            ));
            if ui.button(tr("ui.copy_csv_of_filtered")).clicked()
                && let Some(a) = &self.analysis
            {
                let subset = Analysis {
                    checkpoints: indices.iter().map(|i| a.checkpoints[*i].clone()).collect(),
                    ..Default::default()
                };
                if let Ok(csv) = reports::csv(&subset) {
                    copied = Some((csv, tr("ui.copy_csv_of_filtered").to_string()));
                }
            }
        });
        let widths = [24.0, 90.0, 130.0, 150.0, 160.0, 160.0, 100.0, 50.0, 130.0];
        let headers = [
            "",
            "table.device",
            "table.interface",
            "table.property",
            "table.expected",
            "table.current",
            "table.status",
            "table.points",
            "table.command",
        ];
        egui::ScrollArea::horizontal()
            .id_salt("checkpoint_horizontal")
            .show(ui, |ui| {
                ui.set_min_width(widths.iter().sum::<f32>() + 72.0);
                ui.horizontal(|ui| {
                    for (key, width) in headers.iter().zip(widths) {
                        ui.add_sized([width, 24.0], egui::Label::new(tr(key)).truncate());
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("checkpoint_rows")
                    .show_rows(ui, 32.0, indices.len(), |ui, range| {
                        if let Some(a) = &self.analysis {
                            for idx in &indices[range] {
                                let c = &a.checkpoints[*idx];
                                ui.push_id(&c.id, |ui| {
                                    ui.horizontal(|ui| {
                                        let mut checked = self.is_manually_checked(&c.id);
                                        if ui
                                            .add_sized(
                                                [widths[0], 26.0],
                                                egui::Checkbox::without_text(&mut checked),
                                            )
                                            .changed()
                                        {
                                            manual = Some((c.id.clone(), checked));
                                        }
                                        let property_label = detect_protocol(c)
                                            .map(|badge| {
                                                format!(
                                                    "{} {}",
                                                    tr(badge.catalog_key()),
                                                    c.property
                                                )
                                            })
                                            .unwrap_or_else(|| c.property.clone());
                                        let values = [
                                            c.device.as_str(),
                                            c.interface.as_str(),
                                            property_label.as_str(),
                                            c.expected.as_str(),
                                            c.current.as_deref().unwrap_or("?"),
                                        ];
                                        for (value, width) in values.iter().zip(widths[1..6].iter())
                                        {
                                            let hover_text = if *value == c.expected.as_str() {
                                                if let Some(detail) = extract_network_detail(c) {
                                                    format!(
                                                        "{}\n{} {}\n{} {}\n{} {}",
                                                        detail.badge.badge_text(),
                                                        tr("detail.normalized_cidr"),
                                                        detail.normalized_cidr,
                                                        tr("detail.subnet_mask"),
                                                        detail
                                                            .subnet_mask
                                                            .as_deref()
                                                            .unwrap_or("-"),
                                                        tr("detail.wildcard_mask"),
                                                        detail
                                                            .wildcard_mask
                                                            .as_deref()
                                                            .unwrap_or("-"),
                                                    )
                                                } else {
                                                    (*value).to_string()
                                                }
                                            } else {
                                                (*value).to_string()
                                            };
                                            if ui
                                                .add_sized(
                                                    [*width, 26.0],
                                                    egui::Button::selectable(
                                                        self.expanded_checkpoint_ids.get(&c.id)
                                                            == Some(&true),
                                                        *value,
                                                    ),
                                                )
                                                .on_hover_text(hover_text)
                                                .clicked()
                                            {
                                                selected = Some(c.id.clone());
                                            }
                                        }
                                        let (key, color) = match c.status {
                                            Status::Passed => {
                                                ("status.passed", egui::Color32::LIGHT_GREEN)
                                            }
                                            Status::Failed => {
                                                ("status.failed", egui::Color32::LIGHT_RED)
                                            }
                                            Status::Unverified => (
                                                "status.unverified",
                                                egui::Color32::from_rgb(255, 190, 80),
                                            ),
                                        };
                                        ui.add_sized(
                                            [widths[6], 26.0],
                                            egui::Label::new(
                                                egui::RichText::new(tr(key)).color(color),
                                            )
                                            .truncate(),
                                        );
                                        ui.add_sized(
                                            [widths[7], 26.0],
                                            egui::Label::new(format!("{}", c.weight)),
                                        );
                                        let cmd = commands::generate(c);
                                        if ui
                                            .add_enabled_ui(cmd.supported, |ui| {
                                                ui.add_sized(
                                                    [widths[8], 26.0],
                                                    egui::Button::new(tr("action.copy")),
                                                )
                                            })
                                            .inner
                                            .on_hover_text(catalog.message(&lang, &cmd.explanation))
                                            .clicked()
                                        {
                                            copied = Some((
                                                cmd.text,
                                                format!("{} {}", c.device, c.property),
                                            ));
                                        }
                                    })
                                });
                            }
                        }
                    });
            });
        if let Some(id) = selected {
            self.expanded_checkpoint_ids.clear();
            self.expanded_checkpoint_ids.insert(id, true);
        }
        if let Some((id, checked)) = manual {
            self.set_manual_check(&id, checked);
        }
        if let Some((text, label)) = copied {
            self.copy_text_with_feedback(ctx, text, &label);
        }
    }

    pub fn render_detail_panel(&mut self, ctx: &egui::Context) {
        if self.analysis.is_none() {
            return;
        }
        let catalog = self.localizer.clone();
        let lang = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&lang, key);
        let selected = self
            .analysis
            .as_ref()
            .and_then(|a| {
                a.checkpoints
                    .iter()
                    .find(|c| self.expanded_checkpoint_ids.get(&c.id) == Some(&true))
            })
            .cloned();
        let mut copy = None;
        egui::SidePanel::right("checkpoint_details")
            .default_width(310.0)
            .width_range(250.0..=500.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading(tr("detail.title"));
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(c) = selected {
                        ui.heading(&c.device);
                        ui.label(&c.interface);
                        ui.strong(&c.property);
                        let classification_key = match c.command_classification {
                            cisco::Classification::Ready => "cisco.class.ready",
                            cisco::Classification::MissingParameters => "cisco.class.missing",
                            cisco::Classification::RecognizedUnsupported => {
                                "cisco.class.unsupported"
                            }
                            cisco::Classification::UnknownAmbiguous => "cisco.class.unknown",
                        };
                        ui.label(egui::RichText::new(tr(classification_key)).small().weak());

                        // Protocol badge if applicable
                        if let Some(badge) = detect_protocol(&c) {
                            ui.add_space(2.0);
                            render_protocol_badge(ui, badge, &tr(badge.catalog_key()));
                            ui.add_space(4.0);
                        }

                        for (key, value) in [
                            ("table.expected", c.expected.as_str()),
                            ("table.current", c.current.as_deref().unwrap_or("?")),
                            ("table.type", c.device_type.as_str()),
                            ("table.points", &c.weight.to_string()),
                        ] {
                            ui.label(egui::RichText::new(tr(key)).weak());
                            ui.label(value);
                        }

                        // Network Details display for static routes and OSPF networks
                        if let Some(detail) = extract_network_detail(&c) {
                            ui.separator();
                            ui.strong(tr("detail.network_details"));
                            egui::Frame::group(ui.style())
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(tr("detail.raw_value")).weak());
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&detail.raw_value).monospace(),
                                        )
                                        .wrap(),
                                    );

                                    ui.label(
                                        egui::RichText::new(tr("detail.normalized_cidr")).weak(),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&detail.normalized_cidr)
                                                .monospace()
                                                .strong(),
                                        )
                                        .wrap(),
                                    );

                                    if let Some(ref mask) = detail.subnet_mask {
                                        ui.label(
                                            egui::RichText::new(tr("detail.subnet_mask")).weak(),
                                        );
                                        ui.add(
                                            egui::Label::new(egui::RichText::new(mask).monospace())
                                                .wrap(),
                                        );
                                    }

                                    if let Some(ref wildcard) = detail.wildcard_mask {
                                        ui.label(
                                            egui::RichText::new(tr("detail.wildcard_mask")).weak(),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(wildcard).monospace(),
                                            )
                                            .wrap(),
                                        );
                                    }

                                    if let Some(ref nh) = detail.next_hop {
                                        ui.label(egui::RichText::new(tr("detail.next_hop")).weak());
                                        ui.add(
                                            egui::Label::new(egui::RichText::new(nh).monospace())
                                                .wrap(),
                                        );
                                    }
                                    if let Some(ref iface) = detail.exit_interface {
                                        ui.label(
                                            egui::RichText::new(tr("detail.exit_interface")).weak(),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(iface).monospace(),
                                            )
                                            .wrap(),
                                        );
                                    }
                                    if let Some(dist) = detail.distance {
                                        ui.label(egui::RichText::new(tr("detail.distance")).weak());
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(dist.to_string()).monospace(),
                                            )
                                            .wrap(),
                                        );
                                    }
                                    if let Some(ref area) = detail.area {
                                        ui.label(
                                            egui::RichText::new(tr("detail.ospf_area")).weak(),
                                        );
                                        ui.add(
                                            egui::Label::new(egui::RichText::new(area).monospace())
                                                .wrap(),
                                        );
                                    }
                                    if let Some(pid) = detail.process_id {
                                        ui.label(
                                            egui::RichText::new(tr("detail.ospf_process")).weak(),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(pid.to_string()).monospace(),
                                            )
                                            .wrap(),
                                        );
                                    }
                                });

                            if let Some(ref curr_str) = c.current
                                && !curr_str.trim().is_empty()
                                && curr_str != "?"
                            {
                                let curr_cp = Checkpoint {
                                    command_classification: Default::default(),
                                    property: c.property.clone(),
                                    path: c.path.clone(),
                                    expected: curr_str.clone(),
                                    ..Default::default()
                                };
                                if let Some(curr_detail) = extract_network_detail(&curr_cp) {
                                    ui.add_space(4.0);
                                    ui.strong(tr("detail.current_network"));
                                    egui::Frame::group(ui.style())
                                        .inner_margin(egui::Margin::same(8))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(tr("detail.raw_value")).weak(),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&curr_detail.raw_value)
                                                        .monospace(),
                                                )
                                                .wrap(),
                                            );

                                            ui.label(
                                                egui::RichText::new(tr("detail.normalized_cidr"))
                                                    .weak(),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(
                                                        &curr_detail.normalized_cidr,
                                                    )
                                                    .monospace()
                                                    .strong(),
                                                )
                                                .wrap(),
                                            );

                                            if let Some(ref mask) = curr_detail.subnet_mask {
                                                ui.label(
                                                    egui::RichText::new(tr("detail.subnet_mask"))
                                                        .weak(),
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(mask).monospace(),
                                                    )
                                                    .wrap(),
                                                );
                                            }

                                            if let Some(ref wildcard) = curr_detail.wildcard_mask {
                                                ui.label(
                                                    egui::RichText::new(tr("detail.wildcard_mask"))
                                                        .weak(),
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(wildcard).monospace(),
                                                    )
                                                    .wrap(),
                                                );
                                            }
                                        });
                                }
                            }
                        }

                        ui.separator();
                        ui.strong(tr("detail.explanation"));
                        ui.label(catalog.message(&lang, &c.explanation));
                        ui.strong(tr("detail.path"));
                        ui.label(&c.path);
                        ui.collapsing(tr("detail.evidence"), |ui| {
                            for e in &c.evidence {
                                ui.label(e);
                            }
                        });
                        ui.separator();
                        ui.strong(tr("detail.preview"));
                        let cmd = commands::generate(&c);
                        ui.label(catalog.message(&lang, &cmd.explanation));
                        if cmd.supported {
                            ui.monospace(&cmd.text);
                            if ui.button(tr("action.copy_checkpoint")).clicked() {
                                copy = Some((cmd.text, format!("{} {}", c.device, c.property)));
                            }
                        } else if let Some(request) = checkpoint_library_request(&c) {
                            ui.label(tr(match request.family.as_str() {
                                "router.interface.dot1q" => "detail.dot1q_required",
                                "router.ospf.network" => "detail.ospf_process_required",
                                _ => "detail.parameters_required",
                            }));
                            if ui.button(tr("detail.open_command_library")).clicked() {
                                self.library_search_query.clear();
                                self.library_device_filter = CiscoDeviceFilter::All;
                                self.library_topic_filter = None;
                                self.library_selected_id = Some(request.family);
                                self.library_target_device = request.device;
                                self.library_profile = request.profile;
                                self.library_parameters = request.parameters;
                                self.current_tab = Tab::CommandLibrary;
                                self.preferences.active_tab = Tab::CommandLibrary;
                            }
                        }
                        let mut manual = self.is_manually_checked(&c.id);
                        if ui.checkbox(&mut manual, tr("detail.manual")).changed() {
                            self.set_manual_check(&c.id, manual);
                        }
                        ui.small(tr("detail.manual_note"));
                    } else {
                        ui.label(tr("detail.select"));
                    }
                });
            });
        if let Some((text, label)) = copy {
            self.copy_text_with_feedback(ctx, text, &label);
        }
    }

    /// Tab 3: Next Actions (Remediation plan for incomplete items).
    pub fn render_next_actions_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
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
                if a.checkpoints.is_empty() {
                    ui.heading(tr("empty.no_results"));
                    ui.label(tr("nav.diagnostics"));
                } else {
                    ui.heading(tr("ui.all_checkpoints_passed"));
                    ui.label(tr(
                        "ui.no_pending_remediation_actions_remain_for_this_activity",
                    ));
                }
            });
            return;
        }

        // Verified failures come first; group by device/interface, then weight.
        incomplete.sort_by(|a, b| {
            (a.status != Status::Failed, &a.device, &a.interface)
                .cmp(&(b.status != Status::Failed, &b.device, &b.interface))
                .then_with(|| {
                    b.weight
                        .partial_cmp(&a.weight)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        let mut toggle_check: Option<(String, bool)> = None;
        let mut to_copy: Option<(String, String)> = None;

        ui.horizontal(|ui| {
            ui.heading(tf(
                "ui.next_actions_value_remaining",
                &[("value", &format!("{}", incomplete.len()))],
            ));
            ui.separator();
            if ui.button(tr("action.copy_all")).clicked() {
                let script = commands::device_script(&incomplete, None);
                to_copy = Some((script, tr("action.copy_all").to_string()));
            }
            ui.separator();
            ui.label(
                egui::RichText::new(tr("ui.paste_target_notice"))
                    .small()
                    .color(egui::Color32::from_rgb(255, 204, 128)),
            );
        });
        ui.add_space(8.0);

        let checklist = self.current_checklist();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, c) in incomplete.iter().enumerate() {
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut checked = checklist
                                .checked_checkpoints
                                .get(&c.id)
                                .copied()
                                .unwrap_or(false);
                            if ui.checkbox(&mut checked, "").changed() {
                                toggle_check = Some((c.id.clone(), checked));
                            }

                            ui.strong(format!("#{}. {} - {}", idx + 1, c.device, c.property));
                            if !c.interface.is_empty() {
                                ui.label(tf(
                                    "ui.interface_value",
                                    &[("value", &c.interface.to_string())],
                                ));
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.colored_label(
                                        if c.status == Status::Failed {
                                            egui::Color32::from_rgb(244, 67, 54)
                                        } else {
                                            egui::Color32::from_rgb(255, 179, 0)
                                        },
                                        tr(match c.status {
                                            Status::Passed => "status.passed",
                                            Status::Failed => "status.failed",
                                            Status::Unverified => "status.unverified",
                                        }),
                                    );
                                    ui.strong(tf(
                                        "ui.value_pts",
                                        &[("value", &format!("{:.1}", c.weight))],
                                    ));
                                },
                            );
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(tr("table.expected"));
                            ui.monospace(&c.expected);
                            if let Some(ref curr) = c.current {
                                ui.label(tr("table.current"));
                                ui.monospace(curr);
                            }
                        });

                        if let Some(detail) = extract_network_detail(c) {
                            ui.add_space(3.0);
                            ui.horizontal_wrapped(|ui| {
                                render_protocol_badge(
                                    ui,
                                    detail.badge,
                                    &tr(detail.badge.catalog_key()),
                                );
                                ui.label(egui::RichText::new(tr("detail.normalized_cidr")).weak());
                                ui.monospace(&detail.normalized_cidr);
                                if let Some(ref mask) = detail.subnet_mask {
                                    ui.label(egui::RichText::new(tr("detail.subnet_mask")).weak());
                                    ui.monospace(mask);
                                }
                                if let Some(ref wildcard) = detail.wildcard_mask {
                                    ui.label(
                                        egui::RichText::new(tr("detail.wildcard_mask")).weak(),
                                    );
                                    ui.monospace(wildcard);
                                }
                            });
                        }

                        ui.add_space(4.0);
                        let cmd = commands::generate(c);
                        if cmd.supported && !cmd.gui {
                            ui.horizontal(|ui| {
                                ui.monospace(&cmd.text);
                                if ui.small_button(tr("action.copy_checkpoint")).clicked() {
                                    to_copy = Some((
                                        cmd.text.clone(),
                                        format!("{} {}", c.device, c.property),
                                    ));
                                }
                            });
                        } else if cmd.gui {
                            ui.colored_label(
                                egui::Color32::LIGHT_BLUE,
                                tf(
                                    "ui.manual_gui_action_value",
                                    &[("value", &cmd.text.to_string())],
                                ),
                            );
                        } else {
                            ui.colored_label(
                                egui::Color32::LIGHT_RED,
                                tf(
                                    "ui.no_automatic_cli_command_available_value",
                                    &[(
                                        "value",
                                        &self
                                            .localizer
                                            .message(&self.preferences.language, &cmd.explanation)
                                            .to_string(),
                                    )],
                                ),
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
            self.copy_text_with_feedback(ctx, text, &label);
        }
    }

    /// Tab 4: Device Commands Script Generation.
    pub fn render_device_commands_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        let Some(ref a) = self.analysis else { return };

        let devices = self.unique_devices();
        let current_filter = self
            .command_device_filter
            .clone()
            .unwrap_or_else(|| tr("filter.all_devices"));
        let target_device = self.command_device_filter.as_deref();
        let script = commands::device_script(&a.checkpoints, target_device);

        let mut select_dev: Option<Option<String>> = None;
        let mut do_copy: Option<(String, String)> = None;
        let mut status_update: Option<String> = None;

        let active_filter = self.command_device_filter.clone();

        ui.horizontal(|ui| {
            ui.heading(tr("ui.device_configuration_scripts"));
            ui.separator();

            // Device selector
            egui::ComboBox::from_id_salt("cmd_device_selector")
                .selected_text(&current_filter)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(active_filter.is_none(), tr("filter.all_devices"))
                        .clicked()
                    {
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
            if ui.button(tr("action.copy_device")).clicked() {
                do_copy = Some((script.clone(), current_filter.clone()));
            }

            // Export to file button
            if ui.button(tr("ui.save_script_to_file")).clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr("ui.cli_script_txt_ios"), &["txt", "ios", "cli"])
                    .set_file_name("device_script.txt")
                    .save_file()
            {
                if let Err(e) = self.export_data(&path, &script) {
                    status_update =
                        Some(tf("ui.error_saving_script_e", &[("e", &format!("{}", e))]));
                } else {
                    status_update = Some(tf(
                        "ui.script_saved_to_value",
                        &[("value", &format!("{}", path.display()))],
                    ));
                }
            }
        });

        ui.add_space(8.0);

        // Visible concise localized instruction banner
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::symmetric(10, 6))
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(255, 179, 0),
            ))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(255, 179, 0), "⚠");
                    ui.label(
                        egui::RichText::new(tr("ui.paste_target_notice"))
                            .strong()
                            .color(egui::Color32::from_rgb(255, 204, 128)),
                    );
                });
            });

        ui.add_space(6.0);

        // Show unsupported / GUI details separately with concise localized summaries
        let unsupported_checkpoints: Vec<&Checkpoint> = a
            .checkpoints
            .iter()
            .filter(|c| {
                if let Some(target) = target_device
                    && c.device != target
                {
                    return false;
                }
                let cmd = commands::generate(c);
                !cmd.supported || cmd.gui
            })
            .collect();

        if !unsupported_checkpoints.is_empty() {
            let count_str = unsupported_checkpoints.len().to_string();
            let title = tf("ui.unsupported_checkpoints_title", &[("count", &count_str)]);
            egui::CollapsingHeader::new(egui::RichText::new(title).strong())
                .id_salt("unsupported_checkpoints_collapsing")
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(tr("ui.unsupported_checkpoints_desc"))
                            .small()
                            .weak(),
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("unsupported_checkpoints_scroll")
                        .max_height(140.0)
                        .show(ui, |ui| {
                            for c in &unsupported_checkpoints {
                                let cmd = commands::generate(c);
                                let reason = if cmd.gui {
                                    tf("ui.manual_gui_action_value", &[("value", &cmd.text)])
                                } else {
                                    catalog.message(&language, &cmd.explanation)
                                };
                                ui.horizontal(|ui| {
                                    ui.colored_label(
                                        if cmd.gui {
                                            egui::Color32::LIGHT_BLUE
                                        } else {
                                            egui::Color32::from_rgb(255, 179, 0)
                                        },
                                        "•",
                                    );
                                    ui.strong(&c.device);
                                    if !c.interface.is_empty() {
                                        ui.label(format!("({})", c.interface));
                                    }
                                    ui.label(&c.property);
                                    ui.label(egui::RichText::new(format!("— {reason}")).weak());
                                });
                            }
                        });
                });
            ui.add_space(6.0);
        }

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
            self.copy_text_with_feedback(ctx, text, &label);
        }
        if let Some(msg) = status_update {
            self.set_status(msg);
        }
    }

    /// Tab 5: Raw XML Viewer.
    pub fn render_raw_xml_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        let Some(ref a) = self.analysis else { return };
        let identity = (a.xml.as_ptr() as usize, a.xml.len());
        if self.xml_cache_identity != identity {
            self.xml_chunks = xml_chunks(&a.xml);
            self.xml_cache_identity = identity;
        }
        let xml_len = a.xml.len();
        let xml_chars = a.xml.chars().count();
        let matches = if !self.xml_search_query.is_empty() {
            a.xml.matches(&self.xml_search_query).count()
        } else {
            0
        };

        let mut do_copy = false;

        ui.horizontal(|ui| {
            ui.heading(tr("nav.raw_xml"));
            ui.separator();

            // Search query within XML
            ui.add(
                egui::TextEdit::singleline(&mut self.xml_search_query)
                    .hint_text(tr("ui.find_in_xml")),
            );

            if !self.xml_search_query.is_empty() {
                ui.label(tf(
                    "ui.matches_matches",
                    &[("matches", &format!("{}", matches))],
                ));
                if ui.small_button("✕").clicked() {
                    self.xml_search_query.clear();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr("ui.copy_xml")).clicked() {
                    do_copy = true;
                }
                ui.label(tf(
                    "ui.size_value_kb_value2_chars",
                    &[
                        ("value", &format!("{:.2}", xml_len as f64 / 1024.0)),
                        ("value2", &format!("{}", xml_chars)),
                    ],
                ));
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("xml_rows").show_rows(
                ui,
                20.0,
                self.xml_chunks.len(),
                |ui, range| {
                    for index in range {
                        let (start, end) = self.xml_chunks[index];
                        ui.horizontal(|ui| {
                            ui.weak(format!("{:>6}", index + 1));
                            ui.monospace(&a.xml[start..end]);
                        });
                    }
                },
            );
        });

        if do_copy {
            self.copy_text_with_feedback(ctx, a.xml.clone(), &tr("ui.copy_xml"));
        }
    }

    /// Tab 6: Diagnostics & Parser Logs.
    pub fn render_diagnostics_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        let Some(ref a) = self.analysis else { return };
        let diagnostics: Vec<String> = a
            .diagnostics
            .iter()
            .chain(self.localizer.diagnostics.iter())
            .map(|d| self.localizer.message(&self.preferences.language, d))
            .collect();
        let mut do_copy = false;

        ui.horizontal(|ui| {
            ui.heading(tf(
                "ui.diagnostics_value_items",
                &[("value", &format!("{}", diagnostics.len()))],
            ));
            ui.separator();
            if ui.button(tr("ui.copy_diagnostics")).clicked() {
                do_copy = true;
            }
        });

        ui.add_space(8.0);

        if diagnostics.is_empty() {
            ui.label(tr("empty.no_diagnostics"));
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, diag) in diagnostics.iter().enumerate() {
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(255, 179, 0), "⚠");
                                ui.strong(format!("#{}:", idx + 1));
                                ui.label(self.localizer.message(&self.preferences.language, diag));
                            });
                        });
                    ui.add_space(4.0);
                }
            });
        }

        if do_copy {
            let text = diagnostics.join("\n");
            self.copy_text_with_feedback(ctx, text, &tr("ui.copy_diagnostics"));
        }
    }

    /// Tab 7: Cisco Command Library tab entrypoint.
    pub fn render_command_library_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let specs = cisco::registry();
        self.render_command_library_content(ui, ctx, &specs);
    }

    /// Alias for render_command_library_content matching the Cisco Command Library renderer.
    #[inline]
    #[allow(dead_code)]
    pub fn render_cisco_library(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        specs: &[CommandSpec],
    ) {
        self.render_command_library_content(ui, ctx, specs);
    }

    /// Render Cisco Command Library content with the provided command specifications.
    pub fn render_command_library_content(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        specs: &[CommandSpec],
    ) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let tf = |key: &str, values: &[(&str, &str)]| catalog.format(&language, key, values);
        let is_vi = matches!(language.as_str(), "vi" | "vi-VN" | "vi_VN");

        let mut do_copy: Option<(String, String)> = None;

        // Unique topics from specs
        let mut unique_topics: Vec<String> = specs.iter().map(|s| s.topic.to_string()).collect();
        unique_topics.sort();
        unique_topics.dedup();

        // Top Filter Bar
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("🔍").size(14.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.library_search_query)
                    .hint_text(tr("cisco.search_placeholder"))
                    .desired_width(180.0),
            );
            if !self.library_search_query.is_empty() && ui.button("✖").clicked() {
                self.library_search_query.clear();
            }

            ui.separator();

            // Device category filter: All / Router / Switch
            if ui
                .selectable_label(
                    self.library_device_filter == CiscoDeviceFilter::All,
                    tr("cisco.filter_all"),
                )
                .clicked()
            {
                self.library_device_filter = CiscoDeviceFilter::All;
            }
            if ui
                .selectable_label(
                    self.library_device_filter == CiscoDeviceFilter::Router,
                    tr("cisco.filter_router"),
                )
                .clicked()
            {
                self.library_device_filter = CiscoDeviceFilter::Router;
            }
            if ui
                .selectable_label(
                    self.library_device_filter == CiscoDeviceFilter::Switch,
                    tr("cisco.filter_switch"),
                )
                .clicked()
            {
                self.library_device_filter = CiscoDeviceFilter::Switch;
            }

            ui.separator();

            // Topic filter
            ui.label(tr("cisco.topic_filter"));
            let all_topics_label = tr("cisco.topic_all");
            let current_topic = self
                .library_topic_filter
                .as_deref()
                .unwrap_or(&all_topics_label);
            egui::ComboBox::from_id_salt("cisco_topic_filter_combo")
                .selected_text(if current_topic.starts_with("cisco.topic.") {
                    tr(current_topic)
                } else {
                    current_topic.to_owned()
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            self.library_topic_filter.is_none(),
                            tr("cisco.topic_all"),
                        )
                        .clicked()
                    {
                        self.library_topic_filter = None;
                    }
                    for topic in &unique_topics {
                        let is_sel = self.library_topic_filter.as_deref() == Some(topic.as_str());
                        if ui
                            .selectable_label(
                                is_sel,
                                if topic.starts_with("cisco.topic.") {
                                    tr(topic)
                                } else {
                                    topic.clone()
                                },
                            )
                            .clicked()
                        {
                            self.library_topic_filter = Some(topic.clone());
                        }
                    }
                });

            ui.separator();

            // Profile selector
            ui.label(tr("cisco.profile_label"));
            egui::ComboBox::from_id_salt("cisco_profile_selector_combo")
                .selected_text(self.library_profile.label())
                .show_ui(ui, |ui| {
                    for prof in DeviceProfile::ALL {
                        let is_sel = self.library_profile == prof;
                        if ui.selectable_label(is_sel, prof.label()).clicked() {
                            self.library_profile = prof;
                        }
                    }
                });

            ui.separator();

            // Target device name
            ui.label(tr("cisco.target_device"));
            ui.add(egui::TextEdit::singleline(&mut self.library_target_device).desired_width(80.0));
        });

        ui.separator();

        // Filter specs
        let filtered_specs: Vec<&CommandSpec> = specs
            .iter()
            .filter(|spec| {
                match self.library_device_filter {
                    CiscoDeviceFilter::All => {}
                    CiscoDeviceFilter::Router => {
                        let is_router = spec.id.starts_with("router.")
                            || spec.id.starts_with("basic.")
                            || spec.capability == Capability::Basic
                            || spec.capability == Capability::Routing
                            || spec.capability == Capability::DynamicRouting
                            || spec.capability == Capability::Ipv6Routing;
                        if !is_router {
                            return false;
                        }
                    }
                    CiscoDeviceFilter::Switch => {
                        let is_switch = spec.id.starts_with("switch.")
                            || spec.id.starts_with("basic.")
                            || spec.capability == Capability::Basic
                            || spec.capability == Capability::Switching
                            || spec.capability == Capability::Multilayer;
                        if !is_switch {
                            return false;
                        }
                    }
                }

                if let Some(ref t) = self.library_topic_filter
                    && spec.topic != t
                {
                    return false;
                }

                if !self.library_search_query.trim().is_empty() {
                    let q = self.library_search_query.to_lowercase();
                    let name_match = spec.name_en.to_lowercase().contains(&q)
                        || spec.name_vi.to_lowercase().contains(&q);
                    let id_match = spec.id.to_lowercase().contains(&q);
                    let syntax_match = spec.syntax.to_lowercase().contains(&q);
                    let topic_match = spec.topic.to_lowercase().contains(&q)
                        || tr(spec.topic).to_lowercase().contains(&q);
                    if !name_match && !id_match && !syntax_match && !topic_match {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Auto-select first spec if current selection is invalid or missing
        if let Some(ref sel_id) = self.library_selected_id {
            if !filtered_specs.iter().any(|s| s.id == sel_id) && !filtered_specs.is_empty() {
                self.select_library_family(filtered_specs[0].id);
            }
        } else if !filtered_specs.is_empty() {
            self.select_library_family(filtered_specs[0].id);
        }

        // Capture full available size for the two-pane layout before creating child layouts
        let available_size = ui.available_size();
        let full_height = available_size.y.max(200.0);
        let left_width = 340.0_f32.min(available_size.x * 0.45).max(220.0);

        self.library_layout_height = full_height;
        self.library_left_width = left_width;
        self.library_right_width = (available_size.x - left_width).max(100.0);

        // Two-pane master-detail layout reserving explicit remaining height
        ui.allocate_ui_with_layout(
            egui::vec2(available_size.x, full_height),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.set_min_height(full_height);

                // Left pane: Command list
                ui.allocate_ui_with_layout(
                    egui::vec2(left_width, full_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_height(full_height);
                        ui.set_width(left_width);
                        if specs.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.label(egui::RichText::new(tr("cisco.empty_registry")).italics());
                            });
                            return;
                        }

                        if filtered_specs.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.label(egui::RichText::new(tr("cisco.empty_search")).italics());
                            });
                            return;
                        }

                        egui::ScrollArea::vertical()
                            .id_salt("cisco_specs_list_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for spec in &filtered_specs {
                                    let is_selected =
                                        self.library_selected_id.as_deref() == Some(spec.id);
                                    let name = if is_vi { spec.name_vi } else { spec.name_en };
                                    let support = self.library_profile.support(spec.capability);

                                    let item_frame = egui::Frame::NONE
                                        .fill(if is_selected {
                                            egui::Color32::from_rgba_unmultiplied(60, 90, 140, 60)
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        })
                                        .corner_radius(egui::CornerRadius::same(4))
                                        .inner_margin(egui::Margin::symmetric(8, 6));

                                    item_frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let btn = ui.selectable_label(is_selected, name);
                                            if btn.clicked()
                                                && self.library_selected_id.as_deref()
                                                    != Some(spec.id)
                                            {
                                                self.select_library_family(spec.id);
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    render_cisco_support_badge(
                                                        ui, support, &catalog, &language,
                                                    );
                                                },
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.monospace(
                                                egui::RichText::new(spec.id).size(10.0).weak(),
                                            );
                                            ui.separator();
                                            ui.label(
                                                egui::RichText::new(
                                                    if spec.topic.starts_with("cisco.topic.") {
                                                        tr(spec.topic)
                                                    } else {
                                                        spec.topic.to_owned()
                                                    },
                                                )
                                                .size(10.0)
                                                .italics(),
                                            );
                                        });
                                    });
                                    ui.separator();
                                }
                            });
                    },
                );

                ui.separator();

                // Right pane: Detail and Generation Form
                let right_width = ui.available_width().max(100.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(right_width, full_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_height(full_height);
                        ui.set_width(right_width);

                        let selected_spec = self
                            .library_selected_id
                            .as_deref()
                            .and_then(|id| specs.iter().find(|s| s.id == id));

                        let Some(spec) = selected_spec else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(egui::RichText::new(tr("cisco.no_selection")).italics());
                            });
                            return;
                        };

                        egui::ScrollArea::vertical()
                            .id_salt("cisco_spec_detail_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let name = if is_vi { spec.name_vi } else { spec.name_en };

                                // Title & ID
                                ui.heading(name);
                                ui.horizontal(|ui| {
                                    ui.monospace(egui::RichText::new(spec.id).strong());
                                    ui.separator();
                                    ui.label(format!("{}: {}", tr("cisco.mode"), spec.mode));
                                    ui.separator();
                                    ui.label(format!(
                                        "{}: {:?}",
                                        tr("cisco.capability"),
                                        spec.capability
                                    ));
                                });
                                ui.add_space(6.0);

                                // Profile Support Banner
                                let support = self.library_profile.support(spec.capability);
                                let (bg_color, fg_color, border_color, notice_key) = match support {
                                    SupportState::VerifiedPacketTracer => (
                                        egui::Color32::from_rgb(27, 94, 32),
                                        egui::Color32::from_rgb(200, 230, 201),
                                        egui::Color32::from_rgb(76, 175, 80),
                                        "cisco.support_notice_verified",
                                    ),
                                    SupportState::DocumentedNotTested => (
                                        egui::Color32::from_rgb(13, 71, 161),
                                        egui::Color32::from_rgb(187, 222, 251),
                                        egui::Color32::from_rgb(33, 150, 243),
                                        "cisco.support_notice_not_tested",
                                    ),
                                    SupportState::Unsupported => (
                                        egui::Color32::from_rgb(183, 28, 28),
                                        egui::Color32::from_rgb(255, 205, 210),
                                        egui::Color32::from_rgb(244, 67, 54),
                                        "cisco.support_notice_unsupported",
                                    ),
                                    SupportState::Unknown => (
                                        egui::Color32::from_rgb(66, 66, 66),
                                        egui::Color32::from_rgb(224, 224, 224),
                                        egui::Color32::from_rgb(158, 158, 158),
                                        "cisco.support_notice_unknown",
                                    ),
                                };

                                egui::Frame::NONE
                                    .fill(bg_color)
                                    .stroke(egui::Stroke::new(1.0_f32, border_color))
                                    .corner_radius(egui::CornerRadius::same(4))
                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            render_cisco_support_badge(
                                                ui, support, &catalog, &language,
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}: {}",
                                                    self.library_profile.label(),
                                                    tr(notice_key)
                                                ))
                                                .color(fg_color),
                                            );
                                        });
                                    });
                                ui.add_space(8.0);

                                // Syntax Box
                                ui.strong(tr("cisco.syntax"));
                                ui.label(tr("detail.reference_syntax"));
                                egui::Frame::NONE
                                    .fill(egui::Color32::from_rgb(20, 24, 30))
                                    .corner_radius(egui::CornerRadius::same(4))
                                    .inner_margin(egui::Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.monospace(
                                                egui::RichText::new(spec.syntax)
                                                    .color(egui::Color32::from_rgb(144, 202, 249)),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button(tr("action.copy")).clicked() {
                                                        do_copy = Some((
                                                            spec.syntax.to_string(),
                                                            tr("cisco.syntax").to_string(),
                                                        ));
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(8.0);

                                // Source Documentation & Link Click
                                ui.horizontal(|ui| {
                                    ui.strong(format!("{}:", tr("cisco.source")));
                                    if spec.source.starts_with("http://")
                                        || spec.source.starts_with("https://")
                                    {
                                        ui.hyperlink_to(spec.source, spec.source);
                                        if ui.small_button(tr("action.copy")).clicked() {
                                            do_copy = Some((
                                                spec.source.to_string(),
                                                tr("cisco.source_copied").to_string(),
                                            ));
                                        }
                                    } else {
                                        ui.label(spec.source);
                                        if ui.small_button(tr("action.copy")).clicked() {
                                            do_copy = Some((
                                                spec.source.to_string(),
                                                tr("cisco.source_copied").to_string(),
                                            ));
                                        }
                                    }
                                });
                                ui.add_space(8.0);

                                // Prerequisites
                                ui.strong(tr("cisco.prerequisites"));
                                if spec.prerequisites.is_empty() {
                                    ui.label(
                                        egui::RichText::new(tr("cisco.no_prerequisites")).weak(),
                                    );
                                } else {
                                    for prereq in &spec.prerequisites {
                                        ui.label(format!("• {prereq}"));
                                    }
                                }
                                ui.add_space(8.0);

                                // Verification Commands
                                ui.horizontal(|ui| {
                                    ui.strong(tr("cisco.verification"));
                                    if !spec.verification.is_empty()
                                        && ui.small_button(tr("cisco.copy_verification")).clicked()
                                    {
                                        let joined = spec.verification.join("\n");
                                        do_copy = Some((
                                            joined,
                                            tr("cisco.verification_copied").to_string(),
                                        ));
                                    }
                                });
                                if spec.verification.is_empty() {
                                    ui.label(
                                        egui::RichText::new(tr("cisco.no_verification")).weak(),
                                    );
                                } else {
                                    for vcmd in &spec.verification {
                                        ui.horizontal(|ui| {
                                            ui.monospace(format!("  {vcmd}"));
                                            if ui.small_button(tr("action.copy")).clicked() {
                                                do_copy = Some((
                                                    vcmd.to_string(),
                                                    tr("action.copied").to_string(),
                                                ));
                                            }
                                        });
                                    }
                                }
                                ui.add_space(10.0);

                                // Parameter Form
                                ui.label(
                                    egui::RichText::new(tr("cisco.example_notice"))
                                        .small()
                                        .weak(),
                                );
                                ui.horizontal(|ui| {
                                    ui.heading(tr("cisco.parameters"));
                                    if !spec.parameters.is_empty() {
                                        if ui.button(tr("cisco.fill_all_examples")).clicked() {
                                            self.library_parameters =
                                                cisco::example_request(spec, self.library_profile)
                                                    .parameters;
                                        }
                                        if ui.button(tr("cisco.reset_parameters")).clicked() {
                                            self.library_parameters.clear();
                                        }
                                    }
                                });

                                if spec.parameters.is_empty() {
                                    ui.label(egui::RichText::new(tr("cisco.no_parameters")).weak());
                                } else {
                                    egui::Grid::new("cisco_parameters_form_grid")
                                        .num_columns(4)
                                        .spacing([12.0, 8.0])
                                        .show(ui, |ui| {
                                            for p in &spec.parameters {
                                                // Column 1: Name and Required tag
                                                ui.horizontal(|ui| {
                                                    ui.strong(p.name);
                                                    if p.required {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "[{}]",
                                                                tr("cisco.param_required")
                                                            ))
                                                            .color(egui::Color32::from_rgb(
                                                                229, 57, 53,
                                                            ))
                                                            .strong(),
                                                        );
                                                    } else {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "[{}]",
                                                                tr("cisco.param_optional")
                                                            ))
                                                            .weak(),
                                                        );
                                                    }
                                                });

                                                // Column 2: Input TextEdit
                                                let val_entry = self
                                                    .library_parameters
                                                    .entry(p.name.to_string())
                                                    .or_default();
                                                ui.add(
                                                    egui::TextEdit::singleline(val_entry)
                                                        .desired_width(180.0),
                                                );

                                                // Column 3: Use Example button
                                                if ui
                                                    .small_button(tr("cisco.use_example"))
                                                    .on_hover_text(p.example)
                                                    .clicked()
                                                {
                                                    self.library_parameters.insert(
                                                        p.name.to_string(),
                                                        p.example.to_string(),
                                                    );
                                                }

                                                // Column 4: Localized Description & Example
                                                let desc = if is_vi {
                                                    p.description_vi
                                                } else {
                                                    p.description_en
                                                };
                                                let ex_str = tf(
                                                    "cisco.param_example",
                                                    &[("example", p.example)],
                                                );
                                                ui.label(format!("{desc} ({ex_str})"));

                                                ui.end_row();
                                            }
                                        });
                                }
                                ui.add_space(12.0);

                                // Command Plan Generation
                                let req = CommandRequest {
                                    family: spec.id.to_string(),
                                    device: self.library_target_device.clone(),
                                    profile: self.library_profile,
                                    parameters: self.library_parameters.clone(),
                                };
                                let plan = cisco::generate(&req);

                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.heading(tr("cisco.preview_title"));
                                    if plan.executable()
                                        && ui.button(tr("cisco.copy_script")).clicked()
                                    {
                                        let script = plan.script();
                                        do_copy =
                                            Some((script, tr("cisco.script_copied").to_string()));
                                    }
                                });

                                if plan.executable() {
                                    ui.label(
                                        egui::RichText::new(tr("ui.paste_target_notice"))
                                            .weak()
                                            .italics(),
                                    );
                                    egui::Frame::NONE
                                        .fill(egui::Color32::from_rgb(18, 22, 28))
                                        .stroke(egui::Stroke::new(
                                            1.0_f32,
                                            egui::Color32::from_rgb(46, 125, 50),
                                        ))
                                        .corner_radius(egui::CornerRadius::same(4))
                                        .inner_margin(egui::Margin::symmetric(10, 8))
                                        .show(ui, |ui| {
                                            let mut script = plan.script();
                                            let rows = script.lines().count().clamp(3, 14);
                                            ui.add(
                                                egui::TextEdit::multiline(&mut script)
                                                    .font(egui::TextStyle::Monospace)
                                                    .code_editor()
                                                    .desired_width(f32::INFINITY)
                                                    .desired_rows(rows),
                                            );
                                        });
                                } else {
                                    ui.label(
                                        egui::RichText::new(tr("cisco.not_executable"))
                                            .color(egui::Color32::from_rgb(255, 179, 0))
                                            .italics(),
                                    );

                                    // Missing parameters
                                    if !plan.missing_parameters.is_empty() {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(40, 15, 15))
                                            .corner_radius(egui::CornerRadius::same(4))
                                            .inner_margin(egui::Margin::symmetric(8, 6))
                                            .show(ui, |ui| {
                                                ui.strong(
                                                    egui::RichText::new(tr(
                                                        "cisco.missing_parameters",
                                                    ))
                                                    .color(egui::Color32::from_rgb(239, 83, 80)),
                                                );
                                                for mp in &plan.missing_parameters {
                                                    ui.label(
                                                        egui::RichText::new(format!("  • {mp}"))
                                                            .color(egui::Color32::from_rgb(
                                                                255, 205, 210,
                                                            )),
                                                    );
                                                }
                                            });
                                        ui.add_space(4.0);
                                    }

                                    // Warnings
                                    if !plan.warnings.is_empty() {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(45, 35, 15))
                                            .corner_radius(egui::CornerRadius::same(4))
                                            .inner_margin(egui::Margin::symmetric(8, 6))
                                            .show(ui, |ui| {
                                                ui.strong(
                                                    egui::RichText::new(tr("cisco.warnings"))
                                                        .color(egui::Color32::from_rgb(
                                                            255, 183, 77,
                                                        )),
                                                );
                                                for w in &plan.warnings {
                                                    ui.label(format!(
                                                        "  • {}",
                                                        catalog.message(&language, w)
                                                    ));
                                                }
                                            });
                                        ui.add_space(4.0);
                                    }

                                    // Conflicts
                                    if !plan.conflicts.is_empty() {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(45, 20, 20))
                                            .corner_radius(egui::CornerRadius::same(4))
                                            .inner_margin(egui::Margin::symmetric(8, 6))
                                            .show(ui, |ui| {
                                                ui.strong(
                                                    egui::RichText::new(tr("cisco.conflicts"))
                                                        .color(egui::Color32::from_rgb(
                                                            239, 83, 80,
                                                        )),
                                                );
                                                for c in &plan.conflicts {
                                                    ui.label(format!("  • {c}"));
                                                }
                                            });
                                        ui.add_space(4.0);
                                    }

                                    // Evidence
                                    if !plan.evidence.is_empty() {
                                        ui.strong(tr("cisco.evidence"));
                                        for ev in &plan.evidence {
                                            ui.label(format!("  • {ev}"));
                                        }
                                    }
                                }
                            });
                    },
                );
            },
        );

        if let Some((text, label)) = do_copy {
            self.copy_text_with_feedback(ctx, text, &label);
        }
    }

    /// Settings dialog for user preferences, language selection, and original branding.
    pub fn render_settings_window(&mut self, ctx: &egui::Context) {
        let catalog = self.localizer.clone();
        let language = self.preferences.language.clone();
        let tr = |key: &str| catalog.text(&language, key);
        let mut open = self.show_settings;
        egui::Window::new(tr("menu.settings"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("menu.language"));
                    egui::ComboBox::from_id_salt("settings_language_selector")
                        .selected_text(
                            self.localizer
                                .languages()
                                .into_iter()
                                .find(|(code, _)| {
                                    code == &self.preferences.language
                                        || (code == "vi-VN" && self.preferences.language == "vi")
                                        || (code == "en-US" && self.preferences.language == "en")
                                })
                                .map(|(_, name)| name)
                                .unwrap_or_else(|| self.preferences.language.clone()),
                        )
                        .show_ui(ui, |ui| {
                            for (code, name) in self.localizer.languages() {
                                let is_selected = code == self.preferences.language
                                    || (code == "vi-VN" && self.preferences.language == "vi")
                                    || (code == "en-US" && self.preferences.language == "en");
                                if ui.selectable_label(is_selected, &name).clicked() {
                                    self.preferences.language = code;
                                    self.status_message = None;
                                }
                            }
                        });
                    if ui.small_button(tr("menu.reload_languages")).clicked() {
                        self.localizer = Arc::new(Localizer::new());
                    }
                });
                ui.add_space(8.0);
                ui.separator();
                ui.strong(tr("ui.view_options"));
                ui.checkbox(&mut self.preferences.compact_view, tr("ui.compact_rows"));
                ui.checkbox(
                    &mut self.preferences.show_only_manual_unchecked,
                    tr("ui.show_only_unchecked"),
                );
                ui.add_space(8.0);
                ui.separator();
                ui.strong(tr("app.title"));
                ui.label(tr("app.subtitle"));
                ui.label(tr("info.offline"));
                ui.label(tr("info.read_only"));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(tr("disclaimer.cisco")).weak().small());
            });
        self.show_settings = open;
    }
}

fn xml_chunks(xml: &str) -> Vec<(usize, usize)> {
    let mut chunks = Vec::new();
    let mut start = 0;
    for (index, character) in xml.char_indices() {
        if character == '\n' || index - start >= 256 {
            chunks.push((start, index));
            start = if character == '\n' { index + 1 } else { index };
        }
    }
    if start < xml.len() {
        chunks.push((start, xml.len()));
    }
    chunks
}

fn canonical_identity(path: &Path) -> PathBuf {
    if let Ok(path) = path.canonicalize() {
        return path;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    if let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name())
        && let Ok(parent) = parent.canonicalize()
    {
        return parent.join(name);
    }
    // Normalize non-existing paths too, so repeated folder names cannot collide.
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn atomic_export(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (temp, mut file) = loop {
        let temp = parent.join(format!(
            ".pka-inspector-export-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => break (temp, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

impl eframe::App for InspectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain incoming background thread messages
        self.drain_messages();

        // Process global keyboard shortcuts
        self.handle_shortcuts(ctx);

        // Render UI panels
        self.render_top_panel(ctx);
        if self.current_tab != Tab::CommandLibrary {
            self.render_side_panel(ctx);
        }
        self.render_bottom_panel(ctx);
        if self.current_tab != Tab::CommandLibrary {
            self.render_detail_panel(ctx);
        }
        self.render_central_panel(ctx);

        if self.show_settings {
            self.render_settings_window(ctx);
        }

        self.render_notification(ctx);
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
            command_coverage: Default::default(),
            checkpoints: vec![
                Checkpoint {
                    command_classification: Default::default(),
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
                    command_classification: Default::default(),
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
                    command_classification: Default::default(),
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

        assert!(app.activity_key().ends_with("lab1.pka"));
        assert_eq!(app.manual_check_count(), (0, 3));

        // Mark chk1 as checked
        app.set_manual_check("chk1", true);
        assert!(app.is_manually_checked("chk1"));
        assert!(!app.is_manually_checked("chk2"));
        assert_eq!(app.manual_check_count(), (1, 3));

        // Switch activity to lab2.pka
        app.activity_path = Some(PathBuf::from("C:/labs/lab2.pka"));
        assert!(app.activity_key().ends_with("lab2.pka"));
        // In lab2, chk1 should NOT be checked (isolated checklists!)
        assert!(!app.is_manually_checked("chk1"));
        assert_eq!(app.manual_check_count(), (0, 3));

        // Switch back to lab1.pka
        app.activity_path = Some(PathBuf::from("C:/labs/lab1.pka"));
        assert!(app.activity_key().ends_with("lab1.pka"));
        assert!(app.is_manually_checked("chk1"));
        assert_eq!(app.manual_check_count(), (1, 3));
    }

    #[test]
    fn test_preferences_serialization() {
        let prefs = Preferences {
            active_tab: Tab::NextActions,
            compact_view: true,
            last_activity_path: Some("C:/test.pka".into()),
            ..Default::default()
        };

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
        assert_eq!(
            app.analysis.as_ref().unwrap().checkpoints[0].status,
            Status::Failed
        );

        app.clear_progress();
        assert_eq!(
            app.analysis.as_ref().unwrap().checkpoints[0].status,
            Status::Passed
        );
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
                token: 0,
                path: path.clone(),
                result: Ok(analysis),
            })
            .unwrap();

        app.drain_messages();
        assert!(app.analysis.is_some());
        assert_eq!(app.activity_path, Some(path));
        assert_eq!(app.analysis.as_ref().unwrap().checkpoints.len(), 3);
        assert!(app.status_message.is_some());
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
        // Switch0 switchport mode trunk without explicit switch context remains safely unsupported
        assert!(!cmd1.supported);
        assert!(!cmd1.explanation.is_empty());
    }
    #[test]
    fn language_defaults_and_persists_for_existing_and_custom_packs() {
        let old: Preferences = serde_json::from_str(r#"{"compact_view":true}"#).unwrap();
        assert_eq!(old.language, "vi");
        assert!(old.compact_view);
        for language in ["vi", "en", "fr-CA"] {
            let preferences = Preferences {
                language: language.into(),
                ..Preferences::default()
            };
            let saved = serde_json::to_string(&preferences).unwrap();
            assert_eq!(
                serde_json::from_str::<Preferences>(&saved)
                    .unwrap()
                    .language,
                language
            );
        }
    }

    #[test]
    fn language_selector_discovers_external_pack_without_registration() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("fr.json"),
            r#"{"locale":"fr","name":"Fran?ais","messages":{"nav.overview":"Vue g?n?rale"}}"#,
        )
        .unwrap();
        let mut app = InspectorApp::new(None, None);
        app.localizer = Arc::new(Localizer::from_directory(Some(directory.path())));
        assert!(
            app.localizer
                .languages()
                .iter()
                .any(|(code, _)| code == "fr")
        );
        app.preferences.language = "fr".into();
        assert_eq!(
            app.localizer
                .text(&app.preferences.language, Tab::Overview.title()),
            "Vue g?n?rale"
        );
    }

    #[test]
    fn all_static_ui_catalog_keys_exist_in_both_base_languages() {
        let english: pka_lab_inspector::localization::LanguagePack =
            serde_json::from_str(include_str!("../locales/en.json")).unwrap();
        let vietnamese: pka_lab_inspector::localization::LanguagePack =
            serde_json::from_str(include_str!("../locales/vi.json")).unwrap();
        let source = include_str!("ui.rs").split("#[cfg(test)]").next().unwrap();
        for key in source.split('"').filter(|s| {
            s.starts_with("ui.")
                || s.starts_with("nav.")
                || s.starts_with("filter.")
                || s.starts_with("menu.")
                || s.starts_with("status.")
                || s.starts_with("table.")
                || s.starts_with("detail.")
                || s.starts_with("score.")
                || s.starts_with("action.")
                || s.starts_with("empty.")
                || s.starts_with("info.")
                || s.starts_with("error.")
        }) {
            if key.ends_with('.') {
                continue;
            }
            assert!(
                english.messages.contains_key(key),
                "missing English key: {key}"
            );
            assert!(
                vietnamese
                    .messages
                    .get(key)
                    .is_some_and(|value| !value.is_empty()),
                "missing Vietnamese key: {key}"
            );
        }
    }

    #[test]
    fn same_basename_in_separate_folders_has_separate_checklist() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first");
        let second = directory.path().join("second");
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&second).unwrap();
        std::fs::write(first.join("lab.pka"), b"one").unwrap();
        std::fs::write(second.join("lab.pka"), b"two").unwrap();
        let mut app = InspectorApp::new(None, None);
        app.activity_path = Some(first.join("lab.pka"));
        app.set_manual_check("chk1", true);
        let first_key = app.activity_key();
        app.activity_path = Some(second.join("lab.pka"));
        assert_ne!(app.activity_key(), first_key);
        assert!(!app.is_manually_checked("chk1"));
        app.activity_path = Some(first.join(".").join("lab.pka"));
        assert_eq!(app.activity_key(), first_key);
        assert!(app.is_manually_checked("chk1"));
    }

    #[test]
    fn exports_refuse_canonical_sources_and_packet_tracer_extensions() {
        let directory = tempfile::tempdir().unwrap();
        let activity = directory.path().join("activity.xml");
        let progress = directory.path().join("progress.xml");
        std::fs::write(&activity, b"activity source").unwrap();
        std::fs::write(&progress, b"progress source").unwrap();
        let mut app = InspectorApp::new(None, None);
        app.activity_path = Some(activity.clone());
        app.progress_path = Some(progress.clone());
        for target in [
            directory.path().join(".").join("activity.xml"),
            progress.clone(),
            directory.path().join("new.PKA"),
            directory.path().join("new.PkT"),
        ] {
            assert_eq!(
                app.export_data(&target, b"report").unwrap_err().kind(),
                std::io::ErrorKind::PermissionDenied
            );
        }
        assert_eq!(std::fs::read(&activity).unwrap(), b"activity source");
        assert_eq!(std::fs::read(&progress).unwrap(), b"progress source");
        assert!(!directory.path().join("new.PKA").exists());
    }

    #[test]
    fn exporting_over_a_hardlink_replaces_only_the_export_entry() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("activity.pka");
        let destination = directory.path().join("report.txt");
        std::fs::write(&source, b"original packet tracer bytes").unwrap();
        std::fs::hard_link(&source, &destination).unwrap();
        let mut app = InspectorApp::new(None, None);
        app.activity_path = Some(source.clone());
        app.export_data(&destination, b"new report").unwrap();
        assert_eq!(
            std::fs::read(&source).unwrap(),
            b"original packet tracer bytes"
        );
        assert_eq!(std::fs::read(&destination).unwrap(), b"new report");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn failed_atomic_export_removes_its_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("existing-directory");
        std::fs::create_dir(&destination).unwrap();
        assert!(atomic_export(&destination, b"report").is_err());
        assert!(destination.is_dir());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn stale_activity_result_does_not_replace_newer_state_or_finish_its_spinner() {
        let mut app = InspectorApp::new(None, None);
        app.request_token = 2;
        app.is_loading = true;
        app.tx
            .send(LoadMessage::ActivityLoaded {
                token: 1,
                path: "old.pka".into(),
                result: Ok(make_test_analysis()),
            })
            .unwrap();
        app.drain_messages();
        assert!(app.analysis.is_none());
        assert!(app.is_loading);
        app.tx
            .send(LoadMessage::ActivityLoaded {
                token: 2,
                path: "new.pka".into(),
                result: Ok(make_test_analysis()),
            })
            .unwrap();
        app.drain_messages();
        assert_eq!(app.activity_path, Some("new.pka".into()));
        assert!(!app.is_loading);
    }

    #[test]
    fn clearing_progress_invalidates_in_flight_comparison() {
        let mut app = InspectorApp::new(None, None);
        app.original_analysis = Some(make_test_analysis());
        app.request_token = 1;
        app.is_loading = true;
        app.clear_progress();
        let mut compared = make_test_analysis();
        compared.checkpoints[0].status = Status::Failed;
        app.tx
            .send(LoadMessage::ProgressLoaded {
                token: 1,
                path: "stale.pkt".into(),
                result: Ok((make_test_analysis(), Some(compared))),
            })
            .unwrap();
        app.drain_messages();
        assert!(app.progress_path.is_none());
        assert_eq!(
            app.analysis.as_ref().unwrap().checkpoints[0].status,
            Status::Passed
        );
    }

    #[test]
    fn concurrent_load_requests_are_suppressed() {
        let mut app = InspectorApp::new(None, None);
        app.original_analysis = Some(make_test_analysis());
        app.is_loading = true;
        app.request_token = 9;
        app.load_activity_background("unopened.pka".into(), None);
        app.load_progress_background("unopened.pkt".into(), None);
        assert_eq!(app.request_token, 9);
        assert!(app.rx.try_recv().is_err());
    }

    #[test]
    fn completed_progress_uses_the_background_comparison_result() {
        let mut app = InspectorApp::new(None, None);
        app.original_analysis = Some(make_test_analysis());
        let mut compared = make_test_analysis();
        compared.checkpoints[0].status = Status::Failed;
        // The deliberately non-parseable XML must never be reparsed by drain_messages.
        let mut progress = make_test_analysis();
        progress.xml = "not XML".into();
        app.tx
            .send(LoadMessage::ProgressLoaded {
                token: 0,
                path: "progress.pkt".into(),
                result: Ok((progress, Some(compared))),
            })
            .unwrap();
        app.drain_messages();
        assert_eq!(
            app.analysis.as_ref().unwrap().checkpoints[0].status,
            Status::Failed
        );
        assert!(app.progress_analysis.is_some());
    }
}

#[cfg(test)]
mod rendering_checks {
    use super::*;
    #[test]
    fn virtual_xml_rows_preserve_unicode_and_bound_large_lines() {
        let text = format!(
            "{}\n<value>Ti\u{1ebf}ng Vi\u{1ec7}t</value>",
            "\u{e9}".repeat(1000)
        );
        let rows = xml_chunks(&text);
        assert!(rows.iter().all(|(start, end)| end - start <= 259));
        let joined = rows
            .iter()
            .map(|(start, end)| &text[*start..*end])
            .collect::<String>();
        assert_eq!(joined, text.replace('\n', ""));
    }
    #[test]
    fn all_tabs_render_with_actual_synthetic_analysis_and_detail_selection() {
        let context = egui::Context::default();
        let mut app = InspectorApp::new(None, None);
        let analysis = inspector::parse(
            include_str!("../tests/fixtures/synthetic-126.xml").trim_start_matches('\u{feff}'),
        )
        .unwrap();
        app.expanded_checkpoint_ids
            .insert(analysis.checkpoints[0].id.clone(), true);
        app.analysis = Some(analysis);
        for tab in Tab::ALL {
            app.current_tab = tab;
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440., 900.),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    app.render_top_panel(ctx);
                    app.render_side_panel(ctx);
                    app.render_bottom_panel(ctx);
                    app.render_detail_panel(ctx);
                    app.render_central_panel(ctx);
                    app.render_notification(ctx);
                },
            );
            assert!(!output.shapes.is_empty(), "{tab:?}");
        }
    }

    #[test]
    fn test_protocol_detection_and_badges() {
        let cp_ipv4 = Checkpoint {
            command_classification: Default::default(),
            id: "route1".into(),
            device: "Router0".into(),
            property: "staticroute".into(),
            expected: "172.16.128.0-17-172.18.192.1-0-1".into(),
            path: "network/r0/staticroutes".into(),
            ..Default::default()
        };
        assert_eq!(
            detect_protocol(&cp_ipv4),
            Some(ProtocolBadge::Ipv4StaticRoute)
        );
        assert_eq!(
            ProtocolBadge::Ipv4StaticRoute.badge_text(),
            "[IPv4 Static Route]"
        );

        let cp_ipv6 = Checkpoint {
            command_classification: Default::default(),
            id: "route2".into(),
            device: "Router0".into(),
            property: "ipv6staticroute".into(),
            expected: "2001:DB8:ACAD:3::-64-2001:DB8:ACAD:2::2-0-1".into(),
            path: "network/r0/routesv6".into(),
            ..Default::default()
        };
        assert_eq!(
            detect_protocol(&cp_ipv6),
            Some(ProtocolBadge::Ipv6StaticRoute)
        );
        assert_eq!(
            ProtocolBadge::Ipv6StaticRoute.badge_text(),
            "[IPv6 Static Route]"
        );

        let cp_ospf = Checkpoint {
            command_classification: Default::default(),
            id: "ospf1".into(),
            device: "Router0".into(),
            property: "ospfnetwork".into(),
            expected: "10.0.0.0 0.0.0.3 0".into(),
            path: "network/r0/ospf".into(),
            ..Default::default()
        };
        assert_eq!(detect_protocol(&cp_ospf), Some(ProtocolBadge::OspfNetwork));
        assert_eq!(ProtocolBadge::OspfNetwork.badge_text(), "[OSPF Network]");
    }

    #[test]
    fn indexed_ospf_badges_and_library_parameters_do_not_guess_process() {
        let c = Checkpoint {
            device: "Lab-Router".into(),
            device_type: "Router".into(),
            property: "Route0".into(),
            expected: "198.51.100.0 0.0.0.255 2".into(),
            path: "Network / Lab-Router / OSPF / 20 / Networks / Route0".into(),
            ..Default::default()
        };
        assert_eq!(detect_protocol(&c), Some(ProtocolBadge::OspfNetwork));
        let detail = extract_network_detail(&c).unwrap();
        assert_eq!(detail.area.as_deref(), Some("2"));
        assert_eq!(detail.process_id, None);
        let mut request = checkpoint_library_request(&c).unwrap();
        assert_eq!(request.get("network"), "198.51.100.0/24");
        assert_eq!(request.get("area"), "2");
        assert!(request.get("process_id").is_empty());
        assert!(!cisco::generate(&request).executable());
        request.parameters.insert("process_id".into(), "7".into());
        let plan = cisco::generate(&request);
        assert!(plan.executable());
        assert!(plan.script().contains("router ospf 7\n"));
        assert!(!plan.script().contains("ip route"));
        let mut unrelated = c.clone();
        unrelated.property = "IP Address".into();
        unrelated.path = "Network / Lab-Router / Ports / GigabitEthernet0/0".into();
        assert_eq!(detect_protocol(&unrelated), None);
        assert!(checkpoint_library_request(&unrelated).is_none());
        assert_eq!(c.status, Status::Unverified);
    }

    #[test]
    fn dot1q_library_completion_preserves_interface_and_requires_explicit_vlan() {
        let c = Checkpoint {
            device: "Lab-Router".into(),
            device_type: "Router".into(),
            interface: "GigabitEthernet0/0.77".into(),
            property: "Encapsulation".into(),
            expected: "EthernetEncapProcess".into(),
            path: "Network / Lab-Router / Ports / GigabitEthernet0/0.77 / Encapsulation".into(),
            ..Default::default()
        };
        let mut request = checkpoint_library_request(&c).unwrap();
        assert_eq!(request.family, "router.interface.dot1q");
        assert_eq!(request.get("interface"), "GigabitEthernet0/0.77");
        assert!(request.get("vlan").is_empty());
        assert!(!cisco::generate(&request).executable());
        request.parameters.insert("vlan".into(), "42".into());
        let plan = cisco::generate(&request);
        assert!(plan.executable());
        assert!(plan.script().contains("encapsulation dot1Q 42\n"));
        assert!(!plan.script().contains("dot1Q 77"));
        assert_eq!(c.status, Status::Unverified);
        let mut conflict = c.clone();
        conflict.evidence.extend([
            "commands:parameter:vlan=42".into(),
            "commands:parameter:vlan=43".into(),
        ]);
        assert!(checkpoint_library_request(&conflict).is_none());
    }

    #[test]
    fn incomplete_service_requirements_transfer_without_synthetic_context() {
        for (property, expected, interface, family, missing) in [
            (
                "Clock Rate",
                "64000",
                "Serial0/0/0",
                "router.serial.clock",
                "cable_role",
            ),
            (
                "Encapsulation",
                "PPP",
                "Serial0/0/0",
                "router.serial.ppp",
                "peer_encapsulation",
            ),
            (
                "OSPF Router ID",
                "192.0.2.9",
                "",
                "router.ospf.router_id",
                "process_id",
            ),
            (
                "OSPF Reference Bandwidth",
                "1000",
                "",
                "router.ospf.reference_bandwidth",
                "process_id",
            ),
            (
                "CDP Enable",
                "true",
                "GigabitEthernet0/0",
                "basic.cdp_interface",
                "global_enabled",
            ),
        ] {
            let c = Checkpoint {
                device: "Synthetic-Router".into(),
                device_type: "Router".into(),
                property: property.into(),
                expected: expected.into(),
                interface: interface.into(),
                ..Default::default()
            };
            let request = checkpoint_library_request(&c).unwrap();
            assert_eq!(request.family, family);
            assert!(request.get(missing).is_empty());
            assert!(!cisco::generate(&request).executable());
            assert_eq!(c.status, Status::Unverified);
        }
    }

    #[test]
    fn ipv6_ntp_and_lldp_completion_leave_unprovided_context_empty() {
        let ntp = Checkpoint {
            device: "Synthetic-Router".into(),
            device_type: "Router".into(),
            property: "NTP Server".into(),
            expected: "2001:db8::123".into(),
            ..Default::default()
        };
        let mut request = checkpoint_library_request(&ntp).unwrap();
        assert_eq!(request.family, "basic.ntp_server");
        assert_eq!(request.get("server"), "2001:db8::123");
        assert!(request.get("version").is_empty());
        assert!(!cisco::generate(&request).executable());
        request.parameters.insert("version".into(), "4".into());
        assert!(cisco::generate(&request).executable());
        let lldp = Checkpoint {
            device: "Synthetic-Switch".into(),
            device_type: "Switch".into(),
            interface: "Gi0/1".into(),
            property: "LLDP Transmit".into(),
            expected: "true".into(),
            evidence: vec!["assessment:device-model=2960".into()],
            ..Default::default()
        };
        let mut request = checkpoint_library_request(&lldp).unwrap();
        assert_eq!(request.family, "switch.lldp.interface");
        assert_eq!(request.get("transmit"), "true");
        for missing in ["receive", "global_enabled", "interface_mode"] {
            assert!(request.get(missing).is_empty());
        }
        assert!(!cisco::generate(&request).executable());
        request.parameters.extend([
            ("receive".into(), "false".into()),
            ("global_enabled".into(), "true".into()),
            ("interface_mode".into(), "access".into()),
        ]);
        assert!(cisco::generate(&request).executable());
        assert_eq!(lldp.status, Status::Unverified);
    }

    #[test]
    fn test_extract_network_detail_fields() {
        let cp_static = Checkpoint {
            command_classification: Default::default(),
            id: "r1".into(),
            device: "R1".into(),
            property: "staticroute".into(),
            expected: "172.16.128.0-17-172.18.192.1-0-1".into(),
            path: "network/r1/staticroute".into(),
            ..Default::default()
        };
        let detail_static = extract_network_detail(&cp_static).expect("static route detail");
        assert_eq!(detail_static.raw_value, "172.16.128.0-17-172.18.192.1-0-1");
        assert_eq!(detail_static.normalized_cidr, "172.16.128.0/17");
        assert_eq!(detail_static.subnet_mask.as_deref(), Some("255.255.128.0"));
        assert_eq!(detail_static.wildcard_mask.as_deref(), Some("0.0.127.255"));
        assert_eq!(detail_static.badge, ProtocolBadge::Ipv4StaticRoute);

        let cp_ospf = Checkpoint {
            command_classification: Default::default(),
            id: "o1".into(),
            device: "R1".into(),
            property: "ospfnetwork".into(),
            expected: "10.0.0.0 0.0.0.3 0".into(),
            path: "network/r1/ospf".into(),
            ..Default::default()
        };
        let detail_ospf = extract_network_detail(&cp_ospf).expect("ospf detail");
        assert_eq!(detail_ospf.raw_value, "10.0.0.0 0.0.0.3 0");
        assert_eq!(detail_ospf.normalized_cidr, "10.0.0.0/30");
        assert_eq!(detail_ospf.subnet_mask.as_deref(), Some("255.255.255.252"));
        assert_eq!(detail_ospf.wildcard_mask.as_deref(), Some("0.0.0.3"));
        assert_eq!(detail_ospf.badge, ProtocolBadge::OspfNetwork);
    }

    #[test]
    fn test_copy_command_immediate_notification() {
        let mut app = InspectorApp::new(None, None);
        let ctx = egui::Context::default();
        app.copy_text_with_feedback(
            &ctx,
            "ip route 0.0.0.0 0.0.0.0 Serial0/0/0".into(),
            "Default Route",
        );

        assert!(app.copy_notification.is_some());
        let (msg, ts) = app.copy_notification.as_ref().unwrap();
        assert!(msg.contains("Default Route"));
        assert!(ts.elapsed() < Duration::from_secs(1));
        assert!(
            app.status_message
                .as_ref()
                .unwrap()
                .contains("Default Route")
        );

        // Verify toast overlay renders without panicking
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            app.render_notification(ctx);
        });
        assert!(!output.shapes.is_empty());
    }

    #[test]
    fn test_responsive_layout_rendering_with_network_details() {
        let ctx = egui::Context::default();
        let mut app = InspectorApp::new(None, None);
        let analysis = Analysis {
            checkpoints: vec![
                Checkpoint {
                    command_classification: Default::default(),
                    id: "chk_static".into(),
                    device: "Router1".into(),
                    device_type: "Router".into(),
                    interface: "Serial0/0/0".into(),
                    category: "Routing".into(),
                    property: "staticroute".into(),
                    expected: "172.16.128.0-17-172.18.192.1-0-1".into(),
                    current: Some("172.16.128.0-17-172.18.192.2-0-1".into()),
                    weight: 10.0,
                    path: "network/r1/staticroutes".into(),
                    status: Status::Failed,
                    explanation: "Static route next hop mismatch".into(),
                    evidence: vec![],
                },
                Checkpoint {
                    command_classification: Default::default(),
                    id: "chk_ospf".into(),
                    device: "Router2".into(),
                    device_type: "Router".into(),
                    interface: "GigabitEthernet0/0".into(),
                    category: "Routing".into(),
                    property: "ospfnetwork".into(),
                    expected: "10.0.0.0 0.0.0.3 0".into(),
                    current: None,
                    weight: 10.0,
                    path: "network/r2/ospf".into(),
                    status: Status::Unverified,
                    explanation: "OSPF network statement not verified".into(),
                    evidence: vec![],
                },
            ],
            ..Default::default()
        };
        app.analysis = Some(analysis);

        // Test narrow responsive width (250px)
        app.expanded_checkpoint_ids
            .insert("chk_static".into(), true);
        let narrow_output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800., 600.),
                )),
                ..Default::default()
            },
            |ctx| {
                app.render_detail_panel(ctx);
                app.render_notification(ctx);
            },
        );
        assert!(!narrow_output.shapes.is_empty());

        // Test wide responsive width (500px)
        app.expanded_checkpoint_ids.clear();
        app.expanded_checkpoint_ids.insert("chk_ospf".into(), true);
        let wide_output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600., 1000.),
                )),
                ..Default::default()
            },
            |ctx| {
                app.render_detail_panel(ctx);
                app.render_notification(ctx);
            },
        );
        assert!(!wide_output.shapes.is_empty());
    }

    #[test]
    fn test_cisco_device_filtering_and_basic_inclusion() {
        let specs = make_test_cisco_specs();
        let mut app = InspectorApp::new(None, None);

        // Filter: All -> all 3 specs visible
        app.library_device_filter = CiscoDeviceFilter::All;
        // Direct filter logic test
        let router_specs: Vec<_> = specs
            .iter()
            .filter(|spec| {
                spec.id.starts_with("router.")
                    || spec.id.starts_with("basic.")
                    || spec.capability == Capability::Basic
                    || spec.capability == Capability::Routing
                    || spec.capability == Capability::DynamicRouting
                    || spec.capability == Capability::Ipv6Routing
            })
            .collect();
        // Router filter includes router.interface_ip AND basic.hostname
        assert_eq!(router_specs.len(), 2);
        assert!(router_specs.iter().any(|s| s.id == "router.interface_ip"));
        assert!(router_specs.iter().any(|s| s.id == "basic.hostname"));

        let switch_specs: Vec<_> = specs
            .iter()
            .filter(|spec| {
                spec.id.starts_with("switch.")
                    || spec.id.starts_with("basic.")
                    || spec.capability == Capability::Basic
                    || spec.capability == Capability::Switching
                    || spec.capability == Capability::Multilayer
            })
            .collect();
        // Switch filter includes switch.vlan_create AND basic.hostname
        assert_eq!(switch_specs.len(), 2);
        assert!(switch_specs.iter().any(|s| s.id == "switch.vlan_create"));
        assert!(switch_specs.iter().any(|s| s.id == "basic.hostname"));
    }

    #[test]
    fn test_cisco_search_query_bilingual_and_syntax() {
        let specs = make_test_cisco_specs();

        // Search by English name
        let q_en = "address";
        let match_en: Vec<_> = specs
            .iter()
            .filter(|s| {
                s.name_en.to_lowercase().contains(q_en) || s.name_vi.to_lowercase().contains(q_en)
            })
            .collect();
        assert_eq!(match_en.len(), 1);
        assert_eq!(match_en[0].id, "router.interface_ip");

        // Search by Vietnamese name
        let q_vi = "giao tiếp";
        let match_vi: Vec<_> = specs
            .iter()
            .filter(|s| {
                s.parameters
                    .iter()
                    .any(|p| p.description_vi.to_lowercase().contains(q_vi))
            })
            .collect();
        assert_eq!(match_vi.len(), 1);

        // Search by syntax
        let q_syn = "vlan <vlan_id>";
        let match_syn: Vec<_> = specs.iter().filter(|s| s.syntax.contains(q_syn)).collect();
        assert_eq!(match_syn.len(), 1);
        assert_eq!(match_syn[0].id, "switch.vlan_create");
    }

    #[test]
    fn test_cisco_support_badge_states() {
        let catalog = Localizer::new();
        // VerifiedPacketTracer must be strictly distinguished
        let _s_pt = SupportState::VerifiedPacketTracer;
        assert_eq!(
            catalog.text("en", "cisco.support.verified_pt"),
            "Packet Tracer Verified"
        );
        assert_eq!(
            catalog.text("vi", "cisco.support.verified_pt"),
            "Đã xác minh Packet Tracer"
        );

        // DocumentedNotTested must NEVER claim PT verified
        let _s_doc = SupportState::DocumentedNotTested;
        let doc_en = catalog.text("en", "cisco.support.documented_not_tested");
        assert!(!doc_en.contains("Packet Tracer Verified"));
        assert!(doc_en.contains("Not PT Tested"));

        // Unsupported
        let unsup_en = catalog.text("en", "cisco.support.unsupported");
        assert!(unsup_en.contains("Unsupported"));

        // Unknown
        let unk_en = catalog.text("en", "cisco.support.unknown");
        assert!(unk_en.contains("Unknown"));
    }

    #[test]
    fn test_cisco_library_rendering_and_interaction() {
        let context = egui::Context::default();
        let mut app = InspectorApp::new(None, None);
        app.current_tab = Tab::CommandLibrary;
        let specs = make_test_cisco_specs();

        // Test rendering with English locale
        app.preferences.language = "en".into();
        let output_en = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 800.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.render_command_library_content(ui, ctx, &specs);
                });
            },
        );
        assert!(!output_en.shapes.is_empty());

        // Test "Fill safe examples"
        assert!(app.library_selected_id.is_some());
        let selected_id = app.library_selected_id.clone().unwrap();
        let spec = specs.iter().find(|s| s.id == selected_id).unwrap();
        app.library_parameters = cisco::example_request(spec, app.library_profile).parameters;
        assert!(!app.library_parameters.is_empty());

        // Test rendering with Vietnamese locale
        app.preferences.language = "vi".into();
        let output_vi = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 800.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.render_command_library_content(ui, ctx, &specs);
                });
            },
        );
        assert!(!output_vi.shapes.is_empty());

        // Test rendering with empty specs (empty registry state)
        let output_empty = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 800.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.render_command_library_content(ui, ctx, &[]);
                });
            },
        );
        assert!(!output_empty.shapes.is_empty());
    }

    #[test]
    fn search_switching_families_clears_stale_fields_without_filling_examples() {
        let context = egui::Context::default();
        let mut app = InspectorApp::new(None, None);
        let specs = cisco::registry();
        app.library_selected_id = Some("basic.hostname".into());
        app.library_parameters
            .insert("hostname".into(), "Synthetic-Device".into());
        app.library_search_query = "ntp".into();
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 820.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.render_command_library_content(ui, ctx, &specs);
                });
            },
        );
        assert_eq!(app.library_selected_id.as_deref(), Some("basic.ntp_server"));
        assert!(!app.library_parameters.contains_key("hostname"));
        assert!(
            app.library_parameters
                .values()
                .all(|value| value.is_empty())
        );
        app.library_parameters
            .insert("server".into(), "2001:db8::123".into());
        app.select_library_family("basic.ntp_server");
        assert_eq!(
            app.library_parameters.get("server").unwrap(),
            "2001:db8::123"
        );
        app.select_library_family("router.rip.version");
        assert!(app.library_parameters.is_empty());
    }

    #[test]
    fn test_cisco_library_layout_proportions_at_960x600_and_1280x820() {
        for (width, height, lang) in [
            (960.0, 600.0, "en"),
            (960.0, 600.0, "vi"),
            (1280.0, 820.0, "en"),
            (1280.0, 820.0, "vi"),
        ] {
            let context = egui::Context::default();
            let mut app = InspectorApp::new(None, None);
            app.preferences.language = lang.into();
            app.current_tab = Tab::CommandLibrary;

            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, height),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    app.render_top_panel(ctx);
                    app.render_bottom_panel(ctx);
                    app.render_central_panel(ctx);
                },
            );

            // Verify shapes were generated
            assert!(
                !output.shapes.is_empty(),
                "Failed to generate shapes for {width}x{height} in {lang}"
            );

            // Verify library layout height is allocated to full remaining height, NOT collapsed to a tiny strip (<50px)
            let min_expected_height = height * 0.55;
            println!(
                "Layout metrics for {}x{} ({}): height={}, left_w={}, right_w={}",
                width,
                height,
                lang,
                app.library_layout_height,
                app.library_left_width,
                app.library_right_width
            );
            assert!(
                app.library_layout_height >= min_expected_height,
                "Library height for {width}x{height} ({lang}) collapsed to {}px, expected at least {}px",
                app.library_layout_height,
                min_expected_height
            );

            // Verify left pane width is preserved as 340px
            assert!(
                (app.library_left_width - 340.0).abs() < 1.0,
                "Left list pane width for {width}x{height} should be 340px, got {}px",
                app.library_left_width
            );

            // Verify right detail pane width has ample space
            let min_right_width = width - 340.0 - 100.0;
            assert!(
                app.library_right_width >= min_right_width,
                "Right detail pane width for {width}x{height} should be >= {}px, got {}px",
                min_right_width,
                app.library_right_width
            );

            // Verify visual bounding box of rendered shapes spans the full vertical content area
            let visual_rect = output.shapes.iter().fold(egui::Rect::NOTHING, |acc, s| {
                acc.union(s.shape.visual_bounding_rect())
            });
            assert!(
                visual_rect.height() >= height * 0.8,
                "Visual rendered content height should cover the window, got {}px on {height}px window",
                visual_rect.height()
            );
        }
    }

    #[test]
    fn test_cisco_library_with_synthetic_126_and_tab_switching() {
        let context = egui::Context::default();
        let mut app = InspectorApp::new(None, None);
        let analysis = inspector::parse(
            include_str!("../tests/fixtures/synthetic-126.xml").trim_start_matches('\u{feff}'),
        )
        .unwrap();
        app.analysis = Some(analysis);

        // Switch to CommandLibrary tab with synthetic activity loaded
        app.current_tab = Tab::CommandLibrary;

        for lang in ["en", "vi"] {
            app.preferences.language = lang.into();
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 820.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    app.render_top_panel(ctx);
                    app.render_bottom_panel(ctx);
                    app.render_central_panel(ctx);
                },
            );

            assert!(!output.shapes.is_empty());
            assert!(
                app.library_layout_height >= 600.0,
                "Command library layout with synthetic126 should be full-height, got {}",
                app.library_layout_height
            );
            assert_eq!(app.library_left_width, 340.0);
        }
    }

    #[cfg(test)]
    fn make_test_cisco_specs() -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "router.interface_ip",
                name_en: "Interface IPv4 Address",
                name_vi: "Địa chỉ IPv4 cho cổng",
                topic: "interface",
                syntax: "ip address <ip> <mask>",
                mode: "interface",
                capability: Capability::Routing,
                parameters: vec![
                    cisco::ParameterSpec {
                        name: "interface",
                        kind: cisco::ParameterKind::Interface,
                        required: true,
                        description_en: "Interface name",
                        description_vi: "Tên cổng giao tiếp",
                        example: "GigabitEthernet0/0/0",
                    },
                    cisco::ParameterSpec {
                        name: "address",
                        kind: cisco::ParameterKind::Ipv4,
                        required: true,
                        description_en: "IPv4 address",
                        description_vi: "Địa chỉ IPv4",
                        example: "192.168.1.1",
                    },
                    cisco::ParameterSpec {
                        name: "mask",
                        kind: cisco::ParameterKind::Ipv4,
                        required: true,
                        description_en: "Subnet mask",
                        description_vi: "Mặt nạ mạng",
                        example: "255.255.255.0",
                    },
                ],
                prerequisites: vec!["interface GigabitEthernet0/0/0"],
                verification: vec!["show ip interface brief"],
                source: "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/ipaddr/command/ipaddr-cr-book.html",
                software: "Cisco IOS 15.0+",
                removal: Some("no ip address"),
                semantic_validation: true,
            },
            CommandSpec {
                id: "switch.vlan_create",
                name_en: "Create VLAN",
                name_vi: "Tạo VLAN",
                topic: "vlan",
                syntax: "vlan <vlan_id>",
                mode: "vlan",
                capability: Capability::Switching,
                parameters: vec![
                    cisco::ParameterSpec {
                        name: "vlan_id",
                        kind: cisco::ParameterKind::Number(1, 4094),
                        required: true,
                        description_en: "VLAN ID",
                        description_vi: "Mã số VLAN",
                        example: "10",
                    },
                    cisco::ParameterSpec {
                        name: "name",
                        kind: cisco::ParameterKind::Identifier,
                        required: false,
                        description_en: "VLAN Name",
                        description_vi: "Tên VLAN",
                        example: "MANAGEMENT",
                    },
                ],
                prerequisites: vec![],
                verification: vec!["show vlan brief"],
                source: "Cisco CCNA 200-301 Official Cert Guide",
                software: "Catalyst IOS",
                removal: Some("no vlan"),
                semantic_validation: true,
            },
            CommandSpec {
                id: "basic.hostname",
                name_en: "Set Device Hostname",
                name_vi: "Đặt tên thiết bị",
                topic: "basic",
                syntax: "hostname <name>",
                mode: "global",
                capability: Capability::Basic,
                parameters: vec![cisco::ParameterSpec {
                    name: "hostname",
                    kind: cisco::ParameterKind::Identifier,
                    required: true,
                    description_en: "Device Hostname",
                    description_vi: "Tên thiết bị",
                    example: "R1-CORE",
                }],
                prerequisites: vec![],
                verification: vec!["show running-config | include hostname"],
                source: "Cisco IOS Basic Configuration Guide",
                software: "Cisco IOS",
                removal: Some("no hostname"),
                semantic_validation: true,
            },
        ]
    }
}
