use pka_lab_inspector::{
    cisco::{
        self, CommandRequest, DeviceProfile,
        validator::{DeviceObservation, validate},
    },
    commands,
    model::{Checkpoint, Status},
};
fn request(id: &str) -> CommandRequest {
    let spec = cisco::registry().into_iter().find(|s| s.id == id).unwrap();
    cisco::example_request(&spec, DeviceProfile::Router2911)
}
fn checkpoint(property: &str, expected: &str) -> Checkpoint {
    Checkpoint {
        device: "Synthetic-Router".into(),
        device_type: "Router".into(),
        property: property.into(),
        expected: expected.into(),
        ..Default::default()
    }
}
fn status(c: &Checkpoint, config: &str) -> Status {
    validate(
        c,
        &DeviceObservation {
            device: "Synthetic-Router",
            unique_current_snapshot: true,
            unique_device: true,
            configuration: config,
        },
    )
    .status
}
#[test]
fn new_protocol_and_ntp_learning_examples_require_explicit_inputs() {
    for family in [
        "basic.ntp_server",
        "basic.ntp_source",
        "router.rip.version",
        "router.rip.passive",
        "router.rip.auto_summary",
        "router.eigrp.passive",
        "router.eigrp.router_id",
    ] {
        let req = request(family);
        let plan = cisco::generate(&req);
        assert!(plan.executable(), "{family}: {:?}", plan.warnings);
        assert_ne!(plan.support, cisco::SupportState::VerifiedPacketTracer);
        for field in cisco::registry()
            .into_iter()
            .find(|s| s.id == family)
            .unwrap()
            .parameters
            .into_iter()
            .filter(|p| p.required)
        {
            let mut missing = req.clone();
            missing.parameters.remove(field.name);
            assert!(
                !cisco::generate(&missing).executable(),
                "{family}: {}",
                field.name
            );
        }
        let mut bad = req;
        bad.parameters
            .insert("arbitrary_cli".into(), "reload".into());
        assert!(!cisco::generate(&bad).executable());
    }
}
#[test]
fn ntp_has_no_invented_master_version_keys_or_preference_removal() {
    let mut req = request("basic.ntp_server");
    assert_eq!(
        cisco::generate(&req).script(),
        "enable\nconfigure terminal\nntp server 192.0.2.123\nend\n"
    );
    req.parameters.insert("version".into(), "4".into());
    req.parameters.insert("prefer".into(), "true".into());
    assert!(
        cisco::generate(&req)
            .script()
            .contains("ntp server 192.0.2.123 version 4 prefer\n")
    );
    for (key, value) in [
        ("server", "0.0.0.0"),
        ("server", "127.0.0.1"),
        ("server", "224.0.0.1"),
        ("server", "255.255.255.255"),
        ("server", "server.example"),
        ("server", "192.0.2.123\nreload"),
        ("version", "1"),
        ("version", "5"),
        ("prefer", "false"),
        ("prefer", "1"),
    ] {
        let mut bad = req.clone();
        bad.parameters.insert(key.into(), value.into());
        assert!(!cisco::generate(&bad).executable(), "{key}={value}");
    }
}
#[test]
fn ntp_source_does_not_assign_interface_addresses_or_invent_a_server() {
    let mut req = request("basic.ntp_source");
    assert_eq!(
        cisco::generate(&req).script(),
        "enable\nconfigure terminal\nntp source Loopback0\nend\n"
    );
    req.parameters.insert("address_present".into(), "no".into());
    assert!(!cisco::generate(&req).executable());
    req = request("basic.ntp_source");
    req.profile = DeviceProfile::Catalyst2960;
    assert!(!cisco::generate(&req).executable());
    req.parameters.insert("interface".into(), "Vlan42".into());
    assert!(cisco::generate(&req).executable());
    req.parameters.insert("interface".into(), "Vlan4095".into());
    assert!(!cisco::generate(&req).executable());
}

#[test]
fn ipv6_ntp_reuses_the_family_but_requires_version_and_rejects_ambiguous_scope() {
    let mut req = request("basic.ntp_server");
    req.parameters
        .insert("server".into(), "2001:db8::123".into());
    let incomplete = cisco::generate(&req);
    assert!(!incomplete.executable());
    assert!(incomplete.missing_parameters.iter().any(|p| p == "version"));
    req.parameters.insert("version".into(), "3".into());
    assert!(!cisco::generate(&req).executable());
    req.parameters.insert("version".into(), "4".into());
    assert!(
        cisco::generate(&req)
            .script()
            .contains("ntp server 2001:db8::123 version 4\n")
    );
    for server in [
        "::",
        "::1",
        "ff02::1",
        "fe80::1",
        "::ffff:192.0.2.1",
        "fe80::1%Gi0/0",
    ] {
        req.parameters.insert("server".into(), server.into());
        assert!(!cisco::generate(&req).executable());
    }
    req.parameters
        .insert("server".into(), "2001:db8::123".into());
    req.profile = DeviceProfile::Catalyst2960;
    assert!(!cisco::generate(&req).executable());
}
#[test]
fn routing_controls_are_mode_scoped_without_hidden_network_statements() {
    for (family, context, line) in [
        ("router.rip.version", "router rip", "version 2"),
        ("router.rip.auto_summary", "router rip", "no auto-summary"),
        (
            "router.rip.passive",
            "router rip",
            "passive-interface GigabitEthernet0/0",
        ),
        (
            "router.eigrp.passive",
            "router eigrp 100",
            "passive-interface GigabitEthernet0/0",
        ),
        (
            "router.eigrp.router_id",
            "router eigrp 100",
            "eigrp router-id 192.0.2.9",
        ),
    ] {
        let req = request(family);
        let script = cisco::generate(&req).script();
        assert!(script.contains(&format!("{context}\n{line}\n")), "{script}");
        assert!(!script.contains("network "));
        assert!(!script.contains("no shutdown"));
        assert!(!script.contains("clear "));
        for profile in [
            DeviceProfile::Catalyst2960,
            DeviceProfile::Catalyst3560,
            DeviceProfile::GenericSwitch,
        ] {
            let mut bad = req.clone();
            bad.profile = profile;
            assert!(!cisco::generate(&bad).executable());
        }
    }
}
#[test]
fn classic_eigrp_requires_a_real_as_and_valid_identifier_not_an_interface_ip() {
    for family in ["router.eigrp.passive", "router.eigrp.router_id"] {
        for as_number in ["0", "65536", "named-process", "1\nreload"] {
            let mut req = request(family);
            req.parameters.insert("as_number".into(), as_number.into());
            assert!(!cisco::generate(&req).executable());
        }
    }
    let mut req = request("router.eigrp.router_id");
    req.parameters
        .insert("router_id".into(), "127.0.0.1".into());
    assert!(cisco::generate(&req).executable());
    for id in ["0.0.0.0", "255.255.255.255", "192.0.2.1/24", "::1"] {
        req.parameters.insert("router_id".into(), id.into());
        assert!(!cisco::generate(&req).executable());
    }
    for family in ["router.rip.passive", "router.eigrp.passive"] {
        let mut req = request(family);
        req.parameters.insert("interface".into(), "default".into());
        assert!(!cisco::generate(&req).executable());
        req = request(family);
        req.parameters.insert("state".into(), "true".into());
        assert!(!cisco::generate(&req).executable());
    }
}
#[test]
fn explicit_routing_scalars_are_verified_in_the_correct_process_only() {
    for (property, expected, metadata, mode, line, different) in [
        (
            "OSPF Configured Router ID",
            "192.0.2.9",
            "process_id=20",
            "ospf 20",
            "router-id 192.0.2.9",
            "router-id 192.0.2.10",
        ),
        (
            "OSPF Reference Bandwidth",
            "1000",
            "process_id=20",
            "ospf 20",
            "auto-cost reference-bandwidth 1000",
            "auto-cost reference-bandwidth 100",
        ),
        (
            "EIGRP Configured Router ID",
            "192.0.2.9",
            "as_number=100",
            "eigrp 100",
            "eigrp router-id 192.0.2.9",
            "eigrp router-id 192.0.2.10",
        ),
        ("RIP Version", "2", "", "rip", "version 2", "version 1"),
        (
            "RIP Auto Summary",
            "false",
            "",
            "rip",
            "no auto-summary",
            "auto-summary",
        ),
    ] {
        let mut c = checkpoint(property, expected);
        if !metadata.is_empty() {
            c.evidence.push(format!("commands:parameter:{metadata}"));
        }
        assert_eq!(
            status(&c, &format!("router {mode}\n {line}\n!")),
            Status::Passed,
            "{property}"
        );
        assert_eq!(
            status(&c, &format!("router {mode}\n {different}\n!")),
            Status::Failed,
            "{property}"
        );
        assert_eq!(
            status(&c, &format!("router {mode}\n {line}\n {different}\n!")),
            Status::Unverified
        );
        assert_eq!(
            status(&c, &format!("interface Gi0/0\n {line}\n!")),
            Status::Unverified
        );
        assert_eq!(status(&c, "hostname Synthetic-Router"), Status::Unverified);
        if !metadata.is_empty() {
            c.evidence.clear();
            assert_eq!(
                status(&c, &format!("router {mode}\n {line}\n!")),
                Status::Unverified
            );
        }
    }
}
#[test]
fn passive_comparison_canonicalizes_interface_aliases_without_inferring_defaults() {
    for (property, mode, metadata) in [
        ("OSPF Passive Interface", "ospf 20", "process_id=20"),
        ("EIGRP Passive Interface", "eigrp 100", "as_number=100"),
        ("RIP Passive Interface", "rip", ""),
    ] {
        let mut c = checkpoint(property, "passive");
        c.interface = "Gi0/0".into();
        if !metadata.is_empty() {
            c.evidence.push(format!("commands:parameter:{metadata}"));
        }
        assert_eq!(
            status(
                &c,
                &format!("router {mode}\n passive-interface GigabitEthernet0/0\n!")
            ),
            Status::Passed
        );
        assert_eq!(
            status(
                &c,
                &format!("router {mode}\n no passive-interface GigabitEthernet0/0\n!")
            ),
            Status::Failed
        );
        for line in [
            "passive-interface default",
            "passive-interface Gi0/1",
            "network 192.0.2.0",
        ] {
            assert_eq!(
                status(&c, &format!("router {mode}\n {line}\n!")),
                Status::Unverified
            );
        }
        assert_eq!(
            status(
                &c,
                &format!("router {mode}\n passive-interface Gi0/0\n no passive-interface Gi0/0\n!")
            ),
            Status::Unverified
        );
    }
}
#[test]
fn missing_conflicting_unknown_and_wrong_identity_never_gain_verification() {
    let mut c = checkpoint("OSPF Configured Router ID", "192.0.2.9");
    c.evidence.push("commands:parameter:process_id=20".into());
    let config = "router ospf 20\n router-id 192.0.2.9\n!";
    assert_eq!(
        status(&c, "router ospf 21\n router-id 192.0.2.9\n!"),
        Status::Unverified
    );
    assert_eq!(
        status(&c, "router ospf 20\n router-id invalid\n!"),
        Status::Unverified
    );
    for (unique_snapshot, unique_device, device) in [
        (false, true, "Synthetic-Router"),
        (true, false, "Synthetic-Router"),
        (true, true, "Unrelated"),
    ] {
        assert_eq!(
            validate(
                &c,
                &DeviceObservation {
                    device,
                    unique_current_snapshot: unique_snapshot,
                    unique_device,
                    configuration: config
                }
            )
            .status,
            Status::Unverified
        );
    }
    c.evidence.push("commands:parameter:process_id=21".into());
    assert_eq!(status(&c, config), Status::Unverified);
    c.evidence.clear();
    c.device_type = "Unknown".into();
    assert_eq!(status(&c, config), Status::Unverified);
    for property in ["RIP Auto Summary", "CDP Global"] {
        assert!(!commands::generate(&checkpoint(property, "1")).supported);
    }
}

#[test]
fn batch_preserves_independent_passive_ports_and_ntp_servers_and_blocks_rip_conflicts() {
    let mut one = checkpoint("OSPF Passive Interface", "passive");
    one.interface = "Gi0/0".into();
    one.evidence.push("commands:parameter:process_id=20".into());
    let mut two = one.clone();
    two.interface = "GigabitEthernet0/1".into();
    let script = commands::device_script(&[one.clone(), two.clone()], None);
    assert!(script.contains("passive-interface Gi0/0"));
    assert!(script.contains("passive-interface GigabitEthernet0/1"));
    two.interface = "GigabitEthernet0/0".into();
    two.expected = "active".into();
    assert!(!commands::device_script(&[one, two], None).contains("passive-interface"));
    let version = checkpoint("RIP Version", "1");
    let network = checkpoint("RIP Network", "2 10.0.0.0");
    let script = commands::device_script(&[version, network], None);
    assert!(!script.contains("version "), "{script}");
    assert!(!script.contains("network "));
    let one = checkpoint("NTP Server", "192.0.2.10");
    let mut two = one.clone();
    two.expected = "192.0.2.20".into();
    let script = commands::device_script(&[one.clone(), two], None);
    assert!(script.contains("ntp server 192.0.2.10"));
    assert!(script.contains("ntp server 192.0.2.20"));
    let mut two = one.clone();
    two.evidence.push("commands:parameter:version=4".into());
    assert!(!commands::device_script(&[one, two], None).contains("ntp server"));
}

#[test]
fn comparison_pipeline_scores_only_explicit_supported_synthetic_controls() {
    use pka_lab_inspector::{
        inspector::{compare, parse},
        model::Analysis,
    };
    let mut version = checkpoint("RIP Version", "2");
    version.weight = 3.0;
    let mut router_id = checkpoint("OSPF Configured Router ID", "192.0.2.9");
    router_id.weight = 2.0;
    router_id
        .evidence
        .push("commands:parameter:process_id=20".into());
    let mut missing = checkpoint("EIGRP Configured Router ID", "192.0.2.9");
    missing.weight = 5.0;
    let mut summary = checkpoint("RIP Auto Summary", "false");
    summary.weight = 7.0;
    let mut activity = Analysis {
        checkpoints: vec![version, router_id, missing, summary],
        ..Default::default()
    };
    let current=parse("<PACKETTRACER5><NETWORK><DEVICES><DEVICE><ENGINE><NAME>Synthetic-Router</NAME><TYPE>Router</TYPE></ENGINE><RUNNINGCONFIG><LINE>router rip</LINE><LINE> version 2</LINE><LINE> no auto-summary</LINE><LINE>!</LINE><LINE>router ospf 20</LINE><LINE> router-id 192.0.2.10</LINE><LINE>!</LINE></RUNNINGCONFIG></DEVICE></DEVICES></NETWORK></PACKETTRACER5>").unwrap();
    compare(&mut activity, &current);
    let score = activity.summary();
    assert_eq!((score.passed, score.failed, score.unverified), (2, 1, 1));
    assert_eq!(score.verified_score, 10.0);
    assert_eq!(score.total_score, 17.0);
}

fn switch_checkpoint(property: &str, expected: &str) -> Checkpoint {
    let mut c = checkpoint(property, expected);
    c.device = "Synthetic-Switch".into();
    c.device_type = "Switch".into();
    c.evidence.push("assessment:device-model=2960".into());
    c
}
fn lldp_port(transmit: &str, receive: &str) -> Checkpoint {
    let mut c = switch_checkpoint("LLDP Transmit", transmit);
    c.interface = "GigabitEthernet0/1".into();
    c.evidence.extend([
        format!("commands:parameter:receive={receive}"),
        "commands:parameter:global_enabled=true".into(),
        "commands:parameter:interface_mode=access".into(),
    ]);
    c
}
#[test]
fn discovery_device_scripts_block_global_disable_and_interface_enable_conflicts() {
    let global = switch_checkpoint("LLDP Global", "false");
    let port = lldp_port("true", "false");
    assert!(commands::generate(&global).supported);
    assert!(commands::generate(&port).supported);
    let host = switch_checkpoint("Hostname", "Synthetic-Switch");
    let script = commands::device_script(&[global.clone(), port, host], None);
    assert!(!script.contains("lldp run"), "{script}");
    assert!(!script.contains("lldp transmit"));
    assert!(script.contains("hostname Synthetic-Switch"));
    let script = commands::device_script(&[global, lldp_port("false", "false")], None);
    assert!(script.contains("no lldp run"));
    assert!(script.contains("no lldp transmit"));
    assert!(script.contains("no lldp receive"));
    let global = switch_checkpoint("CDP Global", "false");
    let mut port = switch_checkpoint("CDP Enable", "true");
    port.interface = "Gi0/1".into();
    port.evidence
        .push("commands:parameter:global_enabled=yes".into());
    assert!(commands::generate(&port).supported);
    let script = commands::device_script(&[global, port], None);
    assert!(!script.contains("cdp run"));
    assert!(!script.contains("cdp enable"));
}
#[test]
fn completed_lldp_sibling_requests_are_deduplicated_or_rejected_together() {
    let transmit = lldp_port("true", "false");
    let mut receive = switch_checkpoint("LLDP Receive", "false");
    receive.interface = "Gi0/1".into();
    receive.evidence.extend([
        "commands:parameter:transmit=true".into(),
        "commands:parameter:global_enabled=true".into(),
        "commands:parameter:interface_mode=access".into(),
    ]);
    let script = commands::device_script(&[transmit.clone(), receive.clone()], None);
    assert_eq!(script.matches("\nlldp transmit\n").count(), 1, "{script}");
    assert_eq!(script.matches("\nno lldp receive\n").count(), 1);
    receive.expected = "true".into();
    receive
        .evidence
        .retain(|s| !s.starts_with("commands:parameter:transmit="));
    receive
        .evidence
        .push("commands:parameter:transmit=false".into());
    let script = commands::device_script(&[transmit, receive], None);
    assert!(!script.contains("lldp transmit"));
    assert!(!script.contains("lldp receive"));
}

#[test]
fn generic_assessment_labels_require_one_protocol_category_not_exact_ios_names() {
    let mut c = checkpoint("Version", "2");
    c.path = "Network / Synthetic-Router / RIP / Version".into();
    assert!(
        commands::generate(&c)
            .text
            .contains("router rip\nversion 2\n")
    );
    assert_eq!(c.property, "Version");
    c = checkpoint("Router ID", "192.0.2.9");
    c.path = "Network / Synthetic-Router / OSPF / 1 / Router ID".into();
    assert!(!commands::generate(&c).supported); // numeric branch 1 is not a process ID
    c.evidence.push("commands:parameter:process_id=20".into());
    assert!(
        commands::generate(&c)
            .text
            .contains("router ospf 20\nrouter-id 192.0.2.9\n")
    );
    assert_eq!(
        status(&c, "router ospf 20\n router-id 192.0.2.9\n!"),
        Status::Unverified
    );
    c.property = "Configured Router ID".into();
    assert_eq!(
        status(&c, "router ospf 20\n router-id 192.0.2.9\n!"),
        Status::Passed
    );
    for path in [
        "Network / Synthetic-Router / Settings / Router ID",
        "Network / Synthetic-Router / OSPF / EIGRP / Router ID",
        "Network / Synthetic-Router / IPv6 / OSPF / Router ID",
        "Network / Synthetic-Router / OSPFv3 / Router ID",
    ] {
        c.path = path.into();
        assert!(!commands::generate(&c).supported, "{path}");
    }
    c = checkpoint("Router ID", "192.0.2.9");
    c.device = "OSPF".into();
    c.path = "Network / OSPF / Settings / Router ID".into();
    c.evidence.push("commands:parameter:process_id=20".into());
    assert!(!commands::generate(&c).supported); // device name alone does not identify a protocol
}
