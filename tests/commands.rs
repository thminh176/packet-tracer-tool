use pka_lab_inspector::{
    commands::{device_script, generate},
    model::Checkpoint,
};

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

#[test]
fn supported_families_have_scoped_commands() {
    for (property, value, expected) in [
        ("Hostname", "Branch-1", "hostname Branch-1"),
        (
            "Port Status",
            "up",
            "interface GigabitEthernet0/0\nno shutdown\nexit",
        ),
        ("Shutdown", "true", "shutdown"),
        (
            "IPv4 Address",
            "192.0.2.1 255.255.255.0",
            "ip address 192.0.2.1 255.255.255.0",
        ),
        (
            "IPv6 Address",
            "2001:db8::1/64",
            "ipv6 address 2001:db8::1/64",
        ),
        (
            "IPv6 Link Local",
            "fe80::1",
            "ipv6 address fe80::1 link-local",
        ),
        (
            "Default Gateway",
            "192.0.2.254",
            "ip default-gateway 192.0.2.254",
        ),
        (
            "Static Route",
            "0.0.0.0 0.0.0.0 192.0.2.254 5",
            "ip route 0.0.0.0 0.0.0.0 192.0.2.254 5",
        ),
        (
            "IPv6 Static Route",
            "::/0 2001:db8::2",
            "ipv6 route ::/0 2001:db8::2",
        ),
        ("VLAN", "20 Staff", "vlan 20\nname Staff\nexit"),
        ("Switchport Mode", "access", "switchport mode access"),
        ("Access VLAN", "20", "switchport access vlan 20"),
        ("Native VLAN", "99", "switchport trunk native vlan 99"),
        (
            "Allowed VLANs",
            "10 20 30",
            "switchport trunk allowed vlan 10,20,30",
        ),
        ("Port Security", "enabled", "switchport port-security"),
        (
            "Port Security Maximum",
            "2",
            "switchport port-security maximum 2",
        ),
        (
            "Port Security Violation",
            "restrict",
            "switchport port-security violation restrict",
        ),
        (
            "Port Security MAC",
            "0011.2233.4455",
            "switchport port-security mac-address 0011.2233.4455",
        ),
        (
            "Port Security Sticky",
            "true",
            "switchport port-security mac-address sticky",
        ),
        (
            "Standard ACL",
            "10 permit 192.0.2.0 0.0.0.255",
            "access-list 10 permit 192.0.2.0 0.0.0.255",
        ),
        (
            "Extended ACL",
            "101 deny tcp 192.0.2.0 0.0.0.255 198.51.100.0 0.0.0.255",
            "access-list 101 deny tcp 192.0.2.0 0.0.0.255 198.51.100.0 0.0.0.255",
        ),
        ("ACL Attachment", "101 in", "ip access-group 101 in"),
        ("NAT Role", "inside", "ip nat inside"),
        (
            "NAT Static",
            "192.0.2.1 198.51.100.1",
            "ip nat inside source static 192.0.2.1 198.51.100.1",
        ),
        (
            "NAT Overload",
            "10 GigabitEthernet0/1",
            "ip nat inside source list 10 interface GigabitEthernet0/1 overload",
        ),
        (
            "DHCP Pool",
            "LAN 192.0.2.0 255.255.255.0 192.0.2.1 192.0.2.53 example.test",
            "ip dhcp pool LAN\nnetwork 192.0.2.0 255.255.255.0\ndefault-router 192.0.2.1\ndns-server 192.0.2.53\ndomain-name example.test\nexit",
        ),
        (
            "DHCP Excluded Address",
            "192.0.2.1 192.0.2.10",
            "ip dhcp excluded-address 192.0.2.1 192.0.2.10",
        ),
        (
            "OSPF Network",
            "1 192.0.2.0 0.0.0.255 0",
            "router ospf 1\nnetwork 192.0.2.0 0.0.0.255 area 0\nexit",
        ),
        (
            "RIP Network",
            "2 10.0.0.0",
            "router rip\nversion 2\nnetwork 10.0.0.0\nexit",
        ),
        (
            "EIGRP Network",
            "100 192.0.2.0 0.0.0.255",
            "router eigrp 100\nnetwork 192.0.2.0 0.0.0.255\nexit",
        ),
        ("Enable Secret", "test-secret", "enable secret test-secret"),
        (
            "Password Encryption",
            "enabled",
            "service password-encryption",
        ),
        (
            "Console Password",
            "test-secret",
            "line console 0\npassword test-secret\nlogin\nexit",
        ),
        (
            "VTY Password",
            "0 4 test-secret",
            "line vty 0 4\npassword test-secret\nlogin\nexit",
        ),
        (
            "VTY SSH",
            "0 4",
            "line vty 0 4\nlogin local\ntransport input ssh\nexit",
        ),
        (
            "Local User",
            "admin 15 test-secret",
            "username admin privilege 15 secret test-secret",
        ),
    ] {
        let result = generate(&checkpoint(property, value));
        assert!(result.supported, "{property}: {}", result.explanation);
        assert!(!result.gui);
        assert!(result.text.starts_with("enable\nconfigure terminal\n"));
        assert!(
            result.text.contains(expected),
            "{property}: {}",
            result.text
        );
        assert!(result.text.ends_with("end\n"));
    }
    assert_eq!(
        generate(&checkpoint("save", "true")).text,
        "enable\ncopy running-config startup-config\n"
    );
}

#[test]
fn incomplete_ambiguous_and_sentinel_values_are_explained() {
    for (property, value) in [
        ("IPv4 Address", "192.0.2.1"),
        ("IPv4 Address", "192.0.2.1 255.0.255.0"),
        ("IPv6 Address", "2001:db8::1"),
        ("IPv6 Link Local", "2001:db8::1"),
        ("Port Status", "1"),
        ("Switchport Mode", "2"),
        ("Port Security Violation", "0"),
        ("VLAN", "65535"),
        ("OSPF Network", "65535 192.0.2.0 0.0.0.255 0"),
        ("OSPF Network", "1 192.0.2.0 0.0.0.255 4294967295"),
        ("Static Route", "192.0.2.1 255.255.255.0 198.51.100.1"),
        ("NAT Overload", "10"),
        ("DHCP Pool", "LAN"),
        ("RIP Network", "10.0.0.0"),
        ("RIP Network", "2 10.1.0.0"),
        ("EIGRP Network", "100"),
        ("VTY Password", "4 0 password"),
        ("Hostname", ""),
        ("Hostname", "unknown"),
        ("Hostname", "-1"),
        ("Hostname", "4294967295"),
        ("Save", "1"),
        ("Unrecognized Field", "hello"),
        ("IPv6 Static Route", "::/0 fe80::1"),
    ] {
        let result = generate(&checkpoint(property, value));
        assert!(!result.supported, "{property} {value}: {}", result.text);
        assert!(result.text.is_empty());
        assert!(!result.explanation.is_empty());
    }
}

#[test]
fn injection_in_every_emitted_field_is_rejected() {
    for bad in [
        "R1\nreload",
        "R1\rend",
        "R1;reload",
        "R1!",
        "R1\tend",
        "R1\u{1b}",
        "R1|erase",
        "R1$(reload)",
    ] {
        let mut c = checkpoint("Hostname", bad);
        assert!(!generate(&c).supported);
        c = checkpoint("Hostname", "good");
        c.device = bad.into();
        assert!(!generate(&c).supported);
        c = checkpoint("Hostname", "good");
        c.property = bad.into();
        assert!(!generate(&c).supported);
        c = checkpoint("IPv4 Address", "192.0.2.1 255.255.255.0");
        c.interface = bad.into();
        assert!(!generate(&c).supported);
        assert!(!device_script(&[c], None).contains("reload"));
    }
}

#[test]
fn scripts_keep_devices_and_contexts_separate_and_save_last() {
    let mut items = vec![
        checkpoint("Save", "true"),
        checkpoint("Access VLAN", "20"),
        checkpoint("VLAN", "20 Staff"),
        checkpoint("Hostname", "router-one"),
        checkpoint("OSPF Network", "1 192.0.2.0 0.0.0.255 0"),
    ];
    let mut second = checkpoint("Hostname", "router-two");
    second.device = "R2".into();
    items.push(second);
    let script = device_script(&items, None);
    assert_eq!(script.matches("enable\n").count(), 2);
    assert!(script.find("vlan 20\n").unwrap() < script.find("switchport access vlan").unwrap());
    assert!(script.contains("switchport access vlan 20\nexit\nrouter ospf 1"));
    assert!(script.contains("end\ncopy running-config startup-config\n\n! Device: R2"));
    let selected = device_script(&items, Some("R2"));
    assert!(selected.contains("router-two"));
    assert!(!selected.contains("router-one"));
    assert!(device_script(&items, Some("absent")).is_empty());
}

#[test]
fn gui_endpoints_never_receive_ios_scripts() {
    for kind in ["PC", "Server-PT", "Laptop"] {
        for (property, value) in [
            ("IPv4 Address", "192.0.2.5 255.255.255.0"),
            ("Default Gateway", "192.0.2.1"),
            ("DNS Server", "192.0.2.53"),
            ("IPv6 Address", "2001:db8::5/64"),
        ] {
            let mut c = checkpoint(property, value);
            c.device_type = kind.into();
            let result = generate(&c);
            assert!(result.supported && result.gui);
            assert!(result.text.contains("Desktop > IP Configuration"));
            let script = device_script(&[c], None);
            assert!(!script.contains("configure terminal"));
            assert!(script.lines().all(|l| l.is_empty() || l.starts_with('!')));
        }
    }
}

#[test]
fn missing_interface_does_not_guess_and_unknown_is_only_a_comment() {
    let mut c = checkpoint("IPv4 Address", "192.0.2.1 255.255.255.0");
    c.interface.clear();
    assert!(!generate(&c).supported);
    let c = checkpoint("Unrecognized Field", "5");
    let script = device_script(&[c], None);
    assert!(script.contains("Unsupported"));
    assert!(!script.contains("configure terminal"));
}

#[test]
fn interface_dependencies_are_ordered_without_reordering_acl_rules() {
    let items = vec![
        checkpoint("Port Status", "up"),
        checkpoint("Port Security Maximum", "2"),
        checkpoint("Port Security", "true"),
        checkpoint("Switchport Mode", "access"),
        checkpoint("Standard ACL", "10 deny 192.0.2.0 0.0.0.255"),
        checkpoint("Standard ACL", "10 permit 198.51.100.0 0.0.0.255"),
    ];
    let script = device_script(&items, None);
    for (before, after) in [
        ("switchport mode access", "switchport port-security\n"),
        (
            "switchport port-security\n",
            "switchport port-security maximum",
        ),
        ("switchport port-security maximum", "no shutdown"),
        ("access-list 10 deny", "access-list 10 permit"),
    ] {
        assert!(
            script.find(before).unwrap() < script.find(after).unwrap(),
            "{script}"
        );
    }
}

#[test]
fn explicit_unique_sibling_evidence_completes_addresses() {
    let mut c = checkpoint("IP Address", "192.0.2.5");
    assert!(!generate(&c).supported);
    c.evidence.push("commands:ipv4-mask=255.255.255.0".into());
    assert!(
        generate(&c)
            .text
            .contains("ip address 192.0.2.5 255.255.255.0")
    );
    c.device_type = "PC-PT".into();
    assert!(generate(&c).text.contains("192.0.2.5 255.255.255.0"));
    c.evidence.push("commands:ipv4-mask=255.255.0.0".into());
    assert!(!generate(&c).supported);
    c.evidence = vec!["commands:ipv4-mask=65535".into()];
    assert!(!generate(&c).supported);
    let mut c = checkpoint("IPv6 Address", "2001:db8::1");
    c.evidence.push("commands:ipv6-prefix=64".into());
    assert!(generate(&c).text.contains("2001:db8::1/64"));
    c.evidence = vec!["commands:ipv6-prefix=4294967295".into()];
    assert!(!generate(&c).supported);
}

#[test]
fn unsupported_assessment_types_reject_cli_and_gui() {
    for kind in ["Router", "PC-PT", "Server"] {
        let mut c = checkpoint("IP Address", "192.0.2.5 255.255.255.0");
        c.device_type = kind.into();
        for marker in [
            "assessment:unsupported checkType=variable",
            "assessment:ambiguous",
            "assessment:inventory",
        ] {
            c.evidence = vec![marker.into()];
            let result = generate(&c);
            assert!(!result.supported && result.text.is_empty());
            assert!(!device_script(&[c.clone()], None).contains("192.0.2.5"));
        }
    }
}

#[test]
fn scripts_skip_conflicting_requirements_including_gui_and_aliases() {
    for kind in ["Router", "PC-PT"] {
        let mut first = checkpoint("IP Address", "192.0.2.1 255.255.255.0");
        first.device_type = kind.into();
        let mut second = checkpoint("IPv4 Address", "192.0.2.2 255.255.255.0");
        second.device_type = kind.into();
        let script = device_script(&[first, second], None);
        assert!(script.contains("conflicting"));
        assert!(!script.contains("192.0.2."));
    }
    let mut second = checkpoint("Hostname", "second");
    second.interface = "GigabitEthernet0/1".into();
    let script = device_script(&[checkpoint("Host Name", "first"), second], None);
    assert!(!script.contains("hostname "));
    let script = device_script(
        &[
            checkpoint("Port Status", "up"),
            checkpoint("Shutdown", "true"),
        ],
        None,
    );
    assert!(!script.contains("shutdown\n"));
    for (property, first, second, forbidden) in [
        (
            "Static Route",
            "0.0.0.0 0.0.0.0 192.0.2.1",
            "0.0.0.0 0.0.0.0 192.0.2.2",
            "ip route ",
        ),
        (
            "Standard ACL",
            "10 permit 192.0.2.0 0.0.0.255",
            "10 deny 192.0.2.0 0.0.0.255",
            "access-list ",
        ),
        ("VLAN", "20 Staff", "20 Guests", "vlan 20\n"),
        ("RIP Network", "1 10.0.0.0", "2 192.0.2.0", "router rip"),
    ] {
        let script = device_script(
            &[checkpoint(property, first), checkpoint(property, second)],
            None,
        );
        assert!(script.contains("conflicting"));
        assert!(!script.contains(forbidden), "{script}");
    }
}
