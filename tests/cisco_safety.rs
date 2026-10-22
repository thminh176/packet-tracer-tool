use pka_lab_inspector::{
    commands,
    inspector::{compare, parse},
    model::{Checkpoint, Status},
};
fn check(property: &str, value: &str) -> Checkpoint {
    Checkpoint {
        device: "R1".into(),
        device_type: "Router".into(),
        property: property.into(),
        expected: value.into(),
        ..Default::default()
    }
}
fn verify(property: &str, value: &str, devices: &str) -> Checkpoint {
    let mut a = pka_lab_inspector::model::Analysis {
        checkpoints: vec![check(property, value)],
        ..Default::default()
    };
    let progress = parse(&format!(
        "<PACKETTRACER5><NETWORK><DEVICES>{devices}</DEVICES></NETWORK></PACKETTRACER5>"
    ))
    .unwrap();
    compare(&mut a, &progress);
    a.checkpoints.remove(0)
}
fn device(name: &str, lines: &str) -> String {
    format!(
        "<DEVICE><ENGINE><NAME>{name}</NAME><TYPE>Router</TYPE></ENGINE><RUNNINGCONFIG>{lines}</RUNNINGCONFIG></DEVICE>"
    )
}
#[test]
fn ospf_never_defaults_process_or_area() {
    let mut c = check("OSPF Network", "10.0.0.0 0.255.255.255 0");
    assert!(!commands::generate(&c).supported);
    c.evidence.push("commands:ospf-process=20".into());
    let cmd = commands::generate(&c);
    assert!(cmd.supported, "{}", cmd.explanation);
    assert!(cmd.text.contains("router ospf 20"));
    assert!(!cmd.text.contains("router ospf 1\n"));
    let c = check("OSPF Network", "10.0.0.0/8");
    assert!(!commands::generate(&c).supported);
}
#[test]
fn unrelated_or_duplicate_device_config_never_proves_route_absent() {
    let empty = "<LINE>hostname R2</LINE>";
    let d = device("R2", empty);
    assert_eq!(
        verify("Static Route", "10.0.0.0 255.0.0.0 192.0.2.1", &d).status,
        Status::Unverified
    );
    let d = format!("{}{}", device("R1", empty), device("R1", empty));
    assert_eq!(
        verify("Static Route", "10.0.0.0 255.0.0.0 192.0.2.1", &d).status,
        Status::Unverified
    );
}
#[test]
fn uniquely_owned_known_config_can_prove_missing_complete_route() {
    let d = device("R1", "<LINE>hostname R1</LINE>");
    assert_eq!(
        verify("Static Route", "10.0.0.0 255.0.0.0 192.0.2.1", &d).status,
        Status::Failed
    );
    assert_eq!(
        verify("Static Route", "10.0.0.0/8", &d).status,
        Status::Unverified
    );
    assert_eq!(
        verify("OSPF Network", "10.0.0.0 0.255.255.255 0", &d).status,
        Status::Unverified
    );
}
#[test]
fn incomplete_requirements_never_pass_against_existing_routes() {
    let d = device("R1", "<LINE>ip route 10.0.0.0 255.0.0.0 192.0.2.1</LINE>");
    assert_eq!(
        verify("Static Route", "10.0.0.0/8", &d).status,
        Status::Unverified
    );
}
#[test]
fn opaque_or_unknown_configuration_does_not_prove_absence() {
    let d = device(
        "R1",
        "<LINE>banner motd #</LINE><LINE>hostname R1</LINE><LINE>#</LINE>",
    );
    assert_eq!(
        verify("Static Route", "10.0.0.0 255.0.0.0 192.0.2.1", &d).status,
        Status::Unverified
    );
}
#[test]
fn route_distance_and_ospf_process_are_semantic() {
    let d = device("R1", "<LINE>ip route 10.0.0.0 255.0.0.0 192.0.2.1 5</LINE>");
    assert_eq!(
        verify("Static Route", "10.0.0.0 255.0.0.0 192.0.2.1 1", &d).status,
        Status::Failed
    );
    let d = device(
        "R1",
        "<LINE>router ospf 20</LINE><LINE> network 10.0.0.0 0.255.255.255 area 0</LINE><LINE>!</LINE>",
    );
    assert_eq!(
        verify("OSPF Network", "10 10.0.0.0 0.255.255.255 0", &d).status,
        Status::Failed
    );
    assert_eq!(
        verify("OSPF Network", "20 10.0.0.0 0.255.255.255 0", &d).status,
        Status::Passed
    );
}
