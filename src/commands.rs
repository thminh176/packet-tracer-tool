//! Conservative, offline CLI suggestions. Complex values use the explicit grammars
//! documented in `generate`; unknown assessment encodings are never CLI values.
use crate::model::{Checkpoint, Command};
use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, Ipv6Addr},
};

#[derive(Default)]
struct Plan {
    context: Vec<String>,
    lines: Vec<String>,
    save: bool,
}
fn key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/:".contains(c))
}
fn safe(s: &str) -> bool {
    s.len() <= 1024
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || " ._-/:=".contains(c))
}
fn number(s: &str, min: u32, max: u32) -> bool {
    s.parse::<u32>().is_ok_and(|n| n >= min && n <= max)
}
fn ip(s: &str) -> bool {
    s.parse::<Ipv4Addr>().is_ok()
}
fn mask(s: &str) -> bool {
    s.parse::<Ipv4Addr>().is_ok_and(|v| {
        let n = u32::from(v);
        let inverse = !n;
        inverse & inverse.wrapping_add(1) == 0
    })
}
fn wildcard(s: &str) -> bool {
    s.parse::<Ipv4Addr>().is_ok_and(|v| {
        let n = u32::from(v);
        n & n.wrapping_add(1) == 0
    })
}
fn network(address: &str, subnet: &str) -> bool {
    match (address.parse::<Ipv4Addr>(), subnet.parse::<Ipv4Addr>()) {
        (Ok(a), Ok(m)) => mask(subnet) && u32::from(a) & !u32::from(m) == 0,
        _ => false,
    }
}
fn rip_network(s: &str) -> bool {
    match s.parse::<Ipv4Addr>() {
        Ok(a) => match a.octets()[0] {
            1..=126 => network(s, "255.0.0.0"),
            128..=191 => network(s, "255.255.0.0"),
            192..=223 => network(s, "255.255.255.0"),
            _ => false,
        },
        Err(_) => false,
    }
}
fn prefix6(s: &str) -> bool {
    s.split_once('/')
        .is_some_and(|(a, p)| a.parse::<Ipv6Addr>().is_ok() && number(p, 0, 128))
}
fn iface(s: &str) -> bool {
    token(s)
        && s.chars().any(|c| c.is_ascii_digit())
        && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && [
            "ethernet",
            "fastethernet",
            "gigabitethernet",
            "tengigabitethernet",
            "serial",
            "loopback",
            "vlan",
            "port-channel",
            "fa",
            "gi",
            "eth",
            "se",
            "lo",
            "te",
        ]
        .iter()
        .any(|p| {
            s.to_ascii_lowercase().strip_prefix(p).is_some_and(|tail| {
                tail.starts_with(|c: char| c.is_ascii_digit())
                    && tail
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '/' || c == '.')
            })
        })
}
fn enabled(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "enabled" | "on" | "up" => Some(true),
        "false" | "disabled" | "off" | "down" => Some(false),
        _ => None,
    }
}
fn identifier(s: &str) -> bool {
    token(s) && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && !s.contains(['/', ':'])
}
fn mac(s: &str) -> bool {
    let p: Vec<_> = s.split('.').collect();
    p.len() == 3
        && p.iter()
            .all(|v| v.len() == 4 && v.chars().all(|c| c.is_ascii_hexdigit()))
}
fn unsupported(reason: &str) -> Command {
    Command {
        explanation: reason.into(),
        ..Command::default()
    }
}

// The inspector may attach a uniquely matched explicit sibling requirement.
// Evidence is never interpreted as raw CLI or mined for arbitrary numbers.
fn explicit_value(c: &Checkpoint) -> Result<String, &'static str> {
    if c.evidence.iter().any(|e| {
        [
            "assessment:unsupported",
            "assessment:ambiguous",
            "assessment:inventory",
        ]
        .iter()
        .any(|prefix| e.starts_with(prefix))
    }) {
        return Err("Unsupported assessment check type; no reliable configuration requirement.");
    }
    let property = key(&c.property);
    let marker = match property.as_str() {
        "ipaddress" | "ipv4address" => "commands:ipv4-mask=",
        "ipv6address" => "commands:ipv6-prefix=",
        _ => return Ok(c.expected.clone()),
    };
    let evidence: Vec<_> = c
        .evidence
        .iter()
        .filter_map(|e| e.strip_prefix(marker))
        .collect();
    if evidence.is_empty() {
        return Ok(c.expected.clone());
    }
    if evidence.iter().any(|v| *v != evidence[0]) {
        return Err("Conflicting explicit sibling mask or prefix evidence.");
    }
    let extra = evidence[0];
    if !safe(extra) || ["-1", "65535", "4294967295"].contains(&extra) {
        return Err("Invalid or sentinel sibling mask/prefix evidence.");
    }
    if marker == "commands:ipv4-mask=" {
        if !mask(extra) {
            return Err("Sibling subnet mask is not a contiguous IPv4 mask.");
        }
        let t: Vec<_> = c.expected.split_whitespace().collect();
        if t.len() == 1 && ip(t[0]) {
            return Ok(format!("{} {extra}", t[0]));
        }
        if t.len() != 2 || t[1] != extra {
            return Err("IP requirement conflicts with explicit sibling subnet mask.");
        }
    } else {
        if !number(extra, 0, 128) {
            return Err("Sibling IPv6 prefix is not in range 0..128.");
        }
        if c.expected.parse::<Ipv6Addr>().is_ok() {
            return Ok(format!("{}/{extra}", c.expected));
        }
        if !c.expected.split_once('/').is_some_and(|(_, p)| p == extra) {
            return Err("IPv6 requirement conflicts with explicit sibling prefix.");
        }
    }
    Ok(c.expected.clone())
}

fn plan(c: &Checkpoint) -> Result<Plan, &'static str> {
    if !safe(&c.expected)
        || !safe(&c.interface)
        || !safe(&c.device)
        || !safe(&c.property)
        || !safe(&c.category)
    {
        return Err(
            "Unsafe characters or control characters in assessment data; no commands generated.",
        );
    }
    let explicit = explicit_value(c)?;
    let value = explicit.trim();
    if value.is_empty()
        || [
            "unknown",
            "none",
            "null",
            "unset",
            "n/a",
            "-1",
            "65535",
            "4294967295",
            "18446744073709551615",
        ]
        .contains(&value.to_ascii_lowercase().as_str())
    {
        return Err(
            "Missing, unknown, or sentinel expected value; explicit configuration is required.",
        );
    }
    let p = key(&c.property);
    let t: Vec<&str> = value.split_whitespace().collect();
    if t.iter()
        .any(|v| ["-1", "65535", "4294967295", "18446744073709551615"].contains(v))
    {
        return Err("Sentinel numeric value in explicit configuration; no commands generated.");
    }
    let mut out = Plan::default();
    let interface_property = matches!(
        p.as_str(),
        "portstatus"
            | "interfacestatus"
            | "administrativestatus"
            | "shutdown"
            | "ipv4address"
            | "ipaddress"
            | "ipv6address"
            | "ipv6linklocal"
            | "linklocaladdress"
            | "switchportmode"
            | "accessvlan"
            | "nativevlan"
            | "allowedvlans"
            | "portsecurity"
            | "portsecuritymaximum"
            | "portsecurityviolation"
            | "portsecuritymac"
            | "portsecuritysticky"
            | "natrole"
            | "aclattachment"
    );
    if interface_property {
        if !iface(c.interface.trim()) {
            return Err("A specific, valid interface name is required.");
        }
        out.context
            .push(format!("interface {}", c.interface.trim()));
    }
    let line = match p.as_str() {
        "hostname" if identifier(value) && value.len() <= 63 => format!("hostname {value}"),
        "portstatus" | "interfacestatus" | "administrativestatus" => match enabled(value) {
            Some(true) => "no shutdown".into(),
            Some(false) => "shutdown".into(),
            _ => {
                return Err(
                    "Interface state must explicitly be up/down or enabled/disabled; numeric encodings are ambiguous.",
                );
            }
        },
        "shutdown" => match enabled(value) {
            Some(true) => "shutdown".into(),
            Some(false) => "no shutdown".into(),
            _ => return Err("Shutdown must explicitly be true or false."),
        },
        "ipaddress" | "ipv4address" if t.len() == 2 && ip(t[0]) && mask(t[1]) => {
            format!("ip address {} {}", t[0], t[1])
        }
        "ipv6address" if t.len() == 1 && prefix6(t[0]) => format!("ipv6 address {value}"),
        "ipv6linklocal" | "linklocaladdress"
            if t.len() == 1
                && t[0]
                    .parse::<Ipv6Addr>()
                    .is_ok_and(|a| a.segments()[0] & 0xffc0 == 0xfe80) =>
        {
            format!("ipv6 address {value} link-local")
        }
        "defaultgateway" | "gateway" if t.len() == 1 && ip(value) => {
            format!("ip default-gateway {value}")
        }
        "staticroute" | "ipv4staticroute"
            if (t.len() == 3 || t.len() == 4)
                && network(t[0], t[1])
                && (ip(t[2]) || iface(t[2]))
                && (t.len() == 3 || number(t[3], 1, 254)) =>
        {
            format!("ip route {}", t.join(" "))
        }
        "ipv6staticroute"
            if (t.len() == 2 || t.len() == 3)
                && prefix6(t[0])
                && t[1]
                    .parse::<Ipv6Addr>()
                    .is_ok_and(|a| a.segments()[0] & 0xffc0 != 0xfe80)
                && (t.len() == 2 || number(t[2], 1, 254)) =>
        {
            format!("ipv6 route {}", t.join(" "))
        }
        "vlan"
            if (t.len() == 1 || t.len() == 2)
                && number(t[0], 1, 4094)
                && (t.len() == 1 || identifier(t[1])) =>
        {
            out.context.push(format!("vlan {}", t[0]));
            if t.len() == 2 {
                format!("name {}", t[1])
            } else {
                String::new()
            }
        }
        "switchportmode" if ["access", "trunk"].contains(&value) => {
            format!("switchport mode {value}")
        }
        "accessvlan" if number(value, 1, 4094) => format!("switchport access vlan {value}"),
        "nativevlan" if number(value, 1, 4094) => format!("switchport trunk native vlan {value}"),
        "allowedvlans" if t.iter().all(|v| number(v, 1, 4094)) => {
            format!("switchport trunk allowed vlan {}", t.join(","))
        }
        "portsecurity" if enabled(value) == Some(true) => "switchport port-security".into(),
        "portsecuritymaximum" if number(value, 1, 132) => {
            format!("switchport port-security maximum {value}")
        }
        "portsecurityviolation" if ["protect", "restrict", "shutdown"].contains(&value) => {
            format!("switchport port-security violation {value}")
        }
        "portsecuritymac" if mac(value) => format!("switchport port-security mac-address {value}"),
        "portsecuritysticky" if enabled(value) == Some(true) => {
            "switchport port-security mac-address sticky".into()
        }
        "natrole" if ["inside", "outside"].contains(&value) => format!("ip nat {value}"),
        "natstatic" if t.len() == 2 && ip(t[0]) && ip(t[1]) => {
            format!("ip nat inside source static {} {}", t[0], t[1])
        }
        "natoverload" if t.len() == 2 && number(t[0], 1, 99) && iface(t[1]) => format!(
            "ip nat inside source list {} interface {} overload",
            t[0], t[1]
        ),
        "aclattachment"
            if t.len() == 2
                && (number(t[0], 1, 199) || identifier(t[0]))
                && ["in", "out"].contains(&t[1]) =>
        {
            format!("ip access-group {} {}", t[0], t[1])
        }
        "standardacl"
            if t.len() == 4
                && number(t[0], 1, 99)
                && ["permit", "deny"].contains(&t[1])
                && ip(t[2])
                && wildcard(t[3]) =>
        {
            format!("access-list {}", t.join(" "))
        }
        "extendedacl"
            if t.len() == 7
                && number(t[0], 100, 199)
                && ["permit", "deny"].contains(&t[1])
                && ["ip", "tcp", "udp", "icmp"].contains(&t[2])
                && ip(t[3])
                && wildcard(t[4])
                && ip(t[5])
                && wildcard(t[6]) =>
        {
            format!("access-list {}", t.join(" "))
        }
        "dhcppool"
            if (t.len() == 5 || t.len() == 6)
                && identifier(t[0])
                && network(t[1], t[2])
                && ip(t[3])
                && ip(t[4])
                && (t.len() == 5 || identifier(t[5])) =>
        {
            out.context.push(format!("ip dhcp pool {}", t[0]));
            out.lines.push(format!("network {} {}", t[1], t[2]));
            out.lines.push(format!("default-router {}", t[3]));
            out.lines.push(format!("dns-server {}", t[4]));
            if t.len() == 6 {
                format!("domain-name {}", t[5])
            } else {
                String::new()
            }
        }
        "dhcpexcludedaddress" if (t.len() == 1 || t.len() == 2) && t.iter().all(|v| ip(v)) => {
            format!("ip dhcp excluded-address {}", t.join(" "))
        }
        "ospfnetwork"
            if t.len() == 4
                && number(t[0], 1, 65534)
                && ip(t[1])
                && wildcard(t[2])
                && (number(t[3], 0, 4294967294) || ip(t[3])) =>
        {
            out.context.push(format!("router ospf {}", t[0]));
            format!("network {} {} area {}", t[1], t[2], t[3])
        }
        "ripnetwork" if t.len() == 2 && ["1", "2"].contains(&t[0]) && rip_network(t[1]) => {
            out.context.push("router rip".into());
            out.lines.push(format!("version {}", t[0]));
            format!("network {}", t[1])
        }
        "eigrpnetwork" if t.len() == 3 && number(t[0], 1, 65534) && ip(t[1]) && wildcard(t[2]) => {
            out.context.push(format!("router eigrp {}", t[0]));
            format!("network {} {}", t[1], t[2])
        }
        "enablesecret" if token(value) => format!("enable secret {value}"),
        "passwordencryption" if enabled(value) == Some(true) => {
            "service password-encryption".into()
        }
        "consolepassword" if token(value) => {
            out.context.push("line console 0".into());
            out.lines.push(format!("password {value}"));
            "login".into()
        }
        "vtypassword"
            if t.len() == 3
                && number(t[0], 0, 15)
                && number(t[1], 0, 15)
                && t[0].parse::<u32>().unwrap() <= t[1].parse::<u32>().unwrap()
                && token(t[2]) =>
        {
            out.context.push(format!("line vty {} {}", t[0], t[1]));
            out.lines.push(format!("password {}", t[2]));
            "login".into()
        }
        "vtyssh"
            if t.len() == 2
                && number(t[0], 0, 15)
                && number(t[1], 0, 15)
                && t[0].parse::<u32>().unwrap() <= t[1].parse::<u32>().unwrap() =>
        {
            out.context.push(format!("line vty {} {}", t[0], t[1]));
            out.lines.push("login local".into());
            "transport input ssh".into()
        }
        "localuser" if t.len() == 3 && identifier(t[0]) && number(t[1], 0, 15) && token(t[2]) => {
            format!("username {} privilege {} secret {}", t[0], t[1], t[2])
        }
        "save" | "saveconfiguration" if enabled(value) == Some(true) => {
            out.save = true;
            String::new()
        }
        _ => {
            return Err(
                "Unsupported or incomplete explicit data for this property. Supply a complete documented value; ambiguous numeric encodings and separate sibling fields are not inferred.",
            );
        }
    };
    if !line.is_empty() {
        out.lines.push(line);
    }
    Ok(out)
}

fn is_gui(c: &Checkpoint) -> bool {
    matches!(
        key(&c.device_type).as_str(),
        "pc" | "pcpt" | "server" | "serverpt" | "laptop" | "laptoppt"
    )
}

/// Generate a self-contained suggestion for one checkpoint, never execute it.
/// Values are explicit whitespace-separated fields: IPv4 `address mask`, IPv6
/// `address/prefix`, static route `network mask next-hop [distance]`, VLAN `id
/// [name]`, standard ACL `id action source wildcard`, extended ACL `id action
/// protocol source wildcard destination wildcard`, NAT static `local global`,
/// NAT overload `acl interface`, DHCP `pool network mask gateway dns [domain]`,
/// OSPF `process network wildcard area`, RIP `version network`, EIGRP `asn
/// network wildcard`, VTY password `first last password`, VTY SSH `first last`,
/// local user `name privilege secret`. Only explicit `commands:ipv4-mask=` or
/// `commands:ipv6-prefix=` evidence may complete a single address leaf.
pub fn generate(checkpoint: &Checkpoint) -> Command {
    let resolved = match explicit_value(checkpoint) {
        Ok(v) => v,
        Err(e) => return unsupported(e),
    };
    // Validate even GUI values before displaying them in guidance.
    if let Err(reason) = plan(checkpoint)
        && (!is_gui(checkpoint)
            || !safe(&checkpoint.expected)
            || !safe(&checkpoint.device)
            || !safe(&checkpoint.interface)
            || !safe(&checkpoint.property)
            || !safe(&checkpoint.category))
    {
        return unsupported(reason);
    }
    if is_gui(checkpoint) {
        let property = key(&checkpoint.property);
        let valid = match property.as_str() {
            "ipv4address" | "ipaddress" => {
                let t: Vec<_> = resolved.split_whitespace().collect();
                t.len() == 2 && ip(t[0]) && mask(t[1])
            }
            "defaultgateway" | "gateway" | "dnsserver" => {
                ip(checkpoint.expected.trim())
                    || checkpoint.expected.trim().parse::<Ipv6Addr>().is_ok()
            }
            "subnetmask" => mask(checkpoint.expected.trim()),
            "ipv6address" => prefix6(resolved.trim()),
            _ => false,
        };
        return if valid {
            Command {
                supported: true,
                gui: true,
                text: format!(
                    "Desktop > IP Configuration: set {} to {}.",
                    checkpoint.property, resolved
                ),
                explanation: "Use the PC/server GUI; IOS commands do not apply to this endpoint."
                    .into(),
            }
        } else {
            unsupported(
                "Endpoint configuration is GUI-only; this property needs explicit supported IP configuration data.",
            )
        };
    }
    match plan(checkpoint) {
        Ok(p) => Command { supported:true, gui:false, text:render(&[p]), explanation:"Review the explicit values and apply on the named device. No commands have been executed.".into() },
        Err(e) => unsupported(e),
    }
}

fn render(plans: &[Plan]) -> String {
    let mut text = String::from("enable\n");
    if plans.iter().any(|p| !p.save) {
        text.push_str("configure terminal\n");
        for p in plans.iter().filter(|p| !p.save) {
            for context in &p.context {
                text.push_str(context);
                text.push('\n');
            }
            for line in &p.lines {
                text.push_str(line);
                text.push('\n');
            }
            for _ in &p.context {
                text.push_str("exit\n");
            }
        }
        text.push_str("end\n");
    }
    if plans.iter().any(|p| p.save) {
        text.push_str("copy running-config startup-config\n");
    }
    text
}

fn interface_order(p: &Plan) -> u8 {
    if !p
        .context
        .first()
        .is_some_and(|c| c.starts_with("interface "))
    {
        return 0;
    }
    match p.lines.first().map(String::as_str).unwrap_or("") {
        "shutdown" => 0,
        line if line.starts_with("switchport mode ") => 1,
        line if line.starts_with("switchport access vlan ")
            || line.starts_with("switchport trunk ") =>
        {
            2
        }
        "switchport port-security" => 3,
        "no shutdown" => 5,
        _ => 4,
    }
}

fn conflict_key(c: &Checkpoint, p: &Plan) -> Option<String> {
    let mut property = key(&c.property);
    let t: Vec<_> = c.expected.split_whitespace().collect();
    match property.as_str() {
        "eigrpnetwork" | "dhcpexcludedaddress" | "save" | "saveconfiguration" => return None,
        "staticroute" | "ipv4staticroute" => {
            property = format!("route:{} {}", t.first()?, t.get(1)?)
        }
        "ipv6staticroute" => property = format!("route6:{}", t.first()?),
        "standardacl" | "extendedacl" => {
            property = format!("acl:{}:{}", t.first()?, t.get(2..)?.join(" "))
        }
        "natstatic" => property = format!("natstatic:{}", t.first()?),
        "ospfnetwork" => property = format!("ospf:{} {}", t.get(1)?, t.get(2)?),
        "portstatus" | "interfacestatus" | "administrativestatus" | "shutdown" => {
            property = "interface-state".into()
        }
        "ipaddress" | "ipv4address" => property = "ipv4address".into(),
        "gateway" | "defaultgateway" => property = "gateway".into(),
        _ => (),
    }
    if property == "localuser" {
        property.push_str(c.expected.split_whitespace().next().unwrap_or(""));
    }
    Some(format!("{}:{property}", p.context.join("/")))
}

fn conflict_value(c: &Checkpoint, p: &Plan) -> String {
    if key(&c.property) == "ripnetwork" {
        p.lines.first().cloned().unwrap_or_default()
    } else {
        p.lines.join("\n")
    }
}

fn conflict_plan(c: &Checkpoint) -> Option<Plan> {
    let command = generate(c);
    if !command.supported {
        return None;
    }
    if command.gui {
        Some(Plan {
            context: vec![format!("endpoint {}", c.interface)],
            lines: vec![command.text],
            save: false,
        })
    } else {
        plan(c).ok()
    }
}

/// Group independently scoped CLI blocks per device, with VLAN definitions
/// before interface configuration and explicit saves at the end. GUI/unsupported
/// checkpoints appear as comments, never as executable configuration.
pub fn device_script(checkpoints: &[Checkpoint], device: Option<&str>) -> String {
    let mut groups: BTreeMap<&str, Vec<&Checkpoint>> = BTreeMap::new();
    for c in checkpoints {
        if device.is_none_or(|d| c.device == d) {
            groups.entry(&c.device).or_default().push(c);
        }
    }
    let mut output = String::new();
    for (name, items) in groups {
        if !safe(name) || name.trim().is_empty() {
            output.push_str("! Skipped checkpoint group: missing or unsafe device name.\n");
            continue;
        }
        output.push_str(&format!("! Device: {name}\n"));
        let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for c in &items {
            if let Some(p) = conflict_plan(c)
                && let Some(k) = conflict_key(c, &p)
            {
                let seen = values.entry(k).or_default();
                if !seen.contains(&conflict_value(c, &p)) {
                    seen.push(conflict_value(c, &p));
                }
            }
        }
        let mut plans = Vec::new();
        for c in items {
            if conflict_plan(c)
                .and_then(|p| conflict_key(c, &p))
                .is_some_and(|k| values.get(&k).is_some_and(|v| v.len() > 1))
            {
                output.push_str(
                    "! Skipped conflicting explicit requirements in the same device/context.\n",
                );
                continue;
            }
            let command = generate(c);
            if command.supported && !command.gui {
                if let Ok(p) = plan(c) {
                    plans.push(p);
                }
            } else {
                output.push_str(&format!(
                    "! {}\n",
                    if command.gui {
                        command.text
                    } else {
                        command.explanation
                    }
                ));
            }
        }
        // Preserve ACL entry order; establish interface mode before port security,
        // and bring interfaces up only after their explicit settings are applied.
        plans.sort_by_key(|p| {
            (
                if p.context.first().is_some_and(|v| v.starts_with("vlan ")) {
                    0
                } else if p.context.is_empty() {
                    1
                } else {
                    2
                },
                p.context.join(" "),
                interface_order(p),
            )
        });
        if !plans.is_empty() {
            output.push_str(&render(&plans));
        }
        output.push('\n');
    }
    output
}
