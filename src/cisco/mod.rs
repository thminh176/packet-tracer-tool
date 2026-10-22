//! Typed, offline Cisco command knowledge. Documentation is not Packet Tracer execution evidence.
use std::collections::BTreeMap;
pub mod device_profile;
pub mod parser;
pub mod router;
pub mod switch;
pub use device_profile::{Capability, DeviceProfile, SupportState};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigMode {
    Global,
    Interface(String),
    Router(String),
    Vlan(u16),
    Acl(String),
    DhcpPool(String),
    Line(String),
    Exec,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModeCommand {
    pub mode: ConfigMode,
    pub text: String,
}
pub fn at(mode: ConfigMode, text: impl Into<String>) -> ModeCommand {
    ModeCommand {
        mode,
        text: text.into(),
    }
}
#[derive(Debug, Clone)]
pub enum ParameterKind {
    Identifier,
    Interface,
    Ipv4,
    Ipv6,
    Number(u32, u32),
    Choice(&'static [&'static str]),
    Text,
    Ipv4Network,
    Ipv6Prefix,
    VlanList,
}
#[derive(Debug, Clone)]
pub struct ParameterSpec {
    pub name: &'static str,
    pub kind: ParameterKind,
    pub required: bool,
    pub description_en: &'static str,
    pub description_vi: &'static str,
    pub example: &'static str,
}
#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub id: &'static str,
    pub name_en: &'static str,
    pub name_vi: &'static str,
    pub topic: &'static str,
    pub syntax: &'static str,
    pub mode: &'static str,
    pub capability: Capability,
    pub parameters: Vec<ParameterSpec>,
    pub prerequisites: Vec<&'static str>,
    pub verification: Vec<&'static str>,
    pub source: &'static str,
    pub software: &'static str,
    pub removal: Option<&'static str>,
    pub semantic_validation: bool,
}
#[derive(Debug, Clone, Default)]
pub struct CommandRequest {
    pub family: String,
    pub device: String,
    pub profile: DeviceProfile,
    pub parameters: BTreeMap<String, String>,
}
impl CommandRequest {
    pub fn get(&self, key: &str) -> &str {
        self.parameters
            .get(key)
            .map(String::as_str)
            .unwrap_or("")
            .trim()
    }
    pub fn required(&self, key: &str) -> Result<&str, String> {
        let v = self.get(key);
        if v.is_empty() {
            Err(format!("Missing required parameter: {key}"))
        } else {
            Ok(v)
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    Ready,
    MissingParameters,
    RecognizedUnsupported,
    UnknownAmbiguous,
}
#[derive(Debug, Clone)]
pub struct CommandPlan {
    pub target: String,
    pub profile: DeviceProfile,
    pub family: String,
    pub classification: Classification,
    pub commands: Vec<ModeCommand>,
    pub missing_parameters: Vec<String>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
    pub evidence: Vec<String>,
    pub verification: Vec<String>,
    pub support: SupportState,
}
impl CommandPlan {
    pub fn executable(&self) -> bool {
        self.classification == Classification::Ready
            && self.conflicts.is_empty()
            && !self.commands.is_empty()
    }
    pub fn script(&self) -> String {
        if !self.executable() {
            return String::new();
        }
        render(&self.commands)
    }
}
pub fn registry() -> Vec<CommandSpec> {
    let mut v = switch::specs();
    v.extend(router::specs());
    v
}
pub fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/:".contains(c))
}
pub fn safe_text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 512 && s.chars().all(|c| !c.is_control() && !";!|<>".contains(c))
}
pub fn generate(req: &CommandRequest) -> CommandPlan {
    let mut p = CommandPlan {
        target: req.device.clone(),
        profile: req.profile,
        family: req.family.clone(),
        classification: Classification::UnknownAmbiguous,
        commands: vec![],
        missing_parameters: vec![],
        conflicts: vec![],
        warnings: vec![],
        evidence: vec![],
        verification: vec![],
        support: SupportState::Unknown,
    };
    let specs = registry();
    let Some(spec) = specs.iter().find(|s| s.id == req.family) else {
        return p;
    };
    p.support = req.profile.support(spec.capability);
    p.evidence.push(spec.source.into());
    p.verification = spec.verification.iter().map(|s| s.to_string()).collect();
    if p.support == SupportState::Unsupported {
        p.classification = Classification::RecognizedUnsupported;
        p.warnings
            .push("Unsupported on the identified device profile".into());
        return p;
    }
    for field in &spec.parameters {
        if field.required && req.get(field.name).is_empty() {
            p.missing_parameters.push(field.name.into());
        }
    }
    if !p.missing_parameters.is_empty() {
        p.classification = Classification::MissingParameters;
        return p;
    }
    if !safe_text(&req.device)
        || req
            .parameters
            .values()
            .any(|v| !v.is_empty() && !safe_text(v))
    {
        p.warnings.push("Unsafe or oversized parameter".into());
        return p;
    }
    if p.support != SupportState::VerifiedPacketTracer {
        p.warnings.push("Not execution-tested in Packet Tracer; verify platform/software compatibility before applying".into());
    }
    let result = if req.family.starts_with("switch.") {
        switch::generate(req)
    } else {
        router::generate(req)
    };
    match result {
        Ok(commands) => {
            p.commands = commands;
            p.classification = Classification::Ready;
        }
        Err(reason) => {
            p.warnings.push(reason);
        }
    }
    p
}
pub fn render(commands: &[ModeCommand]) -> String {
    let mut out = String::from("enable\nconfigure terminal\n");
    let mut current = ConfigMode::Global;
    for command in commands {
        if command.mode == ConfigMode::Exec {
            if current != ConfigMode::Exec {
                out.push_str("end\n");
            }
            current = ConfigMode::Exec;
        } else if command.mode != current {
            if current == ConfigMode::Exec {
                out.push_str("configure terminal\n");
            } else if current != ConfigMode::Global {
                out.push_str("exit\n");
            }
            let header = match &command.mode {
                ConfigMode::Global => None,
                ConfigMode::Interface(v) => Some(format!("interface {v}")),
                ConfigMode::Router(v) => Some(format!("router {v}")),
                ConfigMode::Vlan(v) => Some(format!("vlan {v}")),
                ConfigMode::Acl(v) => Some(v.clone()),
                ConfigMode::DhcpPool(v) => Some(format!("ip dhcp pool {v}")),
                ConfigMode::Line(v) => Some(format!("line {v}")),
                ConfigMode::Exec => None,
            };
            if let Some(h) = header {
                out.push_str(&h);
                out.push('\n');
            }
            current = command.mode.clone();
        }
        out.push_str(&command.text);
        out.push('\n');
    }
    if current != ConfigMode::Exec {
        out.push_str("end\n");
    }
    out
}
