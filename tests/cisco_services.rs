use pka_lab_inspector::{
    cisco::{self, CommandRequest, DeviceProfile},
    commands,
    model::{Checkpoint, Status},
};
fn request(id: &str) -> CommandRequest {
    let spec = cisco::registry().into_iter().find(|s| s.id == id).unwrap();
    cisco::example_request(&spec, DeviceProfile::Router2911)
}
#[test]
fn new_service_examples_have_documented_context_and_no_execution_claim() {
    for id in [
        "router.dhcp.relay",
        "router.serial.clock",
        "router.serial.ppp",
        "basic.cdp_global",
        "basic.cdp_interface",
        "router.ospf.router_id",
        "router.ospf.passive",
        "router.ospf.reference_bandwidth",
    ] {
        let req = request(id);
        let plan = cisco::generate(&req);
        assert!(plan.executable(), "{id}: {:?}", plan.warnings);
        assert_ne!(plan.support, cisco::SupportState::VerifiedPacketTracer);
        let script = plan.script();
        assert!(!script.contains("no shutdown"));
        assert!(!script.contains("password"));
        assert!(!script.contains("reload"));
        for field in cisco::registry()
            .into_iter()
            .find(|s| s.id == id)
            .unwrap()
            .parameters
        {
            let mut missing = req.clone();
            missing.parameters.remove(field.name);
            assert!(
                !cisco::generate(&missing).executable(),
                "{id}: {}",
                field.name
            );
        }
    }
}

#[test]
fn ospf_controls_require_explicit_process_and_use_router_configuration_mode() {
    for (family, command) in [
        ("router.ospf.router_id", "router-id 192.0.2.9"),
        (
            "router.ospf.passive",
            "passive-interface GigabitEthernet0/0",
        ),
        (
            "router.ospf.reference_bandwidth",
            "auto-cost reference-bandwidth 1000",
        ),
    ] {
        let req = request(family);
        let plan = cisco::generate(&req);
        assert!(plan.executable());
        assert!(
            plan.script()
                .contains(&format!("router ospf 20\n{command}\n"))
        );
        assert!(!plan.script().contains("router ospf 1\n"));
        assert!(!plan.script().contains("clear "));
        assert!(!plan.script().contains("network "));
        for process in ["", "0", "65535", "65536", "1\nreload"] {
            let mut invalid = req.clone();
            invalid
                .parameters
                .insert("process_id".into(), process.into());
            assert!(
                !cisco::generate(&invalid).executable(),
                "{family}: {process}"
            );
        }
        for profile in [
            DeviceProfile::Catalyst2960,
            DeviceProfile::Catalyst3560,
            DeviceProfile::GenericSwitch,
        ] {
            let mut invalid = req.clone();
            invalid.profile = profile;
            assert!(!cisco::generate(&invalid).executable());
        }
    }
}

#[test]
fn ospf_router_id_is_an_identifier_and_bandwidth_units_are_explicit() {
    let mut req = request("router.ospf.router_id");
    // An identifier need not be an address assigned to an interface.
    req.parameters
        .insert("router_id".into(), "127.0.0.1".into());
    assert!(cisco::generate(&req).executable());
    for id in ["0.0.0.0", "192.0.2.1/24", "::1", "1.2.3.256", "auto"] {
        req.parameters.insert("router_id".into(), id.into());
        assert!(!cisco::generate(&req).executable(), "{id}");
    }
    req = request("router.ospf.reference_bandwidth");
    for mbps in ["1", "4294967"] {
        req.parameters.insert("mbps".into(), mbps.into());
        assert!(cisco::generate(&req).executable());
    }
    for mbps in ["0", "4294968", "1000Mbps", "1.5", "-1"] {
        req.parameters.insert("mbps".into(), mbps.into());
        assert!(!cisco::generate(&req).executable());
    }
    req = request("router.ospf.passive");
    req.parameters.insert("state".into(), "active".into());
    assert!(
        cisco::generate(&req)
            .script()
            .contains("\nno passive-interface GigabitEthernet0/0\n")
    );
    for interface in ["default", "all", "range Gi0/0-3", "Gi0/0\nreload"] {
        req.parameters.insert("interface".into(), interface.into());
        assert!(!cisco::generate(&req).executable());
    }
}
#[test]
fn clock_rate_requires_dce_module_supported_rate_and_physical_serial() {
    let req = request("router.serial.clock");
    assert!(
        cisco::generate(&req)
            .script()
            .contains("clock rate 64000\n")
    );
    for (key, value) in [
        ("cable_role", "dte"),
        ("module_present", "no"),
        ("bps", "1"),
        ("bps", "999999"),
        ("bps", "64000\nreload"),
        ("interface", "GigabitEthernet0/0"),
        ("interface", "Serial0/0/0.42"),
    ] {
        let mut invalid = req.clone();
        invalid.parameters.insert(key.into(), value.into());
        assert!(!cisco::generate(&invalid).executable(), "{key}={value}");
    }
    for profile in [
        DeviceProfile::Catalyst2960,
        DeviceProfile::Catalyst3560,
        DeviceProfile::GenericSwitch,
        DeviceProfile::Isr4321,
    ] {
        let mut invalid = req.clone();
        invalid.profile = profile;
        assert!(!cisco::generate(&invalid).executable(), "{profile:?}");
    }
}
#[test]
fn ppp_preserves_explicit_peer_requirement_without_inventing_authentication() {
    let mut req = request("router.serial.ppp");
    assert_eq!(
        cisco::generate(&req).script(),
        "enable\nconfigure terminal\ninterface Serial0/0/0\nencapsulation ppp\nend\n"
    );
    req.parameters
        .insert("peer_encapsulation".into(), "hdlc".into());
    assert!(!cisco::generate(&req).executable());
    req = request("router.serial.ppp");
    req.parameters
        .insert("interface".into(), "Loopback0".into());
    assert!(!cisco::generate(&req).executable());
}
#[test]
fn relay_rejects_reserved_servers_and_switch_profiles() {
    let req = request("router.dhcp.relay");
    assert!(
        cisco::generate(&req)
            .script()
            .contains("ip helper-address 192.0.2.10\n")
    );
    for server in [
        "0.0.0.0",
        "127.0.0.1",
        "224.0.0.1",
        "255.255.255.255",
        "192.0.2.10;reload",
        "::1",
    ] {
        let mut invalid = req.clone();
        invalid.parameters.insert("server".into(), server.into());
        assert!(!cisco::generate(&invalid).executable(), "{server}");
    }
    for profile in [
        DeviceProfile::Catalyst2960,
        DeviceProfile::Catalyst3560,
        DeviceProfile::GenericSwitch,
    ] {
        let mut invalid = req.clone();
        invalid.profile = profile;
        assert!(!cisco::generate(&invalid).executable());
    }
}
#[test]
fn cdp_states_modes_and_interface_dependencies_are_separate() {
    for profile in [
        DeviceProfile::Router2911,
        DeviceProfile::Catalyst2960,
        DeviceProfile::Catalyst3560,
    ] {
        let mut req = request("basic.cdp_global");
        req.profile = profile;
        assert!(cisco::generate(&req).script().contains("\ncdp run\n"));
        req.parameters.insert("enabled".into(), "false".into());
        assert!(cisco::generate(&req).script().contains("\nno cdp run\n"));
        req = request("basic.cdp_interface");
        req.profile = profile;
        let script = cisco::generate(&req).script();
        assert!(script.contains("\ncdp enable\n"));
        assert!(!script.contains("cdp run"));
        req.parameters.insert("global_enabled".into(), "no".into());
        assert!(!cisco::generate(&req).executable());
    }
    for interface in [
        "Loopback0",
        "Vlan42",
        "GigabitEthernet0/1.42",
        "Serial0/0/0",
        "Port-channel1",
    ] {
        let mut req = request("basic.cdp_interface");
        req.parameters.insert("interface".into(), interface.into());
        assert!(!cisco::generate(&req).executable(), "{interface}");
    }
}
#[test]
fn mappings_preserve_missing_parameters_and_allow_multiple_relay_destinations() {
    let mut c = Checkpoint {
        device: "Synthetic-Router".into(),
        device_type: "Router".into(),
        interface: "Serial0/0/0".into(),
        property: "Clock Rate".into(),
        expected: "64000".into(),
        ..Default::default()
    };
    assert!(!commands::generate(&c).supported);
    c.evidence.extend([
        "commands:parameter:module_present=yes".into(),
        "commands:parameter:cable_role=dce".into(),
    ]);
    assert!(commands::generate(&c).supported);
    assert_eq!(c.status, Status::Unverified);
    c.evidence.push("commands:parameter:cable_role=dte".into());
    assert!(!commands::generate(&c).supported);
    c.evidence.clear();
    c.interface = "GigabitEthernet0/0".into();
    c.property = "IP Helper Address".into();
    c.expected = "192.0.2.10".into();
    let mut second = c.clone();
    second.property = "DHCP Relay Address".into();
    second.expected = "192.0.2.20".into();
    let script = commands::device_script(&[c, second], None);
    assert!(script.contains("ip helper-address 192.0.2.10"), "{script}");
    assert!(script.contains("ip helper-address 192.0.2.20"));
}
