//! Assessment mapping preserves requirements; absent context remains missing, never a default.
use super::{CommandRequest, DeviceProfile};
use crate::model::Checkpoint;
pub fn profile(c: &Checkpoint) -> DeviceProfile {
    let models: std::collections::BTreeSet<_> = c
        .evidence
        .iter()
        .filter_map(|s| s.strip_prefix("assessment:device-model="))
        .collect();
    let model = if models.len() == 1 {
        *models.first().unwrap()
    } else {
        c.device_type.as_str()
    };
    DeviceProfile::from_evidence(model, &c.device_type)
}
fn key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
pub fn request(c: &Checkpoint, resolved: &str) -> Option<CommandRequest> {
    let property = key(&c.property);
    let t: Vec<_> = resolved.split_whitespace().collect();
    let mut req = CommandRequest {
        device: c.device.clone(),
        profile: profile(c),
        ..Default::default()
    };
    let mut put = |key: &str, value: &str| {
        if !value.is_empty() {
            req.parameters.insert(key.into(), value.into());
        }
    };
    let family = match property.as_str() {
        "ipaddress" | "ipv4address" | "subnetmask" => {
            put("interface", &c.interface);
            if t.len() == 2 {
                put("address", t[0]);
                put("mask", t[1]);
            }
            "router.interface.ipv4"
        }
        "ipv6address" => {
            put("interface", &c.interface);
            put("address", resolved);
            "router.interface.ipv6"
        }
        "ipv6linklocal" | "linklocaladdress" => {
            put("interface", &c.interface);
            put("address", resolved);
            "router.interface.link_local"
        }
        "interfacedescription" | "description" if !c.interface.is_empty() => {
            put("interface", &c.interface);
            put("description", resolved);
            "router.interface.description"
        }
        "accessvlan" => {
            put("interface", &c.interface);
            put("vlan", resolved);
            "switch.access_vlan"
        }
        "voicevlan" => {
            put("interface", &c.interface);
            put("vlan", resolved);
            "switch.voice_vlan"
        }
        "nativevlan" => {
            put("interface", &c.interface);
            put("vlan", resolved);
            "switch.trunk_native"
        }
        "allowedvlans" => {
            put("interface", &c.interface);
            put("vlans", &t.join(","));
            "switch.trunk_allowed"
        }
        "switchportmode" => {
            put("interface", &c.interface);
            put("mode", resolved);
            "switch.port_mode"
        }
        "vlan" => {
            if let Some(id) = t.first() {
                put("vlan", id);
            }
            if t.len() == 2 {
                put("name", t[1]);
            }
            "switch.vlan"
        }
        "stpmode" | "spanningtreemode" => {
            put("mode", resolved);
            "switch.stp_mode"
        }
        "portsecurity" | "portsecuritymaximum" | "portsecurityviolation" | "portsecuritysticky" => {
            put("interface", &c.interface);
            "switch.port_security"
        }
        _ if crate::commands::is_static_route_property(&c.property, &c.path) => {
            put("route", resolved);
            "router.route.ipv4"
        }
        _ if crate::commands::is_ipv6_static_route_property(&c.property, &c.path) => {
            put("route", resolved);
            "router.route.ipv6"
        }
        _ if crate::commands::is_ospf_network_property(&c.property, &c.path) => {
            if let Ok(o) = crate::network_values::ParsedOspfNetwork::parse(resolved) {
                if let Some(pid) = o.process_id {
                    put("process_id", &pid.to_string());
                }
                if let Some(prefix) = crate::network_values::wildcard_to_prefix(o.wildcard_mask) {
                    put("network", &format!("{}/{prefix}", o.network));
                }
                if let Some(area) = o.area {
                    put("area", &area);
                }
            }
            "router.ospf.network"
        }
        _ => return None,
    };
    req.family = family.into();
    // Only unambiguous named metadata can complete context. Values are never inferred
    // from an unrelated sibling or unlabelled network snapshot.
    let spec = super::registry().into_iter().find(|s| s.id == family);
    if let Some(spec) = spec {
        for field in spec.parameters {
            if req.parameters.contains_key(field.name) {
                continue;
            }
            let prefix = format!("commands:parameter:{}=", field.name);
            let values: std::collections::BTreeSet<_> = c
                .evidence
                .iter()
                .filter_map(|s| s.strip_prefix(&prefix))
                .collect();
            if values.len() == 1 {
                req.parameters
                    .insert(field.name.into(), (*values.first().unwrap()).into());
            }
        }
    }
    Some(req)
}
#[derive(Debug, Default)]
pub struct CheckpointCoverage {
    pub total: usize,
    pub recognized: usize,
    pub command_ready: usize,
    pub missing_parameters: usize,
    pub validation_verified: usize,
    pub unknown: usize,
}
pub fn coverage(checkpoints: &[Checkpoint]) -> CheckpointCoverage {
    let mut n = CheckpointCoverage {
        total: checkpoints.len(),
        ..Default::default()
    };
    for c in checkpoints {
        if c.status != crate::model::Status::Unverified {
            n.validation_verified += 1;
        }
        if let Some(req) = request(c, &c.expected) {
            n.recognized += 1;
            let p = super::generate(&req);
            if p.executable() {
                n.command_ready += 1;
            }
            if p.classification == super::Classification::MissingParameters {
                n.missing_parameters += 1;
            }
        } else {
            n.unknown += 1;
        }
    }
    n
}
