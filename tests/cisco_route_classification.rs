//! Synthetic protocol-context regressions; raw RouteN labels are not IOS commands.
use pka_lab_inspector::{
    cisco, commands,
    inspector::{compare, parse},
    model::{Analysis, Checkpoint, Status},
};
fn ospf() -> Checkpoint {
    Checkpoint {
        device: "Lab-Router".into(),
        device_type: "Router".into(),
        property: "Route0".into(),
        expected: "198.51.100.0 0.0.0.255 2".into(),
        path: "Network / Lab-Router / OSPF / 20 / Networks / Route0".into(),
        ..Default::default()
    }
}
#[test]
fn ospf_indexed_routes_are_never_static_and_require_explicit_process_evidence() {
    let mut c = ospf();
    assert!(commands::is_ospf_network_property(&c.property, &c.path));
    assert!(!commands::is_static_route_property(&c.property, &c.path));
    assert!(!commands::is_ipv6_static_route_property(
        &c.property,
        &c.path
    ));
    assert!(!commands::generate(&c).supported); // Numeric ancestor alone is not process evidence.
    assert_eq!(
        cisco::mapping::request(&c, &c.expected).unwrap().family,
        "router.ospf.network"
    );
    c.evidence.push("commands:ospf-process=20".into());
    let command = commands::generate(&c);
    assert!(command.supported, "{}", command.explanation);
    assert!(command.text.contains("router ospf 20\n"));
    assert!(
        command
            .text
            .contains("network 198.51.100.0 0.0.0.255 area 2\n")
    );
    assert!(!command.text.contains("ip route"));
    assert_eq!(c.status, Status::Unverified);
}
#[test]
fn dynamic_protocol_branches_and_router_id_are_not_static_routes() {
    for protocol in ["OSPF", "OSPFv2", "OSPFv3", "RIP", "EIGRP", "BGP"] {
        let path = format!("Network / Lab-Router / {protocol} / Networks / Route0");
        assert!(!commands::is_static_route_property("Route0", &path));
        assert!(!commands::is_ipv6_static_route_property("Route0", &path));
    }
    assert!(!commands::is_static_route_property(
        "Router ID",
        "Network / Lab-Router"
    ));
    assert!(!commands::is_ospf_network_property(
        "Router ID",
        &ospf().path
    ));
}
#[test]
fn device_names_and_raw_route_labels_do_not_replace_protocol_context() {
    assert!(commands::is_static_route_property(
        "Route0",
        "Network / OSPF-Router / Static Routes / Route0"
    ));
    assert!(commands::is_ipv6_static_route_property(
        "Route0",
        "Network / Lab-Router / Routesv6 / Route0"
    ));
    assert!(!commands::is_ospf_network_property(
        "Route0",
        "Network / OSPF-Router / Static Routes / Route0"
    ));
}
#[test]
fn indexed_ospf_comparison_uses_ospf_process_not_static_route_table() {
    let mut c = ospf();
    c.evidence.push("commands:ospf-process=20".into());
    let mut analysis = Analysis {
        checkpoints: vec![c],
        ..Default::default()
    };
    let progress = parse("<PACKETTRACER5><NETWORK><DEVICES><DEVICE><ENGINE><NAME>Lab-Router</NAME><TYPE>Router</TYPE></ENGINE><RUNNINGCONFIG><LINE>router ospf 20</LINE><LINE> network 198.51.100.0 0.0.0.255 area 2</LINE><LINE>exit</LINE></RUNNINGCONFIG></DEVICE></DEVICES></NETWORK></PACKETTRACER5>").unwrap();
    compare(&mut analysis, &progress);
    assert_eq!(
        analysis.checkpoints[0].status,
        Status::Passed,
        "{}",
        analysis.checkpoints[0].explanation
    );
    analysis.checkpoints[0]
        .evidence
        .retain(|e| !e.starts_with("commands:ospf-process="));
    compare(&mut analysis, &progress);
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
}
