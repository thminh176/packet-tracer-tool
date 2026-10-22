use pka_lab_inspector::cisco::{self, Capability, DeviceProfile, SupportState};
fn profile(s: &cisco::CommandSpec) -> DeviceProfile {
    if s.id.starts_with("switch.") {
        if s.capability == Capability::Multilayer {
            DeviceProfile::Catalyst3560
        } else {
            DeviceProfile::Catalyst2960
        }
    } else {
        DeviceProfile::Router2911
    }
}
#[test]
fn shipped_learning_examples_are_complete_and_scoped() {
    let specs = cisco::registry();
    let mut ids = std::collections::BTreeSet::new();
    assert!(!specs.is_empty());
    for s in &specs {
        assert!(ids.insert(s.id));
        assert!(s.source.starts_with("https://www.cisco.com/"));
        assert!(!s.software.is_empty());
        let req = cisco::example_request(s, profile(s));
        let plan = cisco::generate(&req);
        assert!(
            plan.executable(),
            "{}: {:?} {:?}",
            s.id,
            plan.missing_parameters,
            plan.warnings
        );
        assert!(!plan.script().contains("reload"));
        for field in s.parameters.iter().filter(|p| p.required) {
            let mut missing = req.clone();
            missing.parameters.remove(field.name);
            assert!(
                !cisco::generate(&missing).executable(),
                "{} missing {}",
                s.id,
                field.name
            );
        }
        let mut injected = req.clone();
        injected.device = "bad\nreload".into();
        assert!(!cisco::generate(&injected).executable());
    }
}
#[test]
fn documentation_never_claims_packet_tracer_execution() {
    for profile in DeviceProfile::ALL {
        for s in cisco::registry() {
            assert_ne!(
                profile.support(s.capability),
                SupportState::VerifiedPacketTracer
            );
        }
    }
}
#[test]
fn malformed_vlan_lists_and_unknown_fields_do_not_generate() {
    assert!(cisco::vlan_list("0,4095").is_none());
    assert!(cisco::vlan_list("10-2").is_none());
    let s = cisco::registry()
        .into_iter()
        .find(|s| s.id == "basic.hostname")
        .unwrap();
    let mut req = cisco::example_request(&s, DeviceProfile::Unknown);
    req.parameters.insert("execute".into(), "reload".into());
    assert!(!cisco::generate(&req).executable());
}
