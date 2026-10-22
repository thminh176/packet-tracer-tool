//! Public checkpoint suggestions may be conditional; typed library plans remain strict.
use pka_lab_inspector::{
    cisco::{self, Classification, CommandRequest, DeviceProfile},
    commands::{device_script, generate},
    model::{Analysis, Checkpoint, Status},
};

fn access_vlan(value: &str) -> Checkpoint {
    Checkpoint {
        id: "synthetic-access-vlan".into(),
        device: "SW1".into(),
        device_type: "Switch".into(),
        interface: "FastEthernet0/1".into(),
        property: "Access VLAN".into(),
        expected: value.into(),
        path: "SW1 / FastEthernet0/1 / Access VLAN".into(),
        weight: 3.0,
        ..Default::default()
    }
}

fn assert_assignment(c: &Checkpoint, expected: &str) {
    let command = generate(c);
    assert!(command.supported, "{}", command.explanation);
    assert!(!command.gui);
    assert_eq!(
        command
            .text
            .lines()
            .filter(|line| *line == expected)
            .count(),
        1
    );
    assert!(
        command
            .text
            .lines()
            .any(|line| line == format!("interface {}", c.interface))
    );
    assert!(
        !command.text.contains("switchport mode"),
        "{}",
        command.text
    );
    assert!(!command.text.lines().any(|line| line.starts_with("vlan ")));
    let script = device_script(std::slice::from_ref(c), Some("SW1"));
    assert_eq!(
        script.lines().filter(|line| *line == expected).count(),
        1,
        "{script}"
    );
    assert!(!script.contains("switchport mode"), "{script}");
}

#[test]
fn complete_access_vlan_requirement_needs_no_invented_mode_or_vlan_state() {
    for model in [None, Some("2960-24TT"), Some("3560-24PS")] {
        let mut c = access_vlan("20");
        if let Some(model) = model {
            c.evidence.push(format!("assessment:device-model={model}"));
        }
        assert_assignment(&c, "switchport access vlan 20");
        let explanation = generate(&c).explanation.to_ascii_lowercase();
        assert!(explanation.contains("mode"), "{explanation}");
        assert!(explanation.contains("vlan"), "{explanation}");
    }
}

#[test]
fn partial_noncontradictory_context_still_allows_conditional_assignment() {
    for metadata in ["interface_mode=access", "vlan_active=yes"] {
        let mut c = access_vlan("20");
        c.evidence.push(format!("commands:parameter:{metadata}"));
        assert_assignment(&c, "switchport access vlan 20");
    }
}

#[test]
fn access_vlan_decimal_boundaries_preserve_reserved_vlan_policy() {
    for value in ["1", "1001", "1006", "4094"] {
        assert_assignment(
            &access_vlan(value),
            &format!("switchport access vlan {value}"),
        );
    }
    for value in [
        "",
        "0",
        "4095",
        "65535",
        "4294967295",
        "1002",
        "1003",
        "1004",
        "1005",
        "-1",
        "+20",
        "20.0",
        "20 30",
        "20;reload",
        "20\nreload",
        "unknown",
    ] {
        let command = generate(&access_vlan(value));
        assert!(!command.supported, "accepted {value:?}: {}", command.text);
        assert!(command.text.is_empty(), "{}", command.text);
    }
}

#[test]
fn access_vlan_rejects_router_unknown_and_conflicting_device_identity() {
    for kind in ["Router", "", "PC"] {
        let mut c = access_vlan("20");
        c.device_type = kind.into();
        assert!(!generate(&c).supported, "accepted device type {kind:?}");
    }
    for model in ["1941", "2911", "ISR4321"] {
        let mut c = access_vlan("20");
        c.evidence.push(format!("assessment:device-model={model}"));
        assert!(
            !generate(&c).supported,
            "accepted contradictory model {model}"
        );
    }
}

#[test]
fn access_vlan_requires_a_safe_physical_switch_interface() {
    for interface in [
        "",
        "Vlan20",
        "Loopback0",
        "Serial0/0/0",
        "Port-channel1",
        "GigabitEthernet0/1.20",
        "FastEthernet0/0",
        "FastEthernet0/1\nshutdown",
    ] {
        let mut c = access_vlan("20");
        c.interface = interface.into();
        assert!(!generate(&c).supported, "accepted interface {interface:?}");
    }
}

#[test]
fn named_typed_evidence_cannot_be_discarded_by_conditional_suggestions() {
    for entries in [
        vec!["interface_mode=trunk"],
        vec!["interface_mode=routed"],
        vec!["vlan_active=no"],
        vec!["interface_mode=invalid"],
        vec!["vlan_active=unknown"],
        vec!["vlan=30"],
        vec!["interface=FastEthernet0/2"],
        vec!["interface_mode=access", "interface_mode=trunk"],
        vec!["vlan_active=yes", "vlan_active=no"],
    ] {
        let mut c = access_vlan("20");
        c.evidence.extend(
            entries
                .iter()
                .map(|entry| format!("commands:parameter:{entry}")),
        );
        let command = generate(&c);
        assert!(!command.supported, "accepted {entries:?}: {}", command.text);
        assert!(command.text.is_empty());
    }
}

#[test]
fn unsupported_assessment_evidence_still_blocks_conditional_assignment() {
    for marker in [
        "assessment:unsupported",
        "assessment:ambiguous",
        "assessment:inventory",
    ] {
        let mut c = access_vlan("20");
        c.evidence.push(marker.into());
        assert!(!generate(&c).supported, "accepted {marker}");
    }
}

#[test]
fn command_readiness_does_not_change_unverified_status_or_fabricate_evidence() {
    let c = access_vlan("20");
    let before = serde_json::to_value(&c).unwrap();
    assert_assignment(&c, "switchport access vlan 20");
    assert_eq!(cisco::mapping::classify(&c), Classification::Ready);
    let coverage = cisco::mapping::coverage(std::slice::from_ref(&c));
    assert_eq!(coverage.command_ready, 1);
    assert_eq!(coverage.validation_verified, 0);
    assert_eq!(serde_json::to_value(&c).unwrap(), before);
    assert_eq!(c.status, Status::Unverified);
    assert!(c.current.is_none());
    assert!(c.evidence.is_empty());
    let summary = Analysis {
        checkpoints: vec![c],
        ..Default::default()
    }
    .summary();
    assert_eq!(summary.unverified, 1);
    assert_eq!(summary.verified_score, 0.0);
}

#[test]
fn typed_library_access_vlan_workflow_remains_strict() {
    let mut request = CommandRequest {
        family: "switch.access_vlan".into(),
        device: "SW1".into(),
        profile: DeviceProfile::Catalyst2960,
        parameters: [
            ("interface".into(), "FastEthernet0/1".into()),
            ("vlan".into(), "20".into()),
        ]
        .into(),
    };
    let plan = cisco::generate(&request);
    assert_eq!(plan.classification, Classification::MissingParameters);
    assert!(!plan.executable());
    assert!(plan.script().is_empty());
    request
        .parameters
        .insert("interface_mode".into(), "access".into());
    assert!(!cisco::generate(&request).executable());
    request
        .parameters
        .insert("vlan_active".into(), "yes".into());
    assert!(cisco::generate(&request).executable());
    request
        .parameters
        .insert("interface_mode".into(), "trunk".into());
    assert!(!cisco::generate(&request).executable());
}
