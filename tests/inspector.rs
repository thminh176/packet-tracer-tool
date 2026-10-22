use pka_lab_inspector::{
    inspector::{compare, load, parse},
    model::{Analysis, Status},
};

fn leaf(id: &str, property: &str, expected: &str, points: &str) -> String {
    format!(
        r#"<NODE><NAME nodeValue="{expected}" checkType="2" headNode="false">{property}</NAME><ID>{id}</ID><POINTS>{points}</POINTS></NODE>"#
    )
}
fn branch(id: &str, children: &str) -> String {
    format!(r#"<NODE><NAME headNode="true">{id}</NAME><ID>{id}</ID>{children}</NODE>"#)
}
fn network(ip: &str) -> String {
    format!(
        "<PACKETTRACER5><NETWORK><DEVICES><DEVICE><ENGINE><NAME>R1</NAME><TYPE>Router</TYPE><PORT><NAME>GigabitEthernet0/0</NAME><IP>{ip}</IP><SUBNET>255.255.255.0</SUBNET></PORT></ENGINE></DEVICE></DEVICES></NETWORK></PACKETTRACER5>"
    )
}
fn activity(leaves: &str, snapshots: &str) -> String {
    format!(
        "<PACKETTRACER5_ACTIVITY>{snapshots}<COMPARISONS>{}</COMPARISONS></PACKETTRACER5_ACTIVITY>",
        branch(
            "Network",
            &branch(
                "R1",
                &branch("Ports", &branch("GigabitEthernet0/0", leaves))
            )
        )
    )
}
fn fixture() -> String {
    activity(
        &leaf("IP", "IP Address", "192.0.2.1", "2.5"),
        &network("192.0.2.1"),
    )
}

#[test]
fn synthetic_126_checkpoints_preserve_leaves_weights_and_stable_paths() {
    let leaves = (0..126)
        .map(|i| {
            leaf(
                &format!("custom-{i}"),
                "Unknown future check",
                "sentinel",
                "0.5",
            )
        })
        .collect::<String>();
    let xml = activity(&leaves, &network("192.0.2.1"));
    let analysis = parse(&xml).unwrap();
    assert_eq!(analysis.checkpoints.len(), 126);
    assert_eq!(analysis.summary().total_score, 63.0);
    assert_eq!(analysis.summary().unverified, 126);
    assert_eq!(analysis.summary().verified_score, 0.0);
    assert_eq!(analysis.xml, xml);
    assert!(analysis.checkpoints.iter().all(|c| c.device == "R1"
        && c.interface == "GigabitEthernet0/0"
        && c.expected == "sentinel"));
    assert_ne!(analysis.checkpoints[0].id, analysis.checkpoints[125].id);
}

#[test]
fn comparison_proves_pass_fail_and_weighted_score() {
    let leaves = format!(
        "{}{}{}",
        leaf("IP", "IP Address", "192.0.2.1", "2.5"),
        leaf("Mask", "Subnet Mask", "255.255.0.0", "3"),
        leaf("Future", "Custom policy", "x", "8")
    );
    let mut analysis = parse(&activity(&leaves, &network("192.0.2.1"))).unwrap();
    compare(&mut analysis, &parse(&network("192.0.2.1")).unwrap());
    assert_eq!(analysis.checkpoints[0].status, Status::Passed);
    assert_eq!(analysis.checkpoints[1].status, Status::Failed);
    assert_eq!(analysis.checkpoints[2].status, Status::Unverified);
    assert!(
        analysis.checkpoints[0]
            .evidence
            .iter()
            .any(|s| s.contains("192.0.2.1"))
    );
    let score = analysis.summary();
    assert_eq!(score.verified_score, 2.5);
    assert_eq!(score.total_score, 13.5);
    assert_eq!(score.coverage, 2.0 / 3.0);
}

#[test]
fn explicit_roles_are_order_independent_and_never_use_answer_as_current() {
    let leaf = leaf("IP", "IP Address", "192.0.2.1", "1");
    let answer = format!("<ANSWER_NETWORK>{}</ANSWER_NETWORK>", network("192.0.2.1"));
    let current = format!(
        "<CURRENT_NETWORK>{}</CURRENT_NETWORK>",
        network("192.0.2.2")
    );
    for snapshots in [format!("{answer}{current}"), format!("{current}{answer}")] {
        let analysis = parse(&activity(&leaf, &snapshots)).unwrap();
        assert_eq!(analysis.checkpoints[0].status, Status::Failed);
        assert_eq!(
            analysis.checkpoints[0].current.as_deref(),
            Some("192.0.2.2")
        );
    }
}

#[test]
fn unlabelled_snapshots_remain_unknown_even_when_values_match() {
    let snapshots = format!(
        "{}{}{}",
        network("192.0.2.1"),
        network("192.0.2.1"),
        network("192.0.2.1")
    );
    let xml = activity(&leaf("IP", "IP Address", "192.0.2.1", "1"), &snapshots);
    let mut analysis = parse(&xml).unwrap();
    compare(&mut analysis, &parse(&xml).unwrap());
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
    assert!(analysis.diagnostics.iter().any(|d| d.contains("ambiguous")));
}

#[test]
fn duplicate_and_conflicting_current_roles_remain_unknown() {
    let net = network("192.0.2.1");
    for snapshots in [
        format!("<CURRENT_NETWORK>{net}{net}</CURRENT_NETWORK>"),
        format!(
            "<CURRENT_NETWORK>{}</CURRENT_NETWORK>",
            net.replace("<PACKETTRACER5>", "<PACKETTRACER5 role=\"answer\">")
        ),
    ] {
        let mut analysis = parse(&fixture()).unwrap();
        compare(&mut analysis, &parse(&activity("", &snapshots)).unwrap());
        assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
    }
}

#[test]
fn sentinel_expected_and_current_never_pass_or_fail() {
    for sentinel in [
        "",
        "--",
        "n/a",
        "unknown",
        "-1",
        "65535",
        "4294967295",
        "0.0.0.0",
        "::",
        "${value}",
    ] {
        let mut analysis = parse(&activity(
            &leaf("IP", "IP Address", sentinel, "1"),
            &network("192.0.2.1"),
        ))
        .unwrap();
        compare(&mut analysis, &parse(&network("192.0.2.1")).unwrap());
        assert_eq!(
            analysis.checkpoints[0].status,
            Status::Unverified,
            "{sentinel}"
        );
        let mut analysis = parse(&fixture()).unwrap();
        compare(&mut analysis, &parse(&network(sentinel)).unwrap());
        assert_eq!(
            analysis.checkpoints[0].status,
            Status::Unverified,
            "{sentinel}"
        );
    }
}

#[test]
fn unknown_custom_variable_and_duplicate_checks_are_retained_unverified() {
    for leaves in [
        leaf("IP", "IP Address", "192.0.2.1", "1").replace("checkType=\"2\"", "checkType=\"3\""),
        leaf("IP", "IP Address", "192.0.2.1", "1").replace(
            "checkType=\"2\"",
            "checkType=\"2\" variableEnabled=\"true\" variableName=\"dynamic\"",
        ),
        leaf("Future", "Unknown future property", "x", "1"),
        leaf("IP", "IP Address", "192.0.2.1", "1").repeat(2),
    ] {
        let mut analysis = parse(&activity(&leaves, &network("192.0.2.1"))).unwrap();
        compare(&mut analysis, &parse(&network("192.0.2.1")).unwrap());
        assert!(
            analysis
                .checkpoints
                .iter()
                .all(|c| c.status == Status::Unverified)
        );
    }
}

#[test]
fn duplicated_device_or_port_identity_never_proves_a_match() {
    let net = network("192.0.2.1");
    let duplicate_device = net.replace(
        "</DEVICES>",
        "<DEVICE><ENGINE><NAME>R1</NAME></ENGINE></DEVICE></DEVICES>",
    );
    let duplicate_port = net.replace(
        "</ENGINE>",
        "<PORT><NAME>GigabitEthernet0/0</NAME><IP>192.0.2.1</IP></PORT></ENGINE>",
    );
    for network in [duplicate_device, duplicate_port] {
        let mut analysis = parse(&fixture()).unwrap();
        compare(&mut analysis, &parse(&network).unwrap());
        assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
    }
}

#[test]
fn invalid_weights_do_not_poison_score_and_raw_xml_is_preserved() {
    for weight in ["NaN", "inf", "-2", "bogus"] {
        let xml = activity(&leaf("Future", "Unknown", "x", weight), "");
        let analysis = parse(&xml).unwrap();
        assert_eq!(analysis.checkpoints.len(), 1);
        assert_eq!(analysis.summary().total_score, 0.0);
        assert_eq!(analysis.xml, xml);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.contains("Invalid weight"))
        );
    }
}

#[test]
fn malformed_xml_entities_dtd_and_depth_are_rejected() {
    for xml in [
        "",
        "<PACKETTRACER5>",
        "<PACKETTRACER5><A></B></PACKETTRACER5>",
        "<PACKETTRACER5/><PACKETTRACER5/>",
        "<!DOCTYPE x [<!ENTITY a 'x'>]><PACKETTRACER5/>",
        "<PACKETTRACER5>&external;</PACKETTRACER5>",
        "<PACKETTRACER5 a='1' a='2'/>",
        "junk<PACKETTRACER5/>",
        "<UNRELATED/>",
    ] {
        assert!(parse(xml).is_err(), "{xml}");
    }
    let deep = format!(
        "<PACKETTRACER5>{}{}</PACKETTRACER5>",
        "<NODE>".repeat(130),
        "</NODE>".repeat(130)
    );
    assert!(parse(&deep).is_err());
}

#[test]
fn entities_cdata_and_unknown_leaves_are_preserved() {
    let xml = "<PACKETTRACER5_ACTIVITY><COMPARISONS><NODE><NAME nodeValue='a &amp; b' headNode='false'><![CDATA[Unknown <property>]]></NAME><ID>x&#47;y</ID><POINTS>0.125</POINTS></NODE></COMPARISONS></PACKETTRACER5_ACTIVITY>";
    let analysis = parse(xml).unwrap();
    assert_eq!(analysis.checkpoints[0].expected, "a & b");
    assert_eq!(analysis.checkpoints[0].property, "Unknown <property>");
    assert_eq!(analysis.checkpoints[0].id, "x/y");
    assert_eq!(analysis.checkpoints[0].weight, 0.125);
}

#[test]
fn empty_container_is_not_counted_and_comparisons_take_precedence() {
    let xml = format!(
        "<PACKETTRACER5_ACTIVITY><COMPARISONS>{}{}</COMPARISONS><INITIALSETUP>{}</INITIALSETUP></PACKETTRACER5_ACTIVITY>",
        branch("Empty container", ""),
        leaf("selected", "Unknown", "x", "3"),
        leaf("unselected", "Unknown", "y", "8")
    );
    let analysis = parse(&xml).unwrap();
    assert_eq!(analysis.checkpoints.len(), 1);
    assert_eq!(analysis.checkpoints[0].id, "selected");
}

#[test]
fn invalid_new_progress_clears_previous_verified_result() {
    let mut analysis = parse(&fixture()).unwrap();
    compare(&mut analysis, &parse(&network("192.0.2.1")).unwrap());
    assert_eq!(analysis.checkpoints[0].status, Status::Passed);
    compare(
        &mut analysis,
        &Analysis {
            xml: "broken".into(),
            ..Default::default()
        },
    );
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
    assert_eq!(analysis.checkpoints[0].current, None);
    assert!(
        !analysis.checkpoints[0]
            .evidence
            .iter()
            .any(|e| e.starts_with("current network"))
    );
}

#[test]
fn duplicate_scalar_fields_and_invalid_masks_are_unverified() {
    let mut analysis = parse(&fixture()).unwrap();
    let duplicate_ip = network("192.0.2.1").replace("</IP>", "</IP><IP>192.0.2.1</IP>");
    compare(&mut analysis, &parse(&duplicate_ip).unwrap());
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
    let mut analysis = parse(&activity(
        &leaf("Mask", "Subnet Mask", "255.0.255.0", "1"),
        &network("192.0.2.1"),
    ))
    .unwrap();
    compare(
        &mut analysis,
        &parse(&network("192.0.2.1").replace("255.255.255.0", "255.0.255.0")).unwrap(),
    );
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
}

#[test]
fn sibling_command_context_is_explicit_unique_and_survives_compare() {
    let ip = leaf("IP", "IP Address", "192.0.2.1", "1");
    let mask = leaf("Mask", "Subnet Mask", "255.255.255.0", "1");
    let mut analysis = parse(&activity(&format!("{ip}{mask}"), &network("192.0.2.1"))).unwrap();
    assert!(
        analysis.checkpoints[0]
            .evidence
            .contains(&"commands:ipv4-mask=255.255.255.0".into())
    );
    compare(&mut analysis, &parse(&network("192.0.2.1")).unwrap());
    assert!(
        analysis.checkpoints[0]
            .evidence
            .contains(&"commands:ipv4-mask=255.255.255.0".into())
    );
    for siblings in [
        ip.clone(),
        format!("{ip}{mask}{mask}"),
        format!("{ip}{}", mask.replace("checkType=\"2\"", "checkType=\"3\"")),
    ] {
        let analysis = parse(&activity(&siblings, &network("192.0.2.1"))).unwrap();
        assert!(
            !analysis.checkpoints[0]
                .evidence
                .iter()
                .any(|e| e.starts_with("commands:"))
        );
    }
}

#[test]
fn disabled_checks_and_initial_inventory_are_not_graded() {
    let disabled = leaf("IP", "IP Address", "192.0.2.1", "1")
        .replace("checkType=\"2\"", "checkType=\"2\" selected=\"false\"");
    for xml in [
        activity(&disabled, &network("192.0.2.1")),
        fixture().replace("COMPARISONS", "INITIALSETUP"),
    ] {
        let mut analysis = parse(&xml).unwrap();
        assert!(analysis.checkpoints.is_empty());
        compare(&mut analysis, &parse(&network("192.0.2.1")).unwrap());
        assert_eq!(analysis.summary().total_score, 0.0);
        assert!(analysis.diagnostics.iter().any(|d| d.contains("inventory")));
    }
}

#[test]
fn selection_separates_unchecked_inventory_from_unknown_selected_checks() {
    let selected = leaf("Future", "Future unknown property", "unknown", "2.5");
    let zero = leaf("Zero", "Unknown zero weight", "x", "0");
    let unchecked = leaf("Unchecked", "IP Address", "192.0.2.1", "500")
        .replace("checkType=\"2\"", "checkType=\"0\"");
    let partial =
        leaf("Partial", "Container", "", "600").replace("checkType=\"2\"", "checkType=\"1\"");
    let analysis = parse(&activity(
        &format!("{selected}{zero}{unchecked}{partial}"),
        &network("192.0.2.1"),
    ))
    .unwrap();
    assert_eq!(analysis.checkpoints.len(), 2);
    assert_eq!(analysis.summary().total_score, 2.5);
    assert!(
        analysis
            .checkpoints
            .iter()
            .all(|c| c.status == Status::Unverified)
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.contains("Excluded 2"))
    );
}

#[test]
fn terminal_unknown_collection_with_explicit_weight_is_preserved() {
    let collection = leaf("Future list", "Future List", "", "3.5")
        .replace("headNode=\"false\"", "headNode=\"true\"");
    let analysis = parse(&activity(&collection, "")).unwrap();
    assert_eq!(analysis.checkpoints.len(), 1);
    assert_eq!(analysis.checkpoints[0].weight, 3.5);
}

#[test]
fn oversized_input_is_rejected_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("oversized.pka");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert!(load(&path).unwrap_err().to_string().contains("64 MiB"));
}

fn ipv6_network(address: &str, prefix: &str) -> String {
    network("192.0.2.1").replace("</PORT>", &format!("<IPV6_ADDRESSES><IPV6_ADDRESS><ADDRESS>{address}</ADDRESS><PREFIX>{prefix}</PREFIX><TYPE>1</TYPE></IPV6_ADDRESS></IPV6_ADDRESSES></PORT>"))
}

#[test]
fn ipv6_canonicalization_prefix_mismatch_and_invalid_sentinels() {
    let leaf = leaf(
        "IPv6",
        "IPv6 Address",
        "2001:0DB8:0000:0000:0000:0000:0000:0001/64",
        "1",
    );
    let mut analysis = parse(&activity(&leaf, &network("192.0.2.1"))).unwrap();
    assert!(
        analysis.checkpoints[0]
            .evidence
            .contains(&"commands:ipv6-prefix=64".into())
    );
    compare(
        &mut analysis,
        &parse(&ipv6_network("2001:db8::1", "64")).unwrap(),
    );
    assert_eq!(analysis.checkpoints[0].status, Status::Passed);
    compare(
        &mut analysis,
        &parse(&ipv6_network("2001:db8::1", "80")).unwrap(),
    );
    assert_eq!(analysis.checkpoints[0].status, Status::Failed);
    for (address, prefix) in [
        ("::", "64"),
        ("not-ipv6", "64"),
        ("2001:db8::1", "129"),
        ("2001:db8::1", "-1"),
    ] {
        compare(
            &mut analysis,
            &parse(&ipv6_network(address, prefix)).unwrap(),
        );
        assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
    }
}

#[test]
fn ipv4_prefix_and_dotted_mask_are_semantically_equivalent() {
    for expected in ["24", "/24", "255.255.255.0"] {
        let mut analysis = parse(&activity(
            &leaf("Mask", "Subnet Mask", expected, "1"),
            &network("192.0.2.1"),
        ))
        .unwrap();
        compare(&mut analysis, &parse(&network("192.0.2.1")).unwrap());
        assert_eq!(analysis.checkpoints[0].status, Status::Passed);
    }
}

fn ios_network(lines: &[&str]) -> String {
    format!(
        "<PACKETTRACER5><NETWORK><DEVICES><DEVICE><ENGINE><NAME>R1</NAME><TYPE>Router</TYPE><RUNNINGCONFIG>{}</RUNNINGCONFIG></ENGINE></DEVICE></DEVICES></NETWORK></PACKETTRACER5>",
        lines
            .iter()
            .map(|line| format!("<LINE>{line}</LINE>"))
            .collect::<String>()
    )
}

#[test]
fn ios_interface_fields_use_named_sections_and_explicit_shutdown_state() {
    let leaves = format!(
        "{}{}{}",
        leaf("IP", "IP Address", "192.0.2.1", "1"),
        leaf("IPv6", "IPv6 Address", "2001:db8::1/64", "1"),
        leaf("Power", "Port Status", "true", "1")
    );
    let mut analysis = parse(&activity(&leaves, &network("192.0.2.1"))).unwrap();
    let progress = ios_network(&[
        "interface GigabitEthernet0/1",
        " ip address 203.0.113.1 255.255.255.0",
        " shutdown",
        "!",
        "interface GigabitEthernet0/0",
        " ip address 192.0.2.1 255.255.255.0",
        " ipv6 address 2001:0DB8::1/64",
        " no shutdown",
        "!",
    ]);
    compare(&mut analysis, &parse(&progress).unwrap());
    assert!(
        analysis
            .checkpoints
            .iter()
            .all(|c| c.status == Status::Passed)
    );
    compare(
        &mut analysis,
        &parse(&progress.replace(" no shutdown", " shutdown")).unwrap(),
    );
    assert_eq!(analysis.checkpoints[2].status, Status::Failed);
    compare(
        &mut analysis,
        &parse(&progress.replace(
            " no shutdown",
            " description unspecified administrative state",
        ))
        .unwrap(),
    );
    assert_eq!(analysis.checkpoints[2].status, Status::Unverified);
    compare(
        &mut analysis,
        &parse(&progress.replace(" ip address 192.0.2.1 255.255.255.0", " ip address dhcp"))
            .unwrap(),
    );
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
}

#[test]
fn hostname_is_distinct_from_display_name_and_banner_text_is_not_configuration() {
    let xml = format!(
        "<PACKETTRACER5_ACTIVITY>{}<COMPARISONS>{}</COMPARISONS></PACKETTRACER5_ACTIVITY>",
        network("192.0.2.1"),
        branch(
            "Network",
            &branch(
                "R1",
                &leaf("Hostname", "Host Name", "router-production", "1")
            )
        )
    );
    let mut analysis = parse(&xml).unwrap();
    compare(&mut analysis, &parse(&network("192.0.2.1")).unwrap());
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
    compare(
        &mut analysis,
        &parse(&ios_network(&["hostname router-production", "!"])).unwrap(),
    );
    assert_eq!(analysis.checkpoints[0].status, Status::Passed);
    compare(
        &mut analysis,
        &parse(&ios_network(&[
            "banner motd ^",
            "hostname router-production",
            "^",
        ]))
        .unwrap(),
    );
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
}

#[test]
fn conflicting_xml_and_explicit_ios_values_are_unverified() {
    let mut analysis = parse(&fixture()).unwrap();
    let progress = network("192.0.2.1").replace("</ENGINE>", "<RUNNINGCONFIG><LINE>interface GigabitEthernet0/0</LINE><LINE> ip address 192.0.2.2 255.255.255.0</LINE><LINE>!</LINE></RUNNINGCONFIG></ENGINE>");
    compare(&mut analysis, &parse(&progress).unwrap());
    assert_eq!(analysis.checkpoints[0].status, Status::Unverified);
}

#[test]
fn native_container_round_trip_corruption_and_utf8_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.pka");
    let xml = fixture();
    let bytes = pktfile::encode(&xml).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(load(&path).unwrap().checkpoints.len(), 1);
    let mut corrupt = bytes;
    corrupt[8] ^= 0x40;
    std::fs::write(&path, &corrupt).unwrap();
    assert!(load(&path).is_err());
    for data in [
        vec![1, 2, 3],
        b"<PACKETTRACER5>\xff</PACKETTRACER5>".to_vec(),
    ] {
        std::fs::write(&path, data).unwrap();
        assert!(load(&path).is_err());
    }
    std::fs::write(&path, format!("\u{feff}{xml}")).unwrap();
    assert_eq!(load(&path).unwrap().checkpoints.len(), 1);
}

/// Opt-in local verification: no source data, paths, answers, or decoded XML
/// are included in the repository or printed by this test.
#[test]
#[ignore = "requires an explicitly supplied legitimate local PKA path"]
fn real_local_pka_verification() {
    let path =
        std::env::var_os("PKA_INSPECTOR_LOCAL_SAMPLE").expect("set PKA_INSPECTOR_LOCAL_SAMPLE");
    let analysis = load(std::path::Path::new(&path)).unwrap();
    assert!(!analysis.xml.is_empty());
    assert!(analysis.summary().total_score.is_finite());
    assert!(
        analysis
            .checkpoints
            .iter()
            .all(|c| c.status == Status::Unverified)
    );
    println!(
        "Native PKA decoded and parsed: {} leaves, {} diagnostics; all unverified",
        analysis.checkpoints.len(),
        analysis.diagnostics.len()
    );
}
