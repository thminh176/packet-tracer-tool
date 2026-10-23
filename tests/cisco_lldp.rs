use pka_lab_inspector::cisco::{
    self, Classification, CommandRequest, ConfigMode, DeviceProfile, ParameterKind, SupportState,
};

const FAMILIES: [&str; 3] = [
    "switch.lldp.global",
    "switch.lldp.interface",
    "switch.lldp.timers",
];

fn example(family: &str) -> CommandRequest {
    let spec = cisco::switch::specs()
        .into_iter()
        .find(|s| s.id == family)
        .unwrap();
    CommandRequest {
        family: family.into(),
        device: "SW1".into(),
        profile: DeviceProfile::Catalyst2960,
        parameters: spec
            .parameters
            .iter()
            .map(|p| (p.name.into(), p.example.into()))
            .collect(),
    }
}
fn set(req: &CommandRequest, key: &str, value: &str) -> CommandRequest {
    let mut req = req.clone();
    req.parameters.insert(key.into(), value.into());
    req
}
fn reject(req: &CommandRequest) {
    let plan = cisco::generate(req);
    assert!(!plan.executable(), "{} {:?}", req.family, req.parameters);
    assert!(plan.commands.is_empty());
    assert!(plan.script().is_empty());
    // Callers of the switch entry point receive the same bounded validation.
    assert!(cisco::switch::generate(req).is_err());
}

#[test]
fn lldp_catalog_is_bilingual_typed_and_documented() {
    for family in FAMILIES {
        let spec = cisco::switch::specs()
            .into_iter()
            .find(|s| s.id == family)
            .unwrap();
        assert_ne!(spec.name_en, spec.name_vi);
        assert!(!spec.semantic_validation);
        assert!(spec.source.starts_with("https://www.cisco.com/"));
        assert!(spec.software.contains("not PT-tested"));
        assert!(!spec.prerequisites.is_empty());
        for p in spec.parameters {
            assert_ne!(p.description_en, p.description_vi);
            assert!(p.required);
            assert!(!matches!(p.kind, ParameterKind::Text));
            if let ParameterKind::Choice(values) = p.kind
                && p.name != "interface_mode"
            {
                assert_eq!(values, &["true", "false"]);
            }
        }
        let plan = cisco::generate(&example(family));
        assert!(plan.executable(), "{family}: {:?}", plan.warnings);
        assert_eq!(plan.support, SupportState::DocumentedNotTested);
        assert!(!plan.verification.is_empty());
        assert!(plan.warnings.iter().any(|w| w.contains("Packet Tracer")));
    }
}

#[test]
fn global_enable_and_disable_do_not_invent_port_state() {
    let req = example(FAMILIES[0]);
    for (enabled, command) in [("true", "lldp run"), ("false", "no lldp run")] {
        let plan = cisco::generate(&set(&req, "enabled", enabled));
        assert!(plan.executable());
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].mode, ConfigMode::Global);
        assert_eq!(
            plan.script(),
            format!("enable\nconfigure terminal\n{command}\nend\n")
        );
    }
}

#[test]
fn transmit_and_receive_are_independent_and_never_enable_globally() {
    let req = example(FAMILIES[1]);
    for tx in ["true", "false"] {
        for rx in ["true", "false"] {
            for port_mode in ["access", "trunk"] {
                let req = set(
                    &set(&set(&req, "transmit", tx), "receive", rx),
                    "interface_mode",
                    port_mode,
                );
                let plan = cisco::generate(&req);
                assert!(plan.executable());
                let tx_no = if tx == "true" { "" } else { "no " };
                let rx_no = if rx == "true" { "" } else { "no " };
                assert_eq!(
                    plan.script(),
                    format!(
                        "enable\nconfigure terminal\ninterface GigabitEthernet0/1\n{tx_no}lldp transmit\n{rx_no}lldp receive\nend\n"
                    )
                );
                assert!(
                    plan.commands
                        .iter()
                        .all(|c| c.mode == ConfigMode::Interface("GigabitEthernet0/1".into()))
                );
            }
        }
    }
}

#[test]
fn interface_enable_requires_explicit_global_prerequisite() {
    let req = set(&example(FAMILIES[1]), "global_enabled", "false");
    reject(&req);
    reject(&set(&req, "transmit", "false"));
    reject(&set(&req, "receive", "false"));
    let disabled = set(&set(&req, "transmit", "false"), "receive", "false");
    assert!(cisco::generate(&disabled).executable());
}

#[test]
fn all_required_fields_fail_closed_when_missing_or_empty() {
    for family in FAMILIES {
        let req = example(family);
        for field in req.parameters.keys() {
            let mut missing = req.clone();
            missing.parameters.remove(field);
            reject(&missing);
            let plan = cisco::generate(&missing);
            assert_eq!(plan.classification, Classification::MissingParameters);
            assert!(plan.missing_parameters.contains(field));
            reject(&set(&req, field, ""));
        }
    }
}

#[test]
fn only_identified_documented_catalyst_profiles_are_accepted() {
    for family in FAMILIES {
        for profile in DeviceProfile::ALL {
            let mut req = example(family);
            req.profile = profile;
            if matches!(
                profile,
                DeviceProfile::Catalyst2960 | DeviceProfile::Catalyst3560
            ) {
                assert!(cisco::generate(&req).executable());
            } else {
                reject(&req);
            }
        }
    }
}

#[test]
fn physical_ports_and_confirmed_static_modes_are_required() {
    let req = example(FAMILIES[1]);
    for interface in ["FastEthernet0/1", "Gi0/2", "GigabitEthernet1/0/1"] {
        assert!(cisco::generate(&set(&req, "interface", interface)).executable());
    }
    for interface in [
        "Gi0/1.10",
        "Loopback0",
        "Vlan10",
        "Port-channel1",
        "Tunnel0",
        "Serial0/0/0",
        "Gi0/0",
        "Gi0/1-2",
        "Gi0/1,Gi0/2",
    ] {
        reject(&set(&req, "interface", interface));
    }
    for mode in [
        "routed",
        "dot1q-tunnel",
        "tunnel",
        "dynamic_auto",
        "unknown",
    ] {
        reject(&set(&req, "interface_mode", mode));
    }
}

#[test]
fn timers_preserve_documented_ranges_and_global_mode() {
    let req = example(FAMILIES[2]);
    let plan = cisco::generate(&req);
    assert_eq!(
        plan.script(),
        "enable\nconfigure terminal\nlldp holdtime 120\nlldp reinit 2\nlldp timer 30\nend\n"
    );
    assert!(plan.commands.iter().all(|c| c.mode == ConfigMode::Global));
    for (key, bounds, invalid) in [
        ("timer", ["5", "65534"], ["4", "65535"]),
        ("holdtime", ["0", "65535"], ["-1", "65536"]),
        ("reinit", ["2", "5"], ["1", "6"]),
    ] {
        for value in bounds {
            assert!(cisco::generate(&set(&req, key, value)).executable());
        }
        for value in invalid.into_iter().chain(["1.5", "+5", "4294967296"]) {
            reject(&set(&req, key, value));
        }
    }
}

#[test]
fn mode_rendering_returns_to_global_before_timers() {
    let mut commands = cisco::generate(&example(FAMILIES[0])).commands;
    commands.extend(cisco::generate(&example(FAMILIES[1])).commands);
    commands.extend(cisco::generate(&example(FAMILIES[2])).commands);
    assert_eq!(
        cisco::render(&commands),
        "enable\nconfigure terminal\nlldp run\ninterface GigabitEthernet0/1\nlldp transmit\nlldp receive\nexit\nlldp holdtime 120\nlldp reinit 2\nlldp timer 30\nend\n"
    );
}

#[test]
fn unknown_keys_injection_and_numeric_boolean_enums_are_rejected() {
    for family in FAMILIES {
        let req = example(family);
        reject(&set(&req, "unsafe_cli", "reload"));
        reject(&set(&req, "unsafe_cli", ""));
        for key in req.parameters.keys() {
            for value in [
                "true\nend\nreload",
                "true;reload",
                "true\rreload",
                "true\t",
                " true",
                "true ",
                "$(reload)",
                "true|reload",
            ] {
                reject(&set(&req, key, value));
            }
        }
    }
    for (family, keys) in [
        (FAMILIES[0], vec!["enabled"]),
        (FAMILIES[1], vec!["transmit", "receive", "global_enabled"]),
    ] {
        for key in keys {
            for value in ["0", "1", "2", "yes", "enabled", "True", "FALSE", "unknown"] {
                reject(&set(&example(family), key, value));
            }
        }
    }
}
