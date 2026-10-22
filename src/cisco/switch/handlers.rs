use crate::cisco::{
    CommandRequest, CommandSpec, ConfigMode, DeviceProfile, ModeCommand, ParameterKind as K, at,
};
use std::collections::BTreeSet;
use std::net::Ipv4Addr;

fn int(req: &CommandRequest, key: &str) -> Result<u32, String> {
    let s = req.required(key)?;
    if !s.bytes().all(|c| c.is_ascii_digit()) {
        return Err(format!("{key} must be an unsigned decimal number"));
    }
    s.parse().map_err(|_| format!("Invalid number: {key}"))
}
fn eq(req: &CommandRequest, key: &str, value: &str) -> Result<(), String> {
    if req.required(key)? == value {
        Ok(())
    } else {
        Err(format!("{key} must be {value}; context cannot be inferred"))
    }
}
fn physical(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    let Some(tail) = [
        "fastethernet",
        "gigabitethernet",
        "tengigabitethernet",
        "fa",
        "gi",
        "te",
    ]
    .iter()
    .find_map(|prefix| lower.strip_prefix(prefix)) else {
        return false;
    };
    let tail = tail.trim_start();
    let parts: Vec<_> = tail.split('/').collect();
    (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|v| !v.is_empty() && v.len() <= 3 && v.bytes().all(|b| b.is_ascii_digit()))
        && parts
            .last()
            .and_then(|v| v.parse::<u16>().ok())
            .is_some_and(|n| n > 0)
}
/// Compare lists by membership without accepting free-form CLI keywords.
fn vlan_set(s: &str) -> Result<BTreeSet<u16>, String> {
    if s.is_empty() || s.len() > 512 {
        return Err("Empty or oversized VLAN list".into());
    }
    let mut set = BTreeSet::new();
    for part in s.split(',') {
        let ends: Vec<_> = part.split('-').collect();
        if !(1..=2).contains(&ends.len()) {
            return Err("Malformed VLAN range".into());
        }
        let parse = |v: &str| -> Result<u16, String> {
            if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                return Err("VLAN lists contain decimal IDs and ranges only".into());
            }
            v.parse::<u16>()
                .ok()
                .filter(|n| (1..=4094).contains(n))
                .ok_or_else(|| "VLAN must be in 1..4094".into())
        };
        let start = parse(ends[0])?;
        let end = if ends.len() == 2 {
            parse(ends[1])?
        } else {
            start
        };
        if start > end {
            return Err("Descending VLAN range".into());
        }
        for n in start..=end {
            if !set.insert(n) {
                return Err("Duplicate or overlapping VLAN IDs".into());
            }
        }
    }
    Ok(set)
}
fn mac(s: &str) -> bool {
    let parts: Vec<_> = s.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|v| v.len() != 4 || !v.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return false;
    }
    let flat = parts.concat();
    u8::from_str_radix(&flat[..2], 16).is_ok_and(|b| b & 1 == 0) && flat.bytes().any(|b| b != b'0')
}
fn validate(req: &CommandRequest, spec: &CommandSpec) -> Result<(), String> {
    for (key, raw) in &req.parameters {
        if raw != raw.trim() || raw.chars().any(char::is_control) {
            return Err(format!("Unsafe whitespace in {key}"));
        }
        let Some(field) = spec.parameters.iter().find(|p| p.name == key) else {
            return Err(format!("Unknown parameter: {key}"));
        };
        if raw.is_empty() {
            if field.required {
                return Err(format!("Missing required parameter: {key}"));
            } else {
                continue;
            }
        }
        let valid = match field.kind {
            K::Number(lo, hi) => int(req, key).is_ok_and(|n| (lo..=hi).contains(&n)),
            K::Choice(values) => values.contains(&raw.as_str()),
            K::Interface => {
                if req.family == "switch.svi.ipv4" {
                    svi(raw).is_some()
                } else {
                    physical(raw)
                }
            }
            K::Ipv4 => raw.parse::<Ipv4Addr>().is_ok(),
            K::Identifier => crate::cisco::identifier(raw),
            K::VlanList => vlan_set(raw).is_ok(),
            _ => false,
        };
        if !valid {
            return Err(format!("Invalid typed parameter: {key}"));
        }
    }
    for p in &spec.parameters {
        if p.required {
            req.required(p.name)?;
        }
    }
    Ok(())
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    let spec = crate::cisco::catalog()
        .iter()
        .find(|s| s.id == req.family)
        .ok_or("Unknown switch family")?;
    validate(req, spec)?;
    let global = |s: String| vec![at(ConfigMode::Global, s)];
    let iface = |s: String| vec![at(ConfigMode::Interface(req.get("interface").into()), s)];
    match req.family.as_str() {
        "switch.vlan" => {
            if req.get("name").len() > 32
                || !req
                    .get("name")
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            {
                return Err("VLAN name must be 1..32 letters, digits, underscore or hyphen".into());
            }
            let n = int(req, "vlan")? as u16;
            // A global vlan command enters config-vlan itself; the renderer also
            // enters ConfigMode::Vlan, so a single typed name command suffices.
            Ok(vec![at(
                ConfigMode::Vlan(n),
                format!("name {}", req.get("name")),
            )])
        }
        "switch.port_mode" => {
            eq(req, "current_group", "0")?;
            let mut commands = vec![];
            if req.get("mode") == "trunk" {
                match req.profile {
                    DeviceProfile::Catalyst3560=>commands.push(at(ConfigMode::Interface(req.get("interface").into()),"switchport trunk encapsulation dot1q")),
                    DeviceProfile::Catalyst2960=>{},
                    _=>return Err("Trunk setup requires an identified Catalyst2960 or Catalyst3560 encapsulation profile".into()),
                }
            }
            commands.extend(iface(format!("switchport mode {}", req.get("mode"))));
            Ok(commands)
        }
        "switch.access_vlan" | "switch.voice_vlan" => {
            eq(req, "interface_mode", "access")?;
            let vlan = int(req, "vlan")?;
            if (1002..=1005).contains(&vlan) {
                return Err("Access and voice VLANs cannot use reserved VLANs 1002-1005".into());
            }
            if req.family == "switch.voice_vlan" && vlan == 1 {
                return Err("Voice VLAN cannot be VLAN 1".into());
            }
            Ok(iface(format!(
                "switchport {} vlan {}",
                if req.family == "switch.access_vlan" {
                    "access"
                } else {
                    "voice"
                },
                vlan
            )))
        }
        "switch.trunk_native" => {
            eq(req, "interface_mode", "trunk")?;
            if int(req, "vlan")? != int(req, "peer_native_vlan")? {
                return Err("Native VLAN mismatch with peer".into());
            }
            Ok(iface(format!(
                "switchport trunk native vlan {}",
                req.get("vlan")
            )))
        }
        "switch.trunk_allowed" => {
            eq(req, "interface_mode", "trunk")?;
            let op = req.get("operation");
            let list = req.get("vlans");
            let suffix = if ["all", "none"].contains(&op) {
                if !list.is_empty() {
                    return Err("all/none must not include a VLAN list".into());
                }
                op.to_string()
            } else {
                vlan_set(req.required("vlans")?)?;
                if op == "replace" {
                    list.into()
                } else {
                    format!("{op} {list}")
                }
            };
            Ok(iface(format!("switchport trunk allowed vlan {suffix}")))
        }
        "switch.stp_mode" => Ok(global(format!("spanning-tree mode {}", req.get("mode")))),
        "switch.stp_root" => Ok(global(format!(
            "spanning-tree vlan {} root {}",
            req.get("vlans"),
            req.get("role")
        ))),
        "switch.stp_priority" => {
            if int(req, "priority")? % 4096 != 0 {
                return Err("Bridge priority must be a multiple of 4096".into());
            }
            Ok(global(format!(
                "spanning-tree vlan {} priority {}",
                req.get("vlans"),
                req.get("priority")
            )))
        }
        "switch.stp_portfast" | "switch.stp_bpduguard" => {
            eq(req, "interface_mode", "access")?;
            Ok(iface(
                if req.family == "switch.stp_portfast" {
                    "spanning-tree portfast"
                } else {
                    "spanning-tree bpduguard enable"
                }
                .into(),
            ))
        }
        "switch.stp_rootguard" => Ok(iface("spanning-tree guard root".into())),
        "switch.stp_cost" => Ok(iface(format!("spanning-tree cost {}", req.get("cost")))),
        "switch.stp_port_priority" => {
            if int(req, "priority")? % 16 != 0 {
                return Err("Port priority must be a multiple of 16".into());
            }
            Ok(iface(format!(
                "spanning-tree port-priority {}",
                req.get("priority")
            )))
        }
        "switch.svi.ipv4" => {
            if !matches!(
                req.profile,
                DeviceProfile::Catalyst2960 | DeviceProfile::Catalyst3560
            ) {
                return Err("SVI setup requires identified Catalyst2960 or Catalyst3560".into());
            }
            let vlan = int(req, "vlan")?;
            if (1002..=1005).contains(&vlan) {
                return Err("SVI cannot be configured on reserved VLANs 1002-1005".into());
            }
            if svi(req.get("interface")) != Some(vlan as u16) {
                return Err("SVI interface and VLAN number must match".into());
            }
            ipv4_host(req, "address")?;
            let mode = ConfigMode::Interface(format!("Vlan{}", vlan));
            Ok(vec![
                at(
                    mode.clone(),
                    format!("ip address {} {}", req.get("address"), req.get("mask")),
                ),
                at(mode, "no shutdown"),
            ])
        }
        "switch.default_gateway" => {
            let (address, mask) = ipv4_host(req, "address")?;
            let (gateway, _) = ipv4_host(req, "gateway")?;
            if address == gateway || address & mask != gateway & mask {
                return Err("Gateway must be a different host in the management subnet".into());
            }
            Ok(global(format!("ip default-gateway {}", req.get("gateway"))))
        }
        "switch.routed_port" => {
            multilayer(req)?;
            eq(req, "current_group", "0")?;
            ipv4_host(req, "address")?;
            let mode = ConfigMode::Interface(req.get("interface").into());
            let mut result = vec![];
            if req.get("current_mode") != "routed" {
                result.push(at(mode.clone(), "no switchport"));
            }
            result.push(at(
                mode.clone(),
                format!("ip address {} {}", req.get("address"), req.get("mask")),
            ));
            result.push(at(mode, "no shutdown"));
            Ok(result)
        }
        "switch.ip_routing" => {
            multilayer(req)?;
            Ok(global("ip routing".into()))
        }
        "switch.port_security" => security(req),
        "switch.etherchannel" => etherchannel(req),
        _ => Err("Unknown switch family".into()),
    }
}
fn svi(s: &str) -> Option<u16> {
    let lower = s.to_ascii_lowercase();
    let tail = lower
        .strip_prefix("vlan")
        .or_else(|| lower.strip_prefix("vl"))?
        .trim_start();
    if tail.is_empty() || !tail.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    tail.parse::<u16>().ok().filter(|n| (1..=4094).contains(n))
}
fn multilayer(req: &CommandRequest) -> Result<(), String> {
    if req.profile != DeviceProfile::Catalyst3560 {
        return Err("This Layer 3 switch operation requires Catalyst3560 with a confirmed IP base or IP services image".into());
    }
    Ok(())
}
/// Deliberate /1../30 host-address subset; /31 and /32 need separate semantics.
fn ipv4_host(req: &CommandRequest, key: &str) -> Result<(u32, u32), String> {
    let ip = req
        .required(key)?
        .parse::<Ipv4Addr>()
        .map_err(|_| format!("Invalid IPv4 {key}"))?;
    let mask = u32::from(
        req.required("mask")?
            .parse::<Ipv4Addr>()
            .map_err(|_| "Invalid IPv4 mask")?,
    );
    let prefix = mask.leading_ones();
    if !(1..=30).contains(&prefix) || mask != u32::MAX << (32 - prefix) {
        return Err("Mask must be contiguous /1 through /30".into());
    }
    let address = u32::from(ip);
    let host = address & !mask;
    if ip.octets()[0] == 0
        || ip.octets()[0] == 127
        || ip.octets()[0] >= 224
        || host == 0
        || host == !mask
    {
        return Err(format!("{key} must be a unicast subnet host address"));
    }
    Ok((address, mask))
}
fn security(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    eq(req, "interface_mode", "access")?;
    eq(req, "current_group", "0")?;
    let max = int(req, "maximum")?;
    if max < int(req, "secure_count")? || max > int(req, "available_capacity")? {
        return Err("Secure maximum conflicts with existing count or available capacity".into());
    }
    let static_mac = req.get("static_mac");
    if !static_mac.is_empty() && (!mac(static_mac) || int(req, "secure_count")? == 0) {
        return Err(
            "Static MAC must be nonzero unicast xxxx.xxxx.xxxx and included in secure_count".into(),
        );
    }
    let aging = req.get("aging_minutes");
    if aging.is_empty()
        && (!req.get("aging_type").is_empty() || !req.get("aging_static").is_empty())
    {
        return Err("Aging settings require aging_minutes".into());
    }
    if !aging.is_empty() {
        req.required("aging_type")?;
        req.required("aging_static")?;
        if int(req, "aging_minutes")? > 0 && req.get("sticky") == "enabled" {
            return Err("Sticky secure MAC addresses do not support aging".into());
        }
    }
    let mut text = vec![
        "switchport port-security".to_string(),
        format!("switchport port-security maximum {max}"),
        format!(
            "switchport port-security violation {}",
            req.get("violation")
        ),
        if req.get("sticky") == "enabled" {
            "switchport port-security mac-address sticky"
        } else {
            "no switchport port-security mac-address sticky"
        }
        .into(),
    ];
    if !static_mac.is_empty() {
        text.push(format!(
            "switchport port-security mac-address {}",
            static_mac.to_ascii_lowercase()
        ));
    }
    if !aging.is_empty() {
        text.push(format!("switchport port-security aging time {aging}"));
        text.push(format!(
            "switchport port-security aging type {}",
            req.get("aging_type")
        ));
        text.push(
            if req.get("aging_static") == "enabled" {
                "switchport port-security aging static"
            } else {
                "no switchport port-security aging static"
            }
            .into(),
        );
    }
    Ok(text
        .into_iter()
        .map(|s| at(ConfigMode::Interface(req.get("interface").into()), s))
        .collect())
}
fn etherchannel(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    let lower = req.get("interface").to_ascii_lowercase();
    let port_type = if lower.starts_with("fa") {
        "fastethernet"
    } else if lower.starts_with("gi") {
        "gigabitethernet"
    } else {
        return Err(
            "Portable EtherChannel subset supports FastEthernet or GigabitEthernet members".into(),
        );
    };
    if port_type != req.get("group_port_type") {
        return Err("EtherChannel physical port type mismatch".into());
    }
    if port_type == "fastethernet" && req.get("speed") == "1000" {
        return Err("FastEthernet cannot operate at 1000 Mbps".into());
    }
    let group = int(req, "group")?;
    if int(req, "current_group")? != 0 && int(req, "current_group")? != group {
        return Err("Interface already belongs to another group".into());
    }
    for (a, b) in [
        ("interface_mode", "group_interface_mode"),
        ("speed", "group_speed"),
        ("duplex", "group_duplex"),
        ("mode", "group_mode"),
    ] {
        if req.get(a) != req.get(b) {
            return Err(format!("EtherChannel mismatch: {a} versus {b}"));
        }
    }
    let compatible = matches!(
        (req.get("mode"), req.get("peer_mode")),
        ("active", "active" | "passive")
            | ("passive", "active")
            | ("desirable", "desirable" | "auto")
            | ("auto", "desirable")
            | ("on", "on")
    );
    if !compatible {
        return Err("Peer modes cannot form this EtherChannel".into());
    }
    if req.get("interface_mode") == "access" {
        if int(req, "access_vlan")? != int(req, "group_access_vlan")? {
            return Err("Access VLAN mismatch".into());
        }
        if [
            "native_vlan",
            "group_native_vlan",
            "allowed_vlans",
            "group_allowed_vlans",
        ]
        .iter()
        .any(|k| !req.get(k).is_empty())
        {
            return Err("Trunk metadata supplied for access EtherChannel".into());
        }
    } else {
        if int(req, "native_vlan")? != int(req, "group_native_vlan")? {
            return Err("Native VLAN mismatch".into());
        }
        if vlan_set(req.required("allowed_vlans")?)?
            != vlan_set(req.required("group_allowed_vlans")?)?
        {
            return Err("Allowed VLAN mismatch".into());
        }
        if ["access_vlan", "group_access_vlan"]
            .iter()
            .any(|k| !req.get(k).is_empty())
        {
            return Err("Access metadata supplied for trunk EtherChannel".into());
        }
    }
    Ok(vec![at(
        ConfigMode::Interface(req.get("interface").into()),
        format!("channel-group {group} mode {}", req.get("mode")),
    )])
}
