use pka_lab_inspector::cisco::{
    self, Classification, CommandPlan, CommandRequest, ConfigMode, DeviceProfile, ParameterKind,
    SupportState,
};

fn request(family: &str, values: &[(&str, &str)]) -> CommandRequest {
    CommandRequest {
        family: family.into(),
        device: "SW1".into(),
        profile: DeviceProfile::Catalyst2960,
        parameters: values
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}
fn example(family: &str) -> CommandRequest {
    let spec = cisco::switch::specs()
        .into_iter()
        .find(|s| s.id == family)
        .unwrap();
    let mut req = request(family, &[]);
    req.parameters = spec
        .parameters
        .iter()
        .filter(|p| p.required)
        .map(|p| (p.name.into(), p.example.into()))
        .collect();
    if family == "switch.trunk_native" || family == "switch.trunk_allowed" {
        req.parameters
            .insert("interface_mode".into(), "trunk".into());
    }
    if family == "switch.trunk_allowed" {
        req.parameters.insert("vlans".into(), "10,20-22".into());
    }
    if family == "switch.etherchannel" {
        req.parameters.insert("access_vlan".into(), "10".into());
        req.parameters
            .insert("group_access_vlan".into(), "10".into());
    }
    if family == "switch.routed_port" || family == "switch.ip_routing" {
        req.profile = DeviceProfile::Catalyst3560;
    }
    req
}
fn ready(req: &CommandRequest) -> CommandPlan {
    let plan = cisco::generate(req);
    assert!(
        plan.executable(),
        "{}: {:?} {:?} {:?}",
        req.family,
        plan.classification,
        plan.missing_parameters,
        plan.warnings
    );
    assert_eq!(plan.classification, Classification::Ready);
    assert!(!plan.verification.is_empty());
    assert!(plan.warnings.iter().any(|w| w.contains("Packet Tracer")));
    plan
}
fn reject(req: &CommandRequest) {
    let plan = cisco::generate(req);
    assert!(
        !plan.executable(),
        "Unexpected ready command: {} {:?}",
        req.family,
        req.parameters
    );
    assert!(plan.script().is_empty());
    assert!(plan.commands.is_empty());
}
fn set(req: &CommandRequest, key: &str, value: &str) -> CommandRequest {
    let mut new = req.clone();
    new.parameters.insert(key.into(), value.into());
    new
}

#[test]
fn every_spec_has_independent_bilingual_metadata_and_a_working_typed_example() {
    let specs = cisco::switch::specs();
    assert_eq!(specs.len(), 20);
    let mut ids = std::collections::BTreeSet::new();
    for spec in specs {
        assert!(ids.insert(spec.id));
        assert!(spec.id.starts_with("switch."));
        assert!(
            spec.source
                .starts_with("https://www.cisco.com/c/en/us/td/docs/")
        );
        assert_ne!(spec.name_en, spec.name_vi);
        assert!(!spec.prerequisites.is_empty());
        assert!(spec.semantic_validation);
        assert!(!spec.verification.is_empty());
        for p in &spec.parameters {
            assert_ne!(p.description_en, p.description_vi);
            assert!(
                !matches!(p.kind, ParameterKind::Text),
                "Free CLI text in {}",
                spec.id
            );
        }
        let plan = ready(&example(spec.id));
        assert_ne!(plan.support, SupportState::VerifiedPacketTracer);
    }
}
#[test]
fn vlan_is_rendered_once_in_vlan_mode() {
    let plan = ready(&example("switch.vlan"));
    assert_eq!(plan.commands[0].mode, ConfigMode::Vlan(10));
    assert_eq!(
        plan.script(),
        "enable\nconfigure terminal\nvlan 10\nname SALES\nend\n"
    );
    for value in ["0", "1", "1002", "1005", "1006", "4095", "-1"] {
        reject(&set(&example("switch.vlan"), "vlan", value));
    }
    for name in [
        "two words",
        "A/B",
        "x\nend",
        "SALES;reload",
        "012345678901234567890123456789012",
    ] {
        reject(&set(&example("switch.vlan"), "name", name));
    }
    reject(&set(&example("switch.vlan"), "vtp_mode", "client"));
    reject(&set(&example("switch.vlan"), "vtp_version", "3"));
}
#[test]
fn access_and_voice_require_explicit_access_context() {
    for family in ["switch.access_vlan", "switch.voice_vlan"] {
        let req = example(family);
        let plan = ready(&req);
        assert!(plan.commands[0].text.ends_with("vlan 10"));
        reject(&set(&req, "interface_mode", "trunk"));
        for mode in ["routed", "dynamic_auto", "unknown", ""] {
            reject(&set(&req, "interface_mode", mode));
        }
    }
    let req = example("switch.voice_vlan");
    for (key, value) in [
        ("vlan_active", "no"),
        ("cdp_enabled", "no"),
        ("endpoint", "switch"),
        ("port_security", "enabled"),
    ] {
        reject(&set(&req, key, value));
    }
}
#[test]
fn mode_conversion_uses_device_encapsulation_and_never_guesses_l3() {
    let req = set(&example("switch.port_mode"), "mode", "trunk");
    assert_eq!(ready(&req).commands.len(), 1);
    let mut multi = req.clone();
    multi.profile = DeviceProfile::Catalyst3560;
    let commands = ready(&multi).commands;
    assert_eq!(commands[0].text, "switchport trunk encapsulation dot1q");
    assert_eq!(commands[1].text, "switchport mode trunk");
    for profile in [DeviceProfile::Unknown, DeviceProfile::GenericSwitch] {
        let mut unknown = req.clone();
        unknown.profile = profile;
        reject(&unknown);
    }
    reject(&set(&req, "current_mode", "routed"));
    reject(&set(&req, "current_group", "1"));
}
#[test]
fn trunk_operations_and_native_peer_consistency() {
    let req = example("switch.trunk_allowed");
    for operation in ["replace", "add", "remove", "except"] {
        let plan = ready(&set(&req, "operation", operation));
        assert!(plan.commands[0].text.ends_with("10,20-22"));
    }
    for operation in ["all", "none"] {
        let mut new = set(&req, "operation", operation);
        new.parameters.remove("vlans");
        assert!(ready(&new).commands[0].text.ends_with(operation));
        reject(&set(&new, "vlans", "10"));
    }
    for list in [
        "",
        "0",
        "4095",
        "20-10",
        "10,,11",
        "1-2-3",
        "1,1",
        "1-3,2",
        "10;end",
        "10, 20",
        "all",
        "1\nreload",
    ] {
        reject(&set(&req, "vlans", list));
    }
    let native = example("switch.trunk_native");
    ready(&native);
    reject(&set(&native, "peer_native_vlan", "20"));
    reject(&set(&native, "interface_mode", "access"));
}
#[test]
fn stp_enums_steps_and_boundaries() {
    let req = example("switch.stp_priority");
    for n in ["0", "4096", "61440"] {
        ready(&set(&req, "priority", n));
    }
    for n in ["1", "4095", "61441", "65536"] {
        reject(&set(&req, "priority", n));
    }
    reject(&set(&req, "stp_mode", "mst"));
    let port = example("switch.stp_port_priority");
    for n in ["0", "16", "240"] {
        ready(&set(&port, "priority", n));
    }
    for n in ["1", "17", "241"] {
        reject(&set(&port, "priority", n));
    }
    let cost = example("switch.stp_cost");
    ready(&set(&cost, "cost", "200000000"));
    for n in ["0", "200000001", "4294967296"] {
        reject(&set(&cost, "cost", n));
    }
    for mode in ["pvst", "rapid-pvst", "mst"] {
        ready(&set(&example("switch.stp_mode"), "mode", mode));
    }
    reject(&set(&example("switch.stp_mode"), "mode", "rstp"));
    assert!(
        ready(&example("switch.stp_root"))
            .script()
            .contains("spanning-tree vlan 10,20-22 root primary")
    );
}
#[test]
fn stp_guards_reject_missing_or_conflicting_context() {
    reject(&set(&example("switch.stp_portfast"), "endpoint", "switch"));
    reject(&set(
        &example("switch.stp_portfast"),
        "interface_mode",
        "trunk",
    ));
    reject(&set(
        &example("switch.stp_bpduguard"),
        "portfast",
        "disabled",
    ));
    reject(&set(
        &example("switch.stp_rootguard"),
        "loop_guard",
        "enabled",
    ));
}
#[test]
fn port_security_sticky_static_and_aging_order() {
    let req = example("switch.port_security");
    assert!(
        ready(&req)
            .script()
            .contains("switchport port-security mac-address sticky")
    );
    let mut static_req = set(&req, "static_mac", "0011.2233.4455");
    static_req
        .parameters
        .insert("secure_count".into(), "1".into());
    assert!(
        ready(&static_req)
            .script()
            .contains("switchport port-security mac-address 0011.2233.4455")
    );
    for mac in [
        "0000.0000.0000",
        "ffff.ffff.ffff",
        "0111.2233.4455",
        "00:11:22:33:44:55",
        "0011.2233.4455 extra",
    ] {
        reject(&set(&static_req, "static_mac", mac));
    }
    let mut aged = set(&static_req, "sticky", "disabled");
    for (k, v) in [
        ("aging_minutes", "1440"),
        ("aging_type", "inactivity"),
        ("aging_static", "enabled"),
    ] {
        aged.parameters.insert(k.into(), v.into());
    }
    let script = ready(&aged).script();
    assert!(script.contains("switchport port-security aging time 1440"));
    assert!(script.contains("switchport port-security aging type inactivity"));
    assert!(script.contains("switchport port-security aging static"));
    reject(&set(&aged, "sticky", "enabled"));
    reject(&set(&aged, "aging_minutes", "1441"));
    aged.parameters.remove("aging_type");
    reject(&aged);
    for (key, value) in [
        ("maximum", "0"),
        ("maximum", "33"),
        ("secure_count", "3"),
        ("available_capacity", "1"),
        ("interface_mode", "trunk"),
        ("current_group", "1"),
        ("voice_vlan", "enabled"),
        ("span_destination", "yes"),
        ("violation", "unknown"),
    ] {
        reject(&set(&req, key, value));
    }
}
#[test]
fn etherchannel_protocol_matrix_requires_a_forming_peer() {
    let req = example("switch.etherchannel");
    for local in ["active", "passive", "auto", "desirable", "on"] {
        for peer in ["active", "passive", "auto", "desirable", "on"] {
            let mut new = set(&set(&req, "mode", local), "group_mode", local);
            new.parameters.insert("peer_mode".into(), peer.into());
            let compatible = matches!(
                (local, peer),
                ("active", "active" | "passive")
                    | ("passive", "active")
                    | ("desirable", "auto" | "desirable")
                    | ("auto", "desirable")
                    | ("on", "on")
            );
            if compatible {
                assert!(
                    ready(&new)
                        .script()
                        .contains(&format!("channel-group 1 mode {local}"))
                );
            } else {
                reject(&new);
            }
        }
    }
}
#[test]
fn etherchannel_consistency_and_conditional_metadata() {
    let req = example("switch.etherchannel");
    reject(&set(
        &set(&req, "interface", "Fa0/1"),
        "group_port_type",
        "fastethernet",
    ));
    for (key, value) in [
        ("group_port_type", "fastethernet"),
        ("resulting_member_count", "9"),
        ("span_destination", "yes"),
        ("member_enabled", "no"),
        ("group_stp_consistent", "no"),
    ] {
        reject(&set(&req, key, value));
    }
    for (key, value) in [
        ("group_access_vlan", "20"),
        ("group_speed", "100"),
        ("group_mode", "passive"),
        ("group_interface_mode", "trunk"),
        ("current_group", "2"),
        ("group", "7"),
        ("duplex", "half"),
    ] {
        reject(&set(&req, key, value));
    }
    let mut trunk = set(
        &set(&req, "interface_mode", "trunk"),
        "group_interface_mode",
        "trunk",
    );
    trunk.parameters.remove("access_vlan");
    trunk.parameters.remove("group_access_vlan");
    for (k, v) in [
        ("native_vlan", "1"),
        ("group_native_vlan", "1"),
        ("allowed_vlans", "1,10-12"),
        ("group_allowed_vlans", "1,10,11,12"),
    ] {
        trunk.parameters.insert(k.into(), v.into());
    }
    ready(&trunk);
    reject(&set(&trunk, "group_native_vlan", "2"));
    reject(&set(&trunk, "group_allowed_vlans", "1,10-11"));
    trunk.parameters.remove("group_allowed_vlans");
    reject(&trunk);
    let mut missing = req;
    missing.parameters.remove("access_vlan");
    reject(&missing);
}
#[test]
fn every_required_dependency_is_enforced_and_inputs_are_closed() {
    for spec in cisco::switch::specs() {
        let req = example(spec.id);
        for field in spec.parameters.iter().filter(|p| p.required) {
            let mut missing = req.clone();
            missing.parameters.remove(field.name);
            reject(&missing);
        }
        reject(&set(&req, "unrecognized_cli", "reload"));
        for field in &spec.parameters {
            reject(&set(&req, field.name, "x\nend\nreload"));
        }
    }
}
#[test]
fn l2_commands_reject_svis_routed_and_interface_range_injection() {
    for family in [
        "switch.access_vlan",
        "switch.port_security",
        "switch.etherchannel",
        "switch.stp_cost",
    ] {
        let req = example(family);
        for interface in [
            "Vlan10",
            "Loopback0",
            "Serial0/0/0",
            "Port-channel1",
            "Gi0/0.10",
            "Gi0/1 - 2",
            "Gi0/1\nend",
            "Gi0/0",
        ] {
            reject(&set(&req, "interface", interface));
        }
        for interface in ["Fa0/1", "Gi0/1", "GigabitEthernet1/0/1"] {
            let mut new = set(&req, "interface", interface);
            if family == "switch.etherchannel" && interface.starts_with("Fa") {
                new.parameters
                    .insert("group_port_type".into(), "fastethernet".into());
                new.parameters.insert("speed".into(), "100".into());
                new.parameters.insert("group_speed".into(), "100".into());
            }
            ready(&new);
        }
    }
}
#[test]
fn router_device_profiles_never_emit_switch_configuration() {
    for spec in cisco::switch::specs() {
        for profile in [
            DeviceProfile::Router1941,
            DeviceProfile::Router2911,
            DeviceProfile::Isr4321,
            DeviceProfile::GenericRouter,
        ] {
            let mut req = example(spec.id);
            req.profile = profile;
            reject(&req);
            assert_eq!(
                cisco::generate(&req).classification,
                Classification::RecognizedUnsupported
            );
        }
    }
}
#[test]
fn svi_management_and_gateway_validate_subnet_hosts() {
    let req = example("switch.svi.ipv4");
    for profile in [DeviceProfile::Catalyst2960, DeviceProfile::Catalyst3560] {
        let mut new = req.clone();
        new.profile = profile;
        assert!(
            ready(&new)
                .script()
                .contains("interface Vlan10\nip address 192.168.10.2 255.255.255.0\nno shutdown")
        );
    }
    reject(&set(&req, "interface", "Vlan20"));
    reject(&set(&req, "forwarding_port", "no"));
    for ip in [
        "0.0.0.0",
        "127.0.0.1",
        "224.0.0.1",
        "192.168.10.0",
        "192.168.10.255",
        "999.1.1.1",
    ] {
        reject(&set(&req, "address", ip));
    }
    for mask in [
        "255.0.255.0",
        "0.0.0.0",
        "255.255.255.254",
        "255.255.255.255",
    ] {
        reject(&set(&req, "mask", mask));
    }
    let gateway = example("switch.default_gateway");
    ready(&gateway);
    for ip in [
        "192.168.11.1",
        "192.168.10.2",
        "192.168.10.0",
        "192.168.10.255",
    ] {
        reject(&set(&gateway, "gateway", ip));
    }
    reject(&set(&gateway, "ip_routing", "enabled"));
}
#[test]
fn multilayer_operations_require_3560_and_confirmed_image() {
    for family in ["switch.routed_port", "switch.ip_routing"] {
        let req = example(family);
        ready(&req);
        for profile in [
            DeviceProfile::Catalyst2960,
            DeviceProfile::GenericSwitch,
            DeviceProfile::Unknown,
        ] {
            let mut new = req.clone();
            new.profile = profile;
            reject(&new);
        }
        reject(&set(&req, "image", "lan_base"));
    }
    let req = example("switch.routed_port");
    let script = ready(&req).script();
    assert!(script.contains("no switchport\nip address 192.168.10.2 255.255.255.0\nno shutdown"));
    assert!(
        !ready(&set(&req, "current_mode", "routed"))
            .script()
            .contains("no switchport")
    );
    reject(&set(&req, "conversion_approved", "no"));
    reject(&set(&req, "current_group", "1"));
}
