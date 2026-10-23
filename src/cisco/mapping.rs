//! Assessment mapping preserves requirements; absent context remains missing, never a default.
use super::{CommandRequest, DeviceProfile};
use crate::model::Checkpoint;
pub fn identity_conflict(c: &Checkpoint) -> bool {
    let models: std::collections::BTreeSet<_> = c
        .evidence
        .iter()
        .filter_map(|e| e.strip_prefix("assessment:device-model="))
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();
    if models.len() > 1 {
        return true;
    }
    let p = profile(c);
    matches!(
        (c.device_type.to_ascii_lowercase().as_str(), p),
        (
            "router",
            DeviceProfile::Catalyst2960 | DeviceProfile::Catalyst3560
        ) | (
            "switch",
            DeviceProfile::Router1941 | DeviceProfile::Router2911 | DeviceProfile::Isr4321
        )
    )
}
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
pub fn request(c: &Checkpoint, resolved: &str) -> Option<CommandRequest> {
    let property = crate::commands::property_key(c);
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
        "defaultgateway" | "gateway" => {
            put("gateway", resolved);
            "switch.default_gateway"
        }
        "hostname" => {
            put("hostname", resolved);
            "basic.hostname"
        }
        "enablesecret" => {
            put("secret", resolved);
            "basic.enable_secret"
        }
        "bannermotd" => {
            put("message", resolved);
            "basic.banner_motd"
        }
        "vtyssh" => {
            if t.len() == 2 {
                put("first", t[0]);
                put("last", t[1]);
            }
            "basic.vty_ssh"
        }
        "save" | "saveconfiguration" if matches!(resolved, "true" | "yes" | "startup") => {
            "basic.save"
        }
        "ipaddress" | "ipv4address" | "subnetmask" => {
            put("interface", &c.interface);
            if t.len() == 2 {
                put("address", t[0]);
                put("mask", t[1]);
            }
            if matches!(
                req.profile,
                DeviceProfile::Catalyst2960 | DeviceProfile::Catalyst3560
            ) && c.interface.to_ascii_lowercase().starts_with("vlan")
            {
                if let Some(vlan) = c.interface.get(4..) {
                    put("vlan", vlan);
                }
                "switch.svi.ipv4"
            } else {
                "router.interface.ipv4"
            }
        }
        "ipv6address" | "ipv6prefixlength" => {
            put("interface", &c.interface);
            put("address", resolved);
            "router.interface.ipv6"
        }
        "ipv6linklocal" | "linklocaladdress" => {
            put("interface", &c.interface);
            put("address", resolved);
            "router.interface.link_local"
        }
        "ospfrouterid" => {
            put("router_id", resolved);
            "router.ospf.router_id"
        }
        "ospfreferencebandwidth" => {
            put("mbps", resolved);
            "router.ospf.reference_bandwidth"
        }
        "ospfpassiveinterface" if !c.interface.is_empty() => {
            put("interface", &c.interface);
            put("state", resolved);
            "router.ospf.passive"
        }
        "iphelperaddress" | "dhcprelayaddress" if !c.interface.is_empty() => {
            put("interface", &c.interface);
            put("server", resolved);
            "router.dhcp.relay"
        }
        "clockrate" if !c.interface.is_empty() => {
            put("interface", &c.interface);
            put("bps", resolved);
            "router.serial.clock"
        }
        "encapsulation" if resolved.eq_ignore_ascii_case("ppp") && !c.interface.is_empty() => {
            put("interface", &c.interface);
            "router.serial.ppp"
        }
        "cdpglobal" if c.interface.is_empty() => {
            put("enabled", resolved);
            "basic.cdp_global"
        }
        "cdpenable" if !c.interface.is_empty() => {
            put("interface", &c.interface);
            put("enabled", resolved);
            "basic.cdp_interface"
        }
        "ipv6unicastrouting" if c.interface.is_empty() => {
            put("enabled", resolved);
            "router.ipv6.routing"
        }
        "ipv6enable" if !c.interface.is_empty() => {
            put("interface", &c.interface);
            put("enabled", resolved);
            "router.interface.ipv6_enable"
        }
        "ipv6autoconfig" if !c.interface.is_empty() => {
            put("interface", &c.interface);
            put("enabled", resolved);
            "router.interface.ipv6_autoconfig"
        }
        "vlanid"
            if c.path
                .split(" / ")
                .any(|part| part.eq_ignore_ascii_case("802.1Q")) =>
        {
            put("interface", &c.interface);
            put("vlan", resolved);
            "router.interface.dot1q"
        }
        "encapsulation" if resolved == "EthernetEncapProcess" => {
            put("interface", &c.interface);
            "router.interface.dot1q"
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
        "vlanname" => {
            if let Some(vlan) = crate::commands::checkpoint_vlan(c) {
                put("vlan", &vlan.to_string());
            }
            put("name", resolved);
            "switch.vlan"
        }
        "stpmode" | "spanningtreemode" => {
            put("mode", resolved);
            "switch.stp_mode"
        }
        "portsecurity" | "portsecuritymaximum" | "portsecurityviolation" | "portsecuritysticky" => {
            put("interface", &c.interface);
            match property.as_str() {
                "portsecuritymaximum" => put("maximum", resolved),
                "portsecurityviolation" => put("violation", resolved),
                "portsecuritysticky" => put(
                    "sticky",
                    match resolved {
                        "true" | "enabled" => "enabled",
                        "false" | "disabled" => "disabled",
                        _ => resolved,
                    },
                ),
                _ => (),
            }
            "switch.port_security"
        }
        "portsecuritymac" => {
            put("interface", &c.interface);
            put("static_mac", resolved);
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
    let spec = super::catalog().iter().find(|s| s.id == family);
    if let Some(spec) = spec {
        for field in &spec.parameters {
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
/// Repeated or contradictory named evidence must not disappear during completion.
pub fn parameter_conflict(c: &Checkpoint, resolved: &str) -> bool {
    let Some(req) = request(c, resolved) else {
        return false;
    };
    let Some(spec) = super::catalog().iter().find(|s| s.id == req.family) else {
        return false;
    };
    spec.parameters.iter().any(|field| {
        let prefix = format!("commands:parameter:{}=", field.name);
        let values: std::collections::BTreeSet<_> = c
            .evidence
            .iter()
            .filter_map(|s| s.strip_prefix(&prefix))
            .collect();
        values.len() > 1
            || values.first().is_some_and(|value| {
                req.parameters
                    .get(field.name)
                    .is_some_and(|required| required.as_str() != *value)
            })
    })
}
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointCoverage {
    pub total: usize,
    pub recognized: usize,
    pub command_ready: usize,
    pub gui_ready: usize,
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
        let resolved = crate::commands::explicit_value(c).unwrap_or_else(|_| c.expected.clone());
        let command = crate::commands::generate(c);
        if command.supported {
            if command.gui {
                n.gui_ready += 1;
            } else {
                n.command_ready += 1;
            }
        }
        if let Some(req) = request(c, &resolved) {
            n.recognized += 1;
            let p = super::generate(&req);
            if !command.supported && p.classification == super::Classification::MissingParameters {
                n.missing_parameters += 1;
            }
        } else {
            if command.supported {
                n.recognized += 1;
            } else {
                n.unknown += 1;
            }
        }
    }
    n
}
/// Command readiness is distinct from automatic verification status and native grading.
pub fn classify(c: &Checkpoint) -> super::Classification {
    if c.evidence.iter().any(|e| {
        matches!(
            e.as_str(),
            "assessment:unsupported" | "assessment:ambiguous" | "assessment:inventory"
        )
    }) || identity_conflict(c)
    {
        return super::Classification::UnknownAmbiguous;
    }
    if crate::commands::generate(c).supported {
        return super::Classification::Ready;
    }
    let resolved = crate::commands::explicit_value(c).unwrap_or_else(|_| c.expected.clone());
    if let Some(req) = request(c, &resolved) {
        let classification = super::generate(&req).classification;
        return if classification == super::Classification::Ready {
            super::Classification::RecognizedUnsupported
        } else {
            classification
        };
    }
    super::Classification::UnknownAmbiguous
}
