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
#[test]
fn typed_switch_validation_is_integrated_without_claiming_operational_state() {
    let mut activity = pka_lab_inspector::model::Analysis {
        checkpoints: vec![Checkpoint {
            device: "R1".into(),
            interface: "Gi0/1".into(),
            device_type: "Switch".into(),
            property: "Access VLAN".into(),
            expected: "20".into(),
            weight: 3.,
            ..Default::default()
        }],
        ..Default::default()
    };
    let d = device(
        "R1",
        "<LINE>interface GigabitEthernet0/1</LINE><LINE> switchport mode access</LINE><LINE> switchport access vlan 20</LINE><LINE>!</LINE>",
    );
    let progress = parse(&format!(
        "<PACKETTRACER5><NETWORK><DEVICES>{d}</DEVICES></NETWORK></PACKETTRACER5>"
    ))
    .unwrap();
    compare(&mut activity, &progress);
    assert_eq!(activity.checkpoints[0].status, Status::Passed);
    assert_eq!(activity.checkpoints[0].current.as_deref(), Some("20"));
    assert_eq!(activity.summary().verified_score, 3.);
    let d = device(
        "R1",
        "<LINE>interface GigabitEthernet0/1</LINE><LINE> switchport access vlan 20</LINE><LINE>!</LINE>",
    );
    let progress = parse(&format!(
        "<PACKETTRACER5><NETWORK><DEVICES>{d}</DEVICES></NETWORK></PACKETTRACER5>"
    ))
    .unwrap();
    compare(&mut activity, &progress);
    assert_eq!(activity.checkpoints[0].status, Status::Unverified);
    assert_eq!(activity.summary().verified_score, 0.);
}
#[test]
fn static_ecmp_and_floating_routes_coexist_in_scripts() {
    let a = check("Static Route", "10.0.0.0 255.0.0.0 192.0.2.1");
    let b = check("Static Route", "10.0.0.0 255.0.0.0 192.0.2.2");
    let d = check("Static Route", "10.0.0.0 255.0.0.0 192.0.2.1 5");
    let script = commands::device_script(&[a, b, d], None);
    assert_eq!(script.matches("ip route ").count(), 3, "{script}");
    assert!(!script.contains("conflicting"));
}
#[test]
fn mixed_profiles_and_conflicting_ospf_metadata_are_not_repairs() {
    let mut a = check("Hostname", "RouterLab");
    let mut b = check("Hostname", "SwitchLab");
    b.device_type = "Switch".into();
    assert!(!commands::device_script(&[a.clone(), b], None).contains("hostname "));
    a.property = "OSPF Network".into();
    a.expected = "10.0.0.0 0.255.255.255 0".into();
    a.evidence = vec![
        "commands:ospf-process=10".into(),
        "commands:ospf-process=20".into(),
    ];
    assert!(!commands::generate(&a).supported);
}
