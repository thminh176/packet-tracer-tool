//! Small explicit-configuration validators. No defaults or operational state are invented.
use super::{ConfigMode, parser};
use crate::model::{Checkpoint, Status};
pub struct DeviceObservation<'a> {
    pub device: &'a str,
    pub unique_current_snapshot: bool,
    pub unique_device: bool,
    pub configuration: &'a str,
}
#[derive(Debug)]
pub struct ValidationResult {
    pub status: Status,
    pub current: Option<String>,
    pub explanation: String,
    pub evidence: Vec<String>,
}
fn unknown(reason: &str) -> ValidationResult {
    ValidationResult {
        status: Status::Unverified,
        current: None,
        explanation: reason.into(),
        evidence: vec![],
    }
}
pub fn validate(c: &Checkpoint, observation: &DeviceObservation<'_>) -> ValidationResult {
    if !observation.unique_current_snapshot
        || !observation.unique_device
        || observation.device != c.device
    {
        return unknown("Current snapshot/device identity is not unique");
    }
    let config = parser::parse(observation.configuration);
    if !config.reliable {
        return unknown("Configuration contains an unsupported or ambiguous mode");
    }
    let property: String = c
        .property
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    let expected = c.expected.trim();
    let mode = if c.interface.is_empty() {
        ConfigMode::Global
    } else {
        let Some(name) = parser::interface_name(&c.interface) else {
            return unknown("Unknown interface identity");
        };
        ConfigMode::Interface(name)
    };
    let entries = config.in_mode(&mode);
    let one = |prefix: &str| -> Option<&str> {
        let mut matches = entries.iter().filter_map(|s| s.strip_prefix(prefix));
        let first = matches.next()?;
        if matches.next().is_some() {
            None
        } else {
            Some(first)
        }
    };
    let (actual, equal) = match property.as_str() {
        "ipv6unicastrouting" if c.interface.is_empty() => {
            if !super::mapping::request(c, "true")
                .is_some_and(|req| super::generate(&req).executable())
            {
                return unknown("IPv6 forwarding validator requires a compatible router profile");
            }
            let Some(required) = crate::commands::assessment::boolean(expected) else {
                return unknown("Unknown IPv6 forwarding flag");
            };
            if super::mapping::parameter_conflict(c, &required.to_string()) {
                return unknown("Conflicting explicit IPv6 forwarding requirements");
            }
            let states: Vec<_> = entries
                .iter()
                .filter_map(|line| match *line {
                    "ipv6 unicast-routing" => Some(true),
                    "no ipv6 unicast-routing" => Some(false),
                    _ => None,
                })
                .collect();
            if states.len() != 1 {
                return unknown(
                    "No unique explicit IPv6 forwarding state; defaults are not inferred",
                );
            }
            (states[0].to_string(), required == states[0])
        }
        "encapsulation" | "vlanid" => {
            let Ok(resolved) = crate::commands::explicit_value(c) else {
                return unknown("Unsupported or conflicting encapsulation requirement");
            };
            let Some(request) = super::mapping::request(c, &resolved)
                .filter(|r| r.family == "router.interface.dot1q")
            else {
                return unknown("No supported dot1Q assessment context");
            };
            if super::mapping::parameter_conflict(c, &resolved)
                || !super::generate(&request).executable()
            {
                return unknown(
                    "dot1Q requires a valid Ethernet subinterface and explicit unique VLAN ID",
                );
            }
            let Some(required) = request.get("vlan").parse::<u16>().ok() else {
                return unknown("Missing expected VLAN ID");
            };
            let Some(line) = one("encapsulation ") else {
                return unknown("No unique explicit encapsulation on the target subinterface");
            };
            let words: Vec<_> = line.split_whitespace().collect();
            if !(words.len() == 2 || (words.len() == 3 && words[2] == "native"))
                || !words[0].eq_ignore_ascii_case("dot1q")
            {
                return unknown("Unsupported encapsulation syntax");
            }
            let Some(vlan) = words[1]
                .parse::<u16>()
                .ok()
                .filter(|v| (1..=4094).contains(v))
            else {
                return unknown("Invalid current VLAN ID");
            };
            let native_matches = match request.get("native") {
                "true" => words.len() == 3,
                "false" => words.len() == 2,
                "" => true,
                _ => return unknown("Unknown required native VLAN option"),
            };
            (
                format!("encapsulation {line}"),
                vlan == required && native_matches,
            )
        }
        "interfacedescription" | "description" if !c.interface.is_empty() => {
            let Some(actual) = one("description ") else {
                return unknown("No unique explicit interface description");
            };
            (actual.to_owned(), actual == expected)
        }
        "switchportmode" => {
            if !matches!(expected, "access" | "trunk") {
                return unknown("Ambiguous required port mode");
            }
            let Some(actual) = one("switchport mode ") else {
                return unknown("No unique explicit port mode");
            };
            if !matches!(actual, "access" | "trunk") {
                return unknown("Dynamic switchport state is outside the validator subset");
            }
            (actual.to_owned(), actual == expected)
        }
        "accessvlan" | "nativevlan" => {
            let required_mode = if property == "accessvlan" {
                "access"
            } else {
                "trunk"
            };
            if one("switchport mode ") != Some(required_mode) {
                return unknown("Relevant static switchport mode is not explicitly verified");
            }
            let prefix = if property == "accessvlan" {
                "switchport access vlan "
            } else {
                "switchport trunk native vlan "
            };
            let Some(actual) = one(prefix) else {
                return unknown("No unique explicit VLAN setting");
            };
            let (Ok(e), Ok(a)) = (expected.parse::<u16>(), actual.parse::<u16>()) else {
                return unknown("Invalid VLAN encoding");
            };
            if !(1..=4094).contains(&e) || !(1..=4094).contains(&a) {
                return unknown("Invalid VLAN range");
            }
            (actual.to_owned(), e == a)
        }
        "allowedvlans" => {
            if one("switchport mode ") != Some("trunk") {
                return unknown("Trunk mode not explicitly verified");
            }
            let Some(actual) = one("switchport trunk allowed vlan ") else {
                return unknown("No unique explicit allowed-VLAN list");
            };
            let exp = expected.split_whitespace().collect::<Vec<_>>().join(",");
            let (Some(e), Some(a)) = (super::vlan_list(&exp), super::vlan_list(actual)) else {
                return unknown("Unknown allowed-VLAN operation or encoding");
            };
            (actual.to_owned(), e == a)
        }
        "stpmode" | "spanningtreemode" if c.interface.is_empty() => {
            if !matches!(expected, "pvst" | "rapid-pvst") {
                return unknown("Unknown required STP mode");
            }
            let Some(actual) = one("spanning-tree mode ") else {
                return unknown("No unique explicit STP mode");
            };
            (actual.to_owned(), actual == expected)
        }
        _ => return unknown("No semantic validator for this checkpoint family"),
    };
    ValidationResult {
        current: Some(actual.clone()),
        status: if equal {
            Status::Passed
        } else {
            Status::Failed
        },
        explanation: if equal {
            "Verified explicit configuration on the uniquely identified current device".into()
        } else {
            "Explicit current configuration differs from the assessment requirement".into()
        },
        evidence: vec![
            format!("expected assessment / {}: {}", c.path, c.expected),
            format!("current device {} / {:?}: {actual}", c.device, mode),
        ],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn check(p: &str, e: &str) -> Checkpoint {
        Checkpoint {
            device: "SW1".into(),
            interface: "Gi0/1".into(),
            property: p.into(),
            expected: e.into(),
            ..Default::default()
        }
    }
    fn observation(s: &str) -> DeviceObservation<'_> {
        DeviceObservation {
            device: "SW1",
            unique_current_snapshot: true,
            unique_device: true,
            configuration: s,
        }
    }
    #[test]
    fn vlan_equivalence_and_mismatch_are_scoped() {
        let c = check("Allowed VLANs", "10 11 12");
        assert_eq!(validate(&c,&observation("interface GigabitEthernet0/1\n switchport mode trunk\n switchport trunk allowed vlan 10-12\n!")).status,Status::Passed);
        assert_eq!(
            validate(
                &check("Access VLAN", "20"),
                &observation(
                    "interface Gi0/1\n switchport mode access\n switchport access vlan 30\n!"
                )
            )
            .status,
            Status::Failed
        );
    }
    #[test]
    fn ambiguous_identity_default_or_duplicate_remain_unknown() {
        let c = check("Access VLAN", "20");
        let mut o = observation("interface Gi0/1\n switchport access vlan 20\n!");
        assert_eq!(validate(&c, &o).status, Status::Unverified);
        o.unique_device = false;
        assert_eq!(validate(&c, &o).status, Status::Unverified);
        let o = observation(
            "interface Gi0/1\n switchport mode access\n switchport access vlan 20\n switchport access vlan 30\n!",
        );
        assert_eq!(validate(&c, &o).status, Status::Unverified);
    }
}
