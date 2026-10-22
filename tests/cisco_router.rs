use pka_lab_inspector::cisco::{
    self, CommandRequest, ConfigMode, DeviceProfile, SupportState, router,
};
use std::collections::HashSet;

fn request(family: &str, pairs: &[(&str, &str)]) -> CommandRequest {
    CommandRequest {
        family: family.into(),
        device: "R1".into(),
        profile: DeviceProfile::Router2911,
        parameters: pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}
fn script(family: &str, pairs: &[(&str, &str)]) -> String {
    cisco::render(&router::generate(&request(family, pairs)).unwrap())
}
#[test]
fn registry_has_unique_documented_router_families_with_bilingual_fields() {
    let specs = router::specs();
    assert_eq!(specs.len(), 18);
    let mut ids = HashSet::new();
    for spec in specs {
        assert!(ids.insert(spec.id), "duplicate {}", spec.id);
        assert!(spec.id.starts_with("router."));
        assert!(spec.source.starts_with("https://www.cisco.com/"));
        assert!(!spec.software.is_empty());
        assert!(!spec.mode.is_empty());
        assert!(!spec.name_en.is_empty() && !spec.name_vi.is_empty());
        assert!(!spec.prerequisites.is_empty() && !spec.verification.is_empty());
        assert!(!spec.semantic_validation); // Canonical registry declares only tested validator subsets.
        for parameter in spec.parameters {
            assert!(!parameter.description_en.is_empty() && !parameter.description_vi.is_empty());
        }
    }
}
#[test]
fn interface_addressing_preserves_host_addresses_and_explicit_modes() {
    assert_eq!(
        script(
            "router.interface.ipv4",
            &[
                ("interface", "GigabitEthernet0/0"),
                ("address", "192.0.2.1"),
                ("mask", "255.255.255.0")
            ]
        ),
        "enable\nconfigure terminal\ninterface GigabitEthernet0/0\nip address 192.0.2.1 255.255.255.0\nend\n"
    );
    assert!(
        script(
            "router.interface.ipv6",
            &[("interface", "Gi0/0"), ("address", "2001:db8::1234/64")]
        )
        .contains("ipv6 address 2001:db8::1234/64")
    );
    assert!(
        script(
            "router.interface.link_local",
            &[("interface", "Gi0/0"), ("address", "fe80::1")]
        )
        .contains("ipv6 address fe80::1 link-local")
    );
    assert!(
        script(
            "router.interface.ipv4",
            &[
                ("interface", "Serial0/0"),
                ("address", "192.0.2.0"),
                ("mask", "255.255.255.254")
            ]
        )
        .contains("ip address 192.0.2.0 255.255.255.254")
    );
    let mut bad = request(
        "router.interface.ipv4",
        &[
            ("interface", "Gi0/0"),
            ("address", "192.0.2.0"),
            ("mask", "255.255.255.0"),
        ],
    );
    assert!(router::generate(&bad).is_err());
    bad.parameters.insert("address".into(), "192.0.2.1".into());
    bad.parameters.insert("mask".into(), "255.0.255.0".into());
    assert!(router::generate(&bad).is_err());
    bad.parameters.insert("mask".into(), "255.255.255.0".into());
    bad.profile = DeviceProfile::Catalyst3560;
    assert!(router::generate(&bad).is_err());
    assert!(
        router::generate(&request(
            "router.interface.link_local",
            &[("interface", "Gi0/0"), ("address", "2001:db8::1")]
        ))
        .is_err()
    );
}
#[test]
fn explicit_state_description_and_dot1q_do_not_add_hidden_commands() {
    assert!(
        script(
            "router.interface.shutdown",
            &[("interface", "Gi0/0"), ("state", "shutdown")]
        )
        .contains("\nshutdown\n")
    );
    assert!(
        script(
            "router.interface.description",
            &[("interface", "Gi0/0"), ("description", "LAN uplink")]
        )
        .contains("description LAN uplink")
    );
    let sub = script(
        "router.interface.dot1q",
        &[
            ("interface", "Gi0/0.10"),
            ("vlan", "10"),
            ("native", "true"),
        ],
    );
    assert!(sub.contains("interface Gi0/0.10\nencapsulation dot1Q 10 native\n"));
    assert!(!sub.contains("no shutdown"));
    for interface in ["Gi0/0", "Serial0/0.10", "Gi0/0.0", "Gi0//0.10"] {
        assert!(
            router::generate(&request(
                "router.interface.dot1q",
                &[("interface", interface), ("vlan", "10")]
            ))
            .is_err(),
            "{interface}"
        );
    }
    assert!(
        router::generate(&request(
            "router.interface.dot1q",
            &[("interface", "Gi0/0.10"), ("vlan", "4095")]
        ))
        .is_err()
    );
}
#[test]
fn dynamic_routing_requires_explicit_process_metadata() {
    let ospf = script(
        "router.ospf.network",
        &[
            ("process_id", "42"),
            ("network", "10.0.0.0/24"),
            ("area", "0.0.0.2"),
        ],
    );
    assert!(ospf.contains("router ospf 42\nnetwork 10.0.0.0 0.0.0.255 area 0.0.0.2"));
    for pairs in [
        vec![("network", "10.0.0.0/24"), ("area", "0")],
        vec![
            ("process_id", "0"),
            ("network", "10.0.0.0/24"),
            ("area", "0"),
        ],
        vec![
            ("process_id", "1"),
            ("network", "10.0.0.0/24"),
            ("area", "bad"),
        ],
        vec![
            ("process_id", "1"),
            ("network", "10.0.0.1/24"),
            ("area", "0"),
        ],
    ] {
        assert!(router::generate(&request("router.ospf.network", &pairs)).is_err());
    }
    assert!(
        script(
            "router.eigrp.network",
            &[("as_number", "100"), ("network", "10.0.0.0/24")]
        )
        .contains("router eigrp 100\nnetwork 10.0.0.0 0.0.0.255")
    );
    assert!(
        router::generate(&request(
            "router.eigrp.network",
            &[("network", "10.0.0.0/24")]
        ))
        .is_err()
    );
    let rip = script(
        "router.rip.network",
        &[("version", "2"), ("network", "10.0.0.0/8")],
    );
    assert!(rip.contains("router rip\nversion 2\nnetwork 10.0.0.0"));
    assert!(!rip.contains("auto-summary"));
    for network in ["10.0.0.0/24", "10.0.0.0", "0.0.0.0/8", "224.0.0.0/24"] {
        assert!(
            router::generate(&request(
                "router.rip.network",
                &[("version", "2"), ("network", network)]
            ))
            .is_err()
        );
    }
}
#[test]
fn static_routes_keep_distance_and_ipv6_scope() {
    assert!(
        script(
            "router.route.ipv4",
            &[("route", "0.0.0.0-0-192.0.2.2-0-20")]
        )
        .contains("ip route 0.0.0.0 0.0.0.0 192.0.2.2 20")
    );
    assert!(
        script(
            "router.route.ipv6",
            &[("route", "2001:db8::/64 GigabitEthernet0/0 fe80::2 10")]
        )
        .contains("ipv6 route 2001:db8::/64 GigabitEthernet0/0 fe80::2 10")
    );
    for route in [
        "10.0.0.0-8",
        "10.0.0.0-8-192.0.2.2-7-1",
        "10.0.0.0 255.0.0.0 192.0.2.2 bad",
    ] {
        assert!(router::generate(&request("router.route.ipv4", &[("route", route)])).is_err());
    }
    assert!(
        router::generate(&request(
            "router.route.ipv6",
            &[("route", "2001:db8::/64 fe80::2")]
        ))
        .is_err()
    );
    assert!(
        script("router.ipv6.routing", &[("enabled", "true")]).contains("\nipv6 unicast-routing\n")
    );
}
#[test]
fn acl_and_nat_use_small_validated_subsets() {
    assert!(
        script(
            "router.acl.standard",
            &[
                ("number", "10"),
                ("action", "permit"),
                ("source", "192.0.2.0 0.0.0.255")
            ]
        )
        .contains("access-list 10 permit 192.0.2.0 0.0.0.255")
    );
    assert!(
        router::generate(&request(
            "router.acl.standard",
            &[("number", "100"), ("action", "permit"), ("source", "any")]
        ))
        .is_err()
    );
    assert!(
        router::generate(&request(
            "router.acl.standard",
            &[
                ("number", "10"),
                ("action", "permit"),
                ("source", "any log extra")
            ]
        ))
        .is_err()
    );
    let apply = router::generate(&request(
        "router.acl.apply",
        &[("interface", "Gi0/0"), ("acl", "10"), ("direction", "in")],
    ))
    .unwrap();
    assert_eq!(apply[0].mode, ConfigMode::Interface("Gi0/0".into()));
    assert_eq!(apply[0].text, "ip access-group 10 in");
    assert!(
        script(
            "router.nat.static",
            &[("local", "10.0.0.10"), ("global", "192.0.2.10")]
        )
        .contains("ip nat inside source static 10.0.0.10 192.0.2.10")
    );
    assert!(
        script(
            "router.nat.interface",
            &[("interface", "Gi0/0"), ("role", "inside")]
        )
        .contains("ip nat inside")
    );
    assert!(
        router::generate(&request(
            "router.nat.static",
            &[("local", "10.0.0.10"), ("global", "10.0.0.10")]
        ))
        .is_err()
    );
}
#[test]
fn dhcp_checks_subnet_gateway_and_exclusion_order() {
    let pool = script(
        "router.dhcp.pool",
        &[
            ("pool", "LAN"),
            ("network", "10.0.0.0/24"),
            ("default_router", "10.0.0.1"),
            ("dns_server", "192.0.2.53"),
        ],
    );
    assert!(pool.contains("ip dhcp pool LAN\nnetwork 10.0.0.0 255.255.255.0\ndefault-router 10.0.0.1\ndns-server 192.0.2.53"));
    for gateway in ["192.0.2.1", "10.0.0.0", "10.0.0.255"] {
        assert!(
            router::generate(&request(
                "router.dhcp.pool",
                &[
                    ("pool", "LAN"),
                    ("network", "10.0.0.0/24"),
                    ("default_router", gateway)
                ]
            ))
            .is_err()
        );
    }
    assert!(
        script(
            "router.dhcp.exclude",
            &[("start", "10.0.0.1"), ("end", "10.0.0.20")]
        )
        .contains("ip dhcp excluded-address 10.0.0.1 10.0.0.20")
    );
    assert!(
        router::generate(&request(
            "router.dhcp.exclude",
            &[("start", "10.0.0.20"), ("end", "10.0.0.1")]
        ))
        .is_err()
    );
}
#[test]
fn direct_router_entry_rejects_unknown_fields_and_cli_injection() {
    for description in ["LAN\nreload", "LAN;reload", "LAN\rno shutdown"] {
        assert!(
            router::generate(&request(
                "router.interface.description",
                &[("interface", "Gi0/0"), ("description", description)]
            ))
            .is_err()
        );
    }
    assert!(
        router::generate(&request(
            "router.interface.description",
            &[
                ("interface", "Gi0/0"),
                ("description", "LAN"),
                ("raw_cli", "reload")
            ]
        ))
        .is_err()
    );
    let plan = cisco::generate(&request(
        "router.ospf.network",
        &[
            ("process_id", "10"),
            ("network", "10.0.0.0/24"),
            ("area", "0"),
        ],
    ));
    assert!(plan.executable());
    assert_ne!(plan.support, SupportState::VerifiedPacketTracer);
    assert!(
        plan.warnings
            .iter()
            .any(|s| s.contains("Not execution-tested"))
    );
}
