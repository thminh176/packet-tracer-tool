//! Conservative mode-aware parsing of saved IOS configuration, never execution.
use super::{ConfigMode, ModeCommand};
#[derive(Debug, Default)]
pub struct ParsedConfig {
    pub entries: Vec<ModeCommand>,
    pub reliable: bool,
    pub diagnostics: Vec<String>,
}
impl ParsedConfig {
    pub fn in_mode(&self, mode: &ConfigMode) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|e| &e.mode == mode)
            .map(|e| e.text.as_str())
            .collect()
    }
}
pub fn interface_name(value: &str) -> Option<String> {
    let value = value.trim();
    let first = value.find(|c: char| c.is_ascii_digit())?;
    let (prefix, number) = value.split_at(first);
    if number.is_empty()
        || !number
            .chars()
            .all(|c| c.is_ascii_digit() || "/.:".contains(c))
    {
        return None;
    }
    let name = match prefix.to_ascii_lowercase().as_str() {
        "gi" | "gigabitethernet" => "GigabitEthernet",
        "fa" | "fastethernet" => "FastEthernet",
        "se" | "serial" => "Serial",
        "lo" | "loopback" => "Loopback",
        "vl" | "vlan" => "Vlan",
        "po" | "port-channel" => "Port-channel",
        "tu" | "tunnel" => "Tunnel",
        _ => return None,
    };
    Some(format!("{name}{number}"))
}
pub fn parse(input: &str) -> ParsedConfig {
    let mut p = ParsedConfig {
        reliable: true,
        ..Default::default()
    };
    if input.len() > 4 * 1024 * 1024 {
        p.reliable = false;
        p.diagnostics
            .push("Configuration resource limit exceeded".into());
        return p;
    }
    let mut mode = ConfigMode::Global;
    let mut seen_modes = std::collections::HashSet::new();
    for (index, line) in input.lines().enumerate() {
        if index >= 50_000 || line.len() > 4096 {
            p.reliable = false;
            p.diagnostics
                .push("Configuration line limit exceeded".into());
            break;
        }
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if [
            "banner ",
            "macro name ",
            "crypto pki certificate",
            "event manager applet ",
        ]
        .iter()
        .any(|s| t.starts_with(s))
        {
            p.reliable = false;
            p.diagnostics
                .push("Opaque or multiline IOS construct; semantic verification disabled".into());
            break;
        }
        if matches!(t, "!" | "exit" | "end" | "configure terminal") {
            mode = ConfigMode::Global;
            continue;
        }
        let words: Vec<_> = t.split_whitespace().collect();
        let next = if let Some(name) = t.strip_prefix("interface ") {
            match interface_name(name) {
                Some(n) => Some(ConfigMode::Interface(n)),
                None => {
                    p.reliable = false;
                    None
                }
            }
        } else if let Some(router) = t.strip_prefix("router ") {
            if router.split_whitespace().count() <= 3 && super::safe_text(router) {
                Some(ConfigMode::Router(router.into()))
            } else {
                p.reliable = false;
                None
            }
        } else if let Some(v) = t.strip_prefix("vlan ") {
            v.parse::<u16>()
                .ok()
                .filter(|v| (1..=4094).contains(v))
                .map(ConfigMode::Vlan)
        } else if t.starts_with("ip access-list ") || t.starts_with("ipv6 access-list ") {
            Some(ConfigMode::Acl(t.into()))
        } else if let Some(name) = t.strip_prefix("ip dhcp pool ") {
            Some(ConfigMode::DhcpPool(name.into()))
        } else {
            t.strip_prefix("line ").map(|s| ConfigMode::Line(s.into()))
        };
        if let Some(next) = next {
            if !seen_modes.insert(next.clone()) {
                p.reliable = false;
                p.diagnostics
                    .push("Duplicate configuration mode identity".into());
            }
            mode = next;
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            mode = ConfigMode::Global;
        } else if mode == ConfigMode::Global {
            p.reliable = false;
            p.diagnostics
                .push("Indented command without a recognized mode".into());
        }
        if words.is_empty() {
            continue;
        }
        p.entries.push(ModeCommand {
            mode: mode.clone(),
            text: words.join(" "),
        });
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modes_keep_ospf_interfaces_and_acl_order() {
        let c = parse(
            "hostname R1\ninterface Gi0/0\n description Uplink\n!\nrouter ospf 20\n network 10.0.0.0 0.255.255.255 area 0\n!\nip access-list extended EDGE\n deny ip any any\n permit ip host 192.0.2.1 any\n",
        );
        assert!(c.reliable);
        assert_eq!(
            c.in_mode(&ConfigMode::Interface("GigabitEthernet0/0".into())),
            vec!["description Uplink"]
        );
        let acl = c.in_mode(&ConfigMode::Acl("ip access-list extended EDGE".into()));
        assert!(acl[0].starts_with("deny"));
        assert!(acl[1].starts_with("permit"));
        assert_eq!(c.in_mode(&ConfigMode::Router("ospf 20".into())).len(), 1);
    }
    #[test]
    fn opaque_and_orphan_modes_are_unreliable() {
        assert!(!parse("banner motd #\nhostname Fake\n#").reliable);
        assert!(!parse("mystery mode\n network 10.0.0.0").reliable);
        assert!(!parse("interface range Gi0/1 - 3\n shutdown").reliable);
    }
    #[test]
    fn aliases_are_explicit_not_arbitrary() {
        assert_eq!(interface_name("Fa0/1"), Some("FastEthernet0/1".into()));
        assert!(interface_name("G0/1").is_none());
        assert!(interface_name("Gi0/1;reload").is_none());
    }
}
