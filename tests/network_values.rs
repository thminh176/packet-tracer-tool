//! Regression and unit tests for network normalization, static routing, and OSPF.

use pka_lab_inspector::model::Checkpoint;
use pka_lab_inspector::network_values::*;
use std::net::Ipv4Addr;

// ---------------------------------------------------------------------------
// 1. Network Normalization Tests
// ---------------------------------------------------------------------------

#[test]
fn test_network_cidr_and_hyphen_equivalence() {
    // 100.0.0.0/8 and 100.0.0.0-8
    let cidr = NormalizedNetwork::parse("100.0.0.0/8").expect("parse 100.0.0.0/8");
    let hyphen = NormalizedNetwork::parse("100.0.0.0-8").expect("parse 100.0.0.0-8");
    assert_eq!(cidr.network, Ipv4Addr::new(100, 0, 0, 0));
    assert_eq!(cidr.prefix, 8);
    assert_eq!(cidr.subnet_mask, Ipv4Addr::new(255, 0, 0, 0));
    assert_eq!(cidr.wildcard_mask, Ipv4Addr::new(0, 255, 255, 255));
    assert_eq!(hyphen.network, cidr.network);
    assert_eq!(hyphen.prefix, cidr.prefix);
    assert_eq!(hyphen.subnet_mask, cidr.subnet_mask);
    assert_eq!(hyphen.wildcard_mask, cidr.wildcard_mask);
}

#[test]
fn test_network_standard_subnets() {
    // /24
    let n24 = NormalizedNetwork::parse("192.168.1.0/24").unwrap();
    assert_eq!(n24.network, Ipv4Addr::new(192, 168, 1, 0));
    assert_eq!(n24.subnet_mask, Ipv4Addr::new(255, 255, 255, 0));
    assert_eq!(n24.wildcard_mask, Ipv4Addr::new(0, 0, 0, 255));

    // /16
    let n16 = NormalizedNetwork::parse("172.16.0.0/16").unwrap();
    assert_eq!(n16.network, Ipv4Addr::new(172, 16, 0, 0));
    assert_eq!(n16.subnet_mask, Ipv4Addr::new(255, 255, 0, 0));
    assert_eq!(n16.wildcard_mask, Ipv4Addr::new(0, 0, 255, 255));

    // /30
    let n30 = NormalizedNetwork::parse("10.0.0.0/30").unwrap();
    assert_eq!(n30.network, Ipv4Addr::new(10, 0, 0, 0));
    assert_eq!(n30.subnet_mask, Ipv4Addr::new(255, 255, 255, 252));
    assert_eq!(n30.wildcard_mask, Ipv4Addr::new(0, 0, 0, 3));

    // /0 (Default)
    let n0 = NormalizedNetwork::parse("0.0.0.0/0").unwrap();
    assert_eq!(n0.network, Ipv4Addr::new(0, 0, 0, 0));
    assert_eq!(n0.prefix, 0);
    assert_eq!(n0.subnet_mask, Ipv4Addr::new(0, 0, 0, 0));
    assert_eq!(n0.wildcard_mask, Ipv4Addr::new(255, 255, 255, 255));

    // /32 (Host)
    let n32 = NormalizedNetwork::parse("192.168.1.1/32").unwrap();
    assert_eq!(n32.network, Ipv4Addr::new(192, 168, 1, 1));
    assert_eq!(n32.prefix, 32);
    assert_eq!(n32.subnet_mask, Ipv4Addr::new(255, 255, 255, 255));
    assert_eq!(n32.wildcard_mask, Ipv4Addr::new(0, 0, 0, 0));
}

#[test]
fn test_network_invalid_prefixes_and_malformed() {
    assert!(NormalizedNetwork::parse("10.0.0.0/33").is_none());
    assert!(NormalizedNetwork::parse("10.0.0.0-33").is_none());
    assert!(NormalizedNetwork::parse("10.0.0.0/255").is_none());
    assert!(NormalizedNetwork::parse("999.0.0.0/24").is_none());
    assert!(NormalizedNetwork::parse("not-an-ip/24").is_none());
    assert!(NormalizedNetwork::parse("").is_none());
    assert!(NormalizedNetwork::parse("   ").is_none());
    assert!(NormalizedNetwork::parse("null").is_none());
    assert!(NormalizedNetwork::parse("unset").is_none());
    assert!(NormalizedNetwork::parse("-1").is_none());
    assert!(NormalizedNetwork::parse("4294967295").is_none());
}

// ---------------------------------------------------------------------------
// 2. Static Routing Tests
// ---------------------------------------------------------------------------

#[test]
fn test_static_route_complete_valid_cli_and_hyphen() {
    // Complete hyphenated Packet Tracer route
    let r1 = ParsedStaticRoute::parse("172.16.128.0-17-172.18.192.1-0-1").unwrap();
    assert_eq!(r1.destination.cidr(), "172.16.128.0/17");
    assert_eq!(r1.destination.subnet_mask, Ipv4Addr::new(255, 255, 128, 0));
    assert_eq!(r1.next_hop, Some(Ipv4Addr::new(172, 18, 192, 1)));
    assert_eq!(r1.distance, Some(1));
    assert!(r1.is_complete);
    assert_eq!(
        r1.to_cisco_command().unwrap(),
        "ip route 172.16.128.0 255.255.128.0 172.18.192.1"
    );

    // Complete CLI route
    let r2 = ParsedStaticRoute::parse("192.168.1.0 255.255.255.0 10.0.0.1").unwrap();
    assert_eq!(r2.destination.cidr(), "192.168.1.0/24");
    assert_eq!(r2.next_hop, Some(Ipv4Addr::new(10, 0, 0, 1)));
    assert!(r2.is_complete);
    assert_eq!(
        r2.to_cisco_command().unwrap(),
        "ip route 192.168.1.0 255.255.255.0 10.0.0.1"
    );
}

#[test]
fn test_static_route_missing_next_hop() {
    let r = ParsedStaticRoute::parse("100.0.0.0-8").unwrap();
    assert_eq!(r.destination.cidr(), "100.0.0.0/8");
    assert!(!r.is_complete);
    assert_eq!(r.missing_fields, vec!["next-hop or exit-interface"]);
    assert!(r.to_cisco_command().is_none());
}

#[test]
fn test_static_route_incorrect_subnet_mask_host_bits() {
    // 192.0.2.1 has host bit set for /24 -> invalid static route destination
    assert!(ParsedStaticRoute::parse("192.0.2.1 255.255.255.0 198.51.100.1").is_err());
    assert!(ParsedStaticRoute::parse("192.0.2.1-24-198.51.100.1-0-1").is_err());
}

#[test]
fn test_static_route_default_route() {
    // Default route via exit interface
    let r1 = ParsedStaticRoute::parse("0.0.0.0-0-Serial0/1/1-0-1").unwrap();
    assert!(r1.is_default);
    assert_eq!(r1.exit_interface, Some("Serial0/1/1".into()));
    assert_eq!(
        r1.to_cisco_command().unwrap(),
        "ip route 0.0.0.0 0.0.0.0 Serial0/1/1"
    );

    // Default route via next-hop IP
    let r2 = ParsedStaticRoute::parse("0.0.0.0 0.0.0.0 172.18.192.2").unwrap();
    assert!(r2.is_default);
    assert_eq!(r2.next_hop, Some(Ipv4Addr::new(172, 18, 192, 2)));
    assert_eq!(
        r2.to_cisco_command().unwrap(),
        "ip route 0.0.0.0 0.0.0.0 172.18.192.2"
    );
}

#[test]
fn test_static_route_floating_administrative_distance() {
    let r = ParsedStaticRoute::parse("10.0.0.0 255.0.0.0 192.168.1.1 5").unwrap();
    assert_eq!(r.distance, Some(5));
    assert_eq!(
        r.to_cisco_command().unwrap(),
        "ip route 10.0.0.0 255.0.0.0 192.168.1.1 5"
    );
}

// ---------------------------------------------------------------------------
// 3. OSPF Tests
// ---------------------------------------------------------------------------

#[test]
fn test_ospf_wildcard_conversions() {
    // /8 -> 0.255.255.255
    assert_eq!(prefix_to_wildcard(8), Some(Ipv4Addr::new(0, 255, 255, 255)));
    // /16 -> 0.0.255.255
    assert_eq!(prefix_to_wildcard(16), Some(Ipv4Addr::new(0, 0, 255, 255)));
    // /24 -> 0.0.0.255
    assert_eq!(prefix_to_wildcard(24), Some(Ipv4Addr::new(0, 0, 0, 255)));
    // /30 -> 0.0.0.3
    assert_eq!(prefix_to_wildcard(30), Some(Ipv4Addr::new(0, 0, 0, 3)));
}

#[test]
fn test_ospf_network_parsing_and_missing_fields() {
    // 4-token complete
    let o1 = ParsedOspfNetwork::parse("1 10.0.0.0 0.0.0.3 0").unwrap();
    assert_eq!(o1.process_id, Some(1));
    assert_eq!(o1.network, Ipv4Addr::new(10, 0, 0, 0));
    assert_eq!(o1.wildcard_mask, Ipv4Addr::new(0, 0, 0, 3));
    assert_eq!(o1.area, Some("0".into()));
    assert!(o1.is_complete);
    assert_eq!(
        o1.to_cisco_command().unwrap(),
        "network 10.0.0.0 0.0.0.3 area 0"
    );

    // 3-token (missing process ID)
    let o2 = ParsedOspfNetwork::parse("10.0.0.0 0.0.0.3 0").unwrap();
    assert!(!o2.is_complete);
    assert_eq!(o2.missing_fields, vec!["OSPF process ID"]);

    // Complete with process ID
    let o2_c = o2.with_process_id(10);
    assert!(o2_c.is_complete);
    assert_eq!(o2_c.process_id, Some(10));

    // Hyphenated CIDR (missing process and area)
    let o3 = ParsedOspfNetwork::parse("100.0.0.0-8").unwrap();
    assert_eq!(o3.wildcard_mask, Ipv4Addr::new(0, 255, 255, 255));
    assert!(!o3.is_complete);
    assert_eq!(o3.missing_fields, vec!["OSPF process ID", "OSPF area ID"]);
}

#[test]
fn test_ospf_reject_sentinels() {
    assert!(ParsedOspfNetwork::parse("65535 10.0.0.0 0.0.0.3 0").is_err());
    assert!(ParsedOspfNetwork::parse("1 10.0.0.0 0.0.0.3 4294967295").is_err());
    assert!(ParsedOspfNetwork::parse("1 10.0.0.0 0.0.0.3 unknown").is_err());
}

// ---------------------------------------------------------------------------
// 4. Command Generation Integration Tests
// ---------------------------------------------------------------------------

#[test]
fn test_command_generation_for_normalized_checkpoints() {
    // Hyphenated static route
    let c1 = Checkpoint {
        id: "r1-route".into(),
        property: "Static Route".into(),
        expected: "172.16.128.0-17-172.18.192.1-0-1".into(),
        device: "R1".into(),
        ..Default::default()
    };
    let cmd1 = pka_lab_inspector::commands::generate(&c1);
    assert!(cmd1.supported);
    assert!(
        cmd1.text
            .contains("ip route 172.16.128.0 255.255.128.0 172.18.192.1")
    );

    // OSPF network with sibling evidence
    let c2 = Checkpoint {
        id: "r1-ospf".into(),
        property: "OSPF Network".into(),
        expected: "10.0.0.0 0.0.0.3 0".into(),
        device: "R1".into(),
        evidence: vec!["commands:ospf-process=1".into()],
        ..Default::default()
    };
    let cmd2 = pka_lab_inspector::commands::generate(&c2);
    assert!(cmd2.supported);
    assert!(cmd2.text.contains("router ospf 1"));
    assert!(cmd2.text.contains("network 10.0.0.0 0.0.0.3 area 0"));

    // Missing next-hop route disables command generation
    let c3 = Checkpoint {
        id: "r1-incomplete".into(),
        property: "Static Route".into(),
        expected: "100.0.0.0-8".into(),
        device: "R1".into(),
        ..Default::default()
    };
    let cmd3 = pka_lab_inspector::commands::generate(&c3);
    assert!(!cmd3.supported);
}
