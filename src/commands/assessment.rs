//! Field-specific assessment encodings. Callers must identify a known Boolean field.
use super::*;

pub(crate) fn boolean(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "enabled" | "on" => Some(true),
        "0" | "false" | "disabled" | "off" => Some(false),
        _ => None,
    }
}

/// Resolve a generic leaf only from one protocol category below its unique device.
/// Numeric tree IDs never become process IDs or autonomous-system numbers.
pub(crate) fn scoped_property(c: &Checkpoint, property: &str) -> Option<String> {
    let parts: Vec<_> = c.path.split(" / ").collect();
    let devices: Vec<_> = parts
        .iter()
        .enumerate()
        .filter(|(_, p)| **p == c.device)
        .map(|(i, _)| i)
        .collect();
    if c.device.is_empty() || devices.len() != 1 || devices[0] + 1 >= parts.len() {
        return None;
    }
    let parents: Vec<_> = parts[devices[0] + 1..parts.len() - 1]
        .iter()
        .map(|p| key(p))
        .collect();
    let protocols: std::collections::BTreeSet<_> = parents
        .iter()
        .filter_map(|p| match p.as_str() {
            "ospf" | "ospfv2" => Some("ospf"),
            "rip" => Some("rip"),
            "eigrp" => Some("eigrp"),
            "ntp" => Some("ntp"),
            "lldp" => Some("lldp"),
            "ospfv3" | "ripv6" | "eigrpv6" => Some("unsupported-ipv6"),
            _ => None,
        })
        .collect();
    if protocols.len() != 1 {
        return None;
    }
    let protocol = *protocols.first()?;
    if matches!(protocol, "ospf" | "rip" | "eigrp") && parents.iter().any(|p| p == "ipv6") {
        return None;
    }
    let name = match (protocol, property) {
        ("ospf", "routerid" | "configuredrouterid") => "ospfrouterid",
        ("ospf", "referencebandwidth") => "ospfreferencebandwidth",
        ("ospf", "passiveinterface") => "ospfpassiveinterface",
        ("eigrp", "routerid" | "configuredrouterid") => "eigrprouterid",
        ("eigrp", "passiveinterface") => "eigrppassiveinterface",
        ("rip", "version") => "ripversion",
        ("rip", "passiveinterface") => "rippassiveinterface",
        ("rip", "autosummary") => "ripautosummary",
        ("ntp", "server") => "ntpserver",
        ("ntp", "sourceinterface") => "ntpsource",
        ("lldp", "enabled") if c.interface.is_empty() => "lldpglobal",
        ("lldp", "transmit") => "lldptransmit",
        ("lldp", "receive") => "lldpreceive",
        ("lldp", "holdtime") => "lldpholdtime",
        ("lldp", "reinit") => "lldpreinit",
        ("lldp", "timer") => "lldptimer",
        _ => return None,
    };
    Some(name.into())
}

/// Identity of a port-scoped dot1Q requirement, not a VLAN inferred from its suffix.
pub(crate) fn dot1q_scope(c: &Checkpoint) -> Option<String> {
    let parts: Vec<_> = c.path.split(" / ").collect();
    let positions: Vec<_> = parts
        .iter()
        .enumerate()
        .filter(|(_, part)| part.eq_ignore_ascii_case(&c.interface))
        .map(|(i, _)| i)
        .collect();
    if c.device.is_empty() || c.interface.is_empty() || positions.len() != 1 {
        return None;
    }
    let port = positions[0];
    let tail: Vec<_> = parts[port + 1..].iter().map(|part| key(part)).collect();
    let recognized = match property_key(c).as_str() {
        // The final path component is a schema ID, not necessarily the display name.
        "encapsulation" if c.expected == "EthernetEncapProcess" => tail.len() == 1,
        "vlanid" => {
            (tail.len() == 2 && tail[0] == "8021q")
                || (tail.len() == 3 && tail[0] == "encapsulation" && tail[1] == "8021q")
        }
        _ => false,
    };
    recognized.then(|| parts[..=port].join(" / "))
}
