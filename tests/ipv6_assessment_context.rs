use pka_lab_inspector::{
    commands,
    inspector::{compare, parse},
    model::{Analysis, Checkpoint, Status},
};
fn check(property: &str, expected: &str) -> Checkpoint {
    Checkpoint {
        device: "Lab-Router".into(),
        device_type: "Router".into(),
        property: property.into(),
        expected: expected.into(),
        ..Default::default()
    }
}
fn observed(c: Checkpoint, config: &str, device: &str) -> Checkpoint {
    let mut a = Analysis {
        checkpoints: vec![c],
        ..Default::default()
    };
    let lines = config
        .lines()
        .map(|s| format!("<LINE>{s}</LINE>"))
        .collect::<String>();
    let progress = parse(&format!("<PACKETTRACER5><NETWORK><DEVICES><DEVICE><ENGINE><NAME>{device}</NAME><TYPE>Router</TYPE></ENGINE><RUNNINGCONFIG>{lines}</RUNNINGCONFIG></DEVICE></DEVICES></NETWORK></PACKETTRACER5>")).unwrap();
    compare(&mut a, &progress);
    a.checkpoints.remove(0)
}
#[test]
fn one_is_a_scoped_ipv6_forwarding_flag_not_a_raw_cli_number() {
    for value in ["1", "true", "enabled", "on"] {
        let c = check("IPv6 Unicast Routing", value);
        let cmd = commands::generate(&c);
        assert!(cmd.supported, "{}", cmd.explanation);
        assert_eq!(
            cmd.text,
            "enable\nconfigure terminal\nipv6 unicast-routing\nend\n"
        );
        assert_eq!(c.status, Status::Unverified);
        assert!(c.current.is_none());
    }
    let cmd = commands::generate(&check("IPv6 Unicast Routing", "0"));
    assert!(cmd.supported);
    assert!(cmd.text.contains("no ipv6 unicast-routing\n"));
    assert!(cmd.explanation.contains("all IPv6 transit traffic"));
}
#[test]
fn unknown_numeric_flags_device_types_and_interface_scope_are_rejected() {
    for value in [
        "2",
        "64",
        "98",
        "-1",
        "65535",
        "1;reload",
        "1\nreload",
        "unknown",
    ] {
        assert!(
            !commands::generate(&check("IPv6 Unicast Routing", value)).supported,
            "{value}"
        );
    }
    let mut c = check("IPv6 Unicast Routing", "1");
    c.interface = "Gi0/0".into();
    assert!(!commands::generate(&c).supported);
    c.interface.clear();
    c.device_type = "Switch".into();
    assert!(!commands::generate(&c).supported);
    assert!(!commands::generate(&check("Port Status", "1")).supported);
    assert!(!commands::generate(&check("Unknown Boolean", "1")).supported);
}
#[test]
fn interface_ipv6_enablement_and_slaac_are_distinct_from_global_routing() {
    for (property, line) in [
        ("IPv6 Enable", "ipv6 enable"),
        ("IPv6 Auto Config", "ipv6 address autoconfig"),
    ] {
        let mut c = check(property, "1");
        assert!(!commands::generate(&c).supported);
        c.interface = "GigabitEthernet0/0".into();
        let cmd = commands::generate(&c);
        assert!(cmd.supported, "{}", cmd.explanation);
        assert!(
            cmd.text
                .contains(&format!("interface GigabitEthernet0/0\n{line}\n"))
        );
        assert!(!cmd.text.contains("ipv6 unicast-routing"));
        assert!(!cmd.text.contains("no shutdown"));
        c.expected = "0".into();
        assert!(!commands::generate(&c).supported); // Enable-only workflow.
    }
}
#[test]
fn forwarding_comparison_requires_unique_explicit_scoped_state() {
    assert_eq!(
        observed(
            check("IPv6 Unicast Routing", "1"),
            "ipv6 unicast-routing",
            "Lab-Router"
        )
        .status,
        Status::Passed
    );
    assert_eq!(
        observed(
            check("IPv6 Unicast Routing", "1"),
            "no ipv6 unicast-routing",
            "Lab-Router"
        )
        .status,
        Status::Failed
    );
    for config in [
        "hostname Lab-Router",
        "ipv6 unicast-routing\nno ipv6 unicast-routing",
        "interface Gi0/0\n ipv6 unicast-routing",
    ] {
        assert_eq!(
            observed(check("IPv6 Unicast Routing", "1"), config, "Lab-Router").status,
            Status::Unverified
        );
    }
    assert_eq!(
        observed(
            check("IPv6 Unicast Routing", "1"),
            "ipv6 unicast-routing",
            "Other-Router"
        )
        .status,
        Status::Unverified
    );
    let mut c = check("IPv6 Unicast Routing", "1");
    c.evidence.push("commands:parameter:enabled=false".into());
    assert_eq!(
        observed(c, "ipv6 unicast-routing", "Lab-Router").status,
        Status::Unverified
    );
}
#[test]
fn interface_flags_compare_explicit_ios_commands_without_inventing_defaults() {
    for (property, line) in [
        ("IPv6 Enable", "ipv6 enable"),
        ("IPv6 Auto Config", "ipv6 address autoconfig"),
    ] {
        let mut c = check(property, "1");
        c.interface = "Gi0/0".into();
        assert_eq!(
            observed(
                c.clone(),
                &format!("interface GigabitEthernet0/0\n {line}\n!"),
                "Lab-Router"
            )
            .status,
            Status::Passed
        );
        assert_eq!(
            observed(
                c.clone(),
                &format!("interface GigabitEthernet0/0\n no {line}\n!"),
                "Lab-Router"
            )
            .status,
            Status::Failed
        );
        assert_eq!(
            observed(
                c,
                "interface Gi0/0\n description Missing explicit flag\n!",
                "Lab-Router"
            )
            .status,
            Status::Unverified
        );
    }
}

#[test]
fn original_ipv6_dot1q_fixture_verifies_only_its_explicit_current_configuration() {
    let a = parse(include_str!("fixtures/ipv6-dot1q.xml")).unwrap();
    assert_eq!(a.summary().total, 4);
    assert_eq!(a.summary().passed, 4);
    assert_eq!(a.summary().failed, 0);
    assert_eq!(a.summary().unverified, 0);
    for c in &a.checkpoints {
        assert!(commands::generate(c).supported);
    }
    let conflicting = include_str!("fixtures/ipv6-dot1q.xml")
        .replace("</PORT>", "<IPV6_ENABLED>0</IPV6_ENABLED></PORT>");
    let a = parse(&conflicting).unwrap();
    assert_eq!(
        a.checkpoints
            .iter()
            .find(|c| c.property == "IPv6 Enable")
            .unwrap()
            .status,
        Status::Unverified
    );
}
