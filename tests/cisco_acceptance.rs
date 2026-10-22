//! Synthetic public-API acceptance cases; these do not execute Packet Tracer.
use pka_lab_inspector::{
    cisco::{self, CommandRequest, ConfigMode, DeviceProfile, SupportState},
    commands,
    model::Checkpoint,
};

#[test]
fn explicit_interface_separator_space_normalizes_without_accepting_controls() {
    use cisco::parser::interface_name;
    assert_eq!(interface_name("FastEthernet 0/1"), interface_name("Fa0/1"));
    assert_eq!(interface_name("vlan 10"), Some("Vlan10".into()));
    assert!(interface_name("Gi\n0/0").is_none());
    assert!(interface_name("Gi0/0;reload").is_none());
}

fn checkpoint(property: &str, expected: &str) -> Checkpoint {
    Checkpoint {
        device: "R1".into(),
        device_type: "Router".into(),
        interface: "GigabitEthernet0/0".into(),
        property: property.into(),
        expected: expected.into(),
        ..Default::default()
    }
}

fn request(family: &str, parameters: &[(&str, &str)]) -> CommandRequest {
    CommandRequest {
        family: family.into(),
        device: "Synthetic-R1".into(),
        profile: DeviceProfile::Router2911,
        parameters: parameters
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

fn rejected(c: &Checkpoint) {
    let command = commands::generate(c);
    assert!(
        !command.supported && command.text.is_empty(),
        "{}: {}",
        c.property,
        command.text
    );
    assert!(!command.explanation.is_empty());
}

#[test]
fn hostname_obeys_documented_terminal_character_rule_at_both_apis() {
    for name in ["Branch-", "-Branch", "1Branch", "Branch_Name"] {
        let typed = cisco::generate(&request("basic.hostname", &[("hostname", name)]));
        assert!(!typed.executable(), "{name}: {}", typed.script());
        rejected(&checkpoint("Hostname", name));
    }
    let valid = "Branch-1";
    assert!(cisco::generate(&request("basic.hostname", &[("hostname", valid)])).executable());
    assert!(commands::generate(&checkpoint("Hostname", valid)).supported);
}

#[test]
fn typed_console_and_vty_require_existing_authentication_context() {
    for (family, fields) in [
        (
            "basic.console",
            vec![
                ("local_user_present", "yes"),
                ("aaa_mode", "disabled"),
                ("minutes", "10"),
                ("seconds", "30"),
            ],
        ),
        (
            "basic.vty_ssh",
            vec![
                ("first", "0"),
                ("last", "4"),
                ("local_user_present", "yes"),
                ("ssh_enabled", "yes"),
                ("aaa_mode", "disabled"),
            ],
        ),
    ] {
        let complete = request(family, &fields);
        let plan = cisco::generate(&complete);
        assert!(plan.executable(), "{:?}", plan.warnings);
        assert!(
            plan.commands
                .iter()
                .all(|c| matches!(c.mode, ConfigMode::Line(_)))
        );
        assert!(!plan.script().contains("aaa new-model"));
        assert!(!plan.script().contains("crypto key"));
        for field in fields.iter().map(|(k, _)| *k) {
            let mut incomplete = complete.clone();
            incomplete.parameters.remove(field);
            assert!(
                !cisco::generate(&incomplete).executable(),
                "{family}: {field}"
            );
        }
    }
}

#[test]
fn checkpoint_vty_cannot_bypass_typed_authentication_prerequisites() {
    rejected(&checkpoint("VTY SSH", "0 4"));
}

#[test]
fn banner_delimiter_and_control_characters_never_escape_global_scope() {
    for message in [
        "Authorized # access",
        "Authorized\nreload",
        "Authorized\rend",
        "Authorized\u{1b} access",
    ] {
        let plan = cisco::generate(&request("basic.banner_motd", &[("message", message)]));
        assert!(!plan.executable());
        assert!(plan.script().is_empty());
    }
    let plan = cisco::generate(&request(
        "basic.banner_motd",
        &[("message", "Authorized access only")],
    ));
    assert_eq!(
        plan.script(),
        "enable\nconfigure terminal\nbanner motd #Authorized access only#\nend\n"
    );
}

#[test]
fn cidr_address_cannot_silently_override_explicit_sibling_mask() {
    for value in ["192.0.2.1/24", "192.0.2.1-24"] {
        let mut c = checkpoint("IPv4 Address", value);
        c.evidence.push("commands:ipv4-mask=255.255.0.0".into());
        rejected(&c);
    }
}

#[test]
fn complete_route_cannot_silently_override_explicit_next_hop() {
    let mut c = checkpoint("Static Route", "10.0.0.0 255.0.0.0 192.0.2.1");
    c.evidence.push("commands:route-next-hop=192.0.2.2".into());
    rejected(&c);
}

#[test]
fn named_parameter_evidence_must_agree_with_requirement() {
    let mut c = checkpoint("Hostname", "Branch-1");
    c.evidence
        .push("commands:parameter:hostname=Branch-2".into());
    rejected(&c);
}

#[test]
fn missing_named_parameter_requires_unique_evidence() {
    let mut c = checkpoint("VLAN", "20");
    c.device_type = "Switch".into();
    c.evidence = vec![
        "assessment:device-model=2960".into(),
        "commands:parameter:vtp_mode=transparent".into(),
        "commands:parameter:vtp_version=2".into(),
        "commands:parameter:name=Staff".into(),
    ];
    let unique = commands::generate(&c);
    assert!(unique.supported, "{}", unique.explanation);
    assert!(unique.text.contains("name Staff"));
    c.evidence.push("commands:parameter:name=Guests".into());
    rejected(&c);
}

#[test]
fn interface_aliases_cannot_hide_conflicting_batch_addresses() {
    let first = checkpoint("IP Address", "192.0.2.1 255.255.255.0");
    let mut second = checkpoint("IPv4 Address", "192.0.2.2 255.255.255.0");
    second.interface = "Gi0/0".into();
    let script = commands::device_script(&[first, second], None);
    assert!(!script.contains("ip address "), "{script}");
    assert!(script.contains("conflicting"));
}

#[test]
fn description_aliases_cannot_hide_conflicting_batch_values() {
    let first = checkpoint("Description", "WAN");
    let second = checkpoint("Interface Description", "LAN");
    let script = commands::device_script(&[first, second], None);
    assert!(!script.contains("description "), "{script}");
    assert!(script.contains("conflicting"));
}

#[test]
fn batch_conflicts_leave_independent_devices_and_settings_available() {
    let first = checkpoint("Hostname", "Branch-1");
    let second = checkpoint("Host Name", "Branch-2");
    let address = checkpoint("IPv4 Address", "192.0.2.1 255.255.255.0");
    let mut other = checkpoint("Hostname", "Branch-3");
    other.device = "R2".into();
    let script = commands::device_script(&[first, second, address, other], None);
    assert!(!script.contains("hostname Branch-1\n") && !script.contains("hostname Branch-2\n"));
    assert!(script.contains("ip address 192.0.2.1 255.255.255.0"));
    assert!(script.contains("hostname Branch-3\n"));
}

#[test]
fn unsupported_and_unknown_requests_never_gain_execution_evidence() {
    let mut req = request(
        "router.route.ipv4",
        &[("route", "10.0.0.0 255.0.0.0 192.0.2.1")],
    );
    req.profile = DeviceProfile::Catalyst2960;
    let unsupported = cisco::generate(&req);
    assert!(!unsupported.executable());
    assert!(unsupported.script().is_empty());
    let known = cisco::generate(&request("basic.hostname", &[("hostname", "Branch-1")]));
    assert!(known.executable());
    assert_ne!(known.support, SupportState::VerifiedPacketTracer);
    assert!(
        known
            .warnings
            .iter()
            .any(|w| w.contains("Not execution-tested"))
    );
    assert!(!cisco::generate(&request("unknown.family", &[])).executable());
}

#[test]
fn typed_unknown_parameters_and_untrusted_targets_fail_closed() {
    let mut req = request(
        "basic.hostname",
        &[("hostname", "Branch-1"), ("typo", "ignored")],
    );
    assert!(!cisco::generate(&req).executable());
    req.parameters.remove("typo");
    for target in ["R1\nreload", "R1\rend", "R1;reload", "R1\u{1b}"] {
        req.device = target.into();
        assert!(!cisco::generate(&req).executable());
    }
}

#[test]
fn explicit_ipv4_pair_has_equivalent_scoped_output_at_both_apis() {
    let typed = cisco::generate(&request(
        "router.interface.ipv4",
        &[
            ("interface", "GigabitEthernet0/0"),
            ("address", "192.0.2.1"),
            ("mask", "255.255.255.0"),
        ],
    ));
    let legacy = commands::generate(&checkpoint("IPv4 Address", "192.0.2.1 255.255.255.0"));
    assert!(typed.executable() && legacy.supported);
    // A redundant exit before end is harmless; compare effective scope/commands.
    let without_exit = |text: &str| {
        text.lines()
            .filter(|line| *line != "exit")
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(without_exit(&typed.script()), without_exit(&legacy.text));
    assert!(
        typed
            .commands
            .iter()
            .all(|c| c.mode == ConfigMode::Interface("GigabitEthernet0/0".into()))
    );
    assert!(!typed.script().contains("shutdown"));
    assert!(!typed.script().contains("copy running-config"));
}

#[test]
fn identical_explicit_evidence_is_accepted_and_deduplicated() {
    let mut c = checkpoint("IPv4 Address", "192.0.2.1");
    c.evidence = vec!["commands:ipv4-mask=255.255.255.0".into(); 2];
    assert!(commands::generate(&c).supported);
    let script = commands::device_script(&[c.clone(), c], None);
    assert_eq!(
        script.matches("ip address 192.0.2.1 255.255.255.0").count(),
        1
    );
    assert!(!script.contains("conflicting"));
}

#[test]
fn explicit_save_is_exec_scoped_and_follows_configuration() {
    let typed = cisco::generate(&request("basic.save", &[]));
    assert!(typed.executable());
    assert_eq!(typed.commands.len(), 1);
    assert_eq!(typed.commands[0].mode, ConfigMode::Exec);
    assert!(
        typed
            .script()
            .ends_with("end\ncopy running-config startup-config\n")
    );
    let script = commands::device_script(
        &[
            checkpoint("Save", "true"),
            checkpoint("Hostname", "Branch-1"),
        ],
        None,
    );
    assert!(
        script.find("hostname Branch-1").unwrap()
            < script.find("copy running-config startup-config").unwrap()
    );
    assert!(script.contains("end\ncopy running-config startup-config\n"));
}
