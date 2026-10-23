//! Field-specific assessment encodings. Callers must identify a known Boolean field.
use super::*;

pub(crate) fn boolean(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "enabled" | "on" => Some(true),
        "0" | "false" | "disabled" | "off" => Some(false),
        _ => None,
    }
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
