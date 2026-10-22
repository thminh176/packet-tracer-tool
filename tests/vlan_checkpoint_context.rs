use pka_lab_inspector::{
    commands,
    model::{Checkpoint, Status},
};
fn checkpoint(property: &str, expected: &str, path: &str) -> Checkpoint {
    Checkpoint {
        device: "Lab-Router".into(),
        device_type: "Router".into(),
        interface: "GigabitEthernet0/0.77".into(),
        property: property.into(),
        expected: expected.into(),
        path: path.into(),
        ..Default::default()
    }
}
#[test]
fn selected_dot1q_vlan_id_uses_requirement_not_subinterface_suffix() {
    let c = checkpoint(
        "VLAN ID",
        "42",
        "Network / Lab-Router / Ports / GigabitEthernet0/0.77 / 802.1Q / VLAN ID",
    );
    let command = commands::generate(&c);
    assert!(command.supported, "{}", command.explanation);
    assert!(command.text.contains("encapsulation dot1Q 42\n"));
    assert!(!command.text.contains("dot1Q 77"));
    assert_eq!(c.status, Status::Unverified);
    let mut unrelated = c.clone();
    unrelated.path = "Network / Lab-Router / Other / VLAN ID".into();
    assert!(!commands::generate(&unrelated).supported);
}
#[test]
fn encapsulation_object_requires_explicit_vlan_and_cannot_mine_the_interface_name() {
    let mut c = checkpoint(
        "Encapsulation",
        "EthernetEncapProcess",
        "Network / Lab-Router / Ports / GigabitEthernet0/0.77 / Encapsulation",
    );
    assert!(!commands::generate(&c).supported);
    c.evidence.push("commands:parameter:vlan=42".into());
    assert!(
        commands::generate(&c)
            .text
            .contains("encapsulation dot1Q 42\n")
    );
    c.evidence.push("commands:parameter:vlan=43".into());
    assert!(!commands::generate(&c).supported);
}
#[test]
fn vlan_name_assignment_preserves_explicit_branch_and_rejects_conflicting_metadata() {
    let mut c = checkpoint(
        "VLAN Name",
        "Staff",
        "Network / Lab-Switch / VLANS / 42 / VLAN Name",
    );
    c.device_type = "Switch".into();
    c.interface.clear();
    let command = commands::generate(&c);
    assert!(command.supported, "{}", command.explanation);
    assert!(command.text.contains("vlan 42\nname Staff\n"));
    c.evidence.push("commands:parameter:name=Guests".into());
    assert!(!commands::generate(&c).supported);
}

#[test]
fn trunk_assignments_are_conditional_and_reject_explicit_conflicts() {
    for (property, value, line, conflict) in [
        (
            "Native VLAN",
            "42",
            "switchport trunk native vlan 42",
            "peer_native_vlan=43",
        ),
        (
            "Allowed VLANs",
            "10,20-22",
            "switchport trunk allowed vlan 10,20-22",
            "operation=add",
        ),
    ] {
        let mut c = checkpoint(property, value, "Network / Lab-Switch / Ports");
        c.device_type = "Switch".into();
        c.interface = "GigabitEthernet0/1".into();
        let command = commands::generate(&c);
        assert!(command.supported, "{}", command.explanation);
        assert!(command.text.lines().any(|text| text == line));
        assert!(!command.text.contains("switchport mode"));
        assert_eq!(c.status, Status::Unverified);
        c.evidence.push(format!("commands:parameter:{conflict}"));
        assert!(!commands::generate(&c).supported);
    }
}
