//! Read-only Packet Tracer inspection. No activity scores or source files are modified.
use crate::model::{Analysis, Checkpoint, Status};
use anyhow::{Context, Result, bail, ensure};
use quick_xml::{Reader, events::Event};
use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
    path::Path,
    time::Instant,
};

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_NODES: usize = 1_000_000;
const MAX_DEPTH: usize = 128;

#[derive(Default, Debug)]
struct Node {
    tag: String,
    attrs: BTreeMap<String, String>,
    text: String,
    children: Vec<Node>,
}
impl Node {
    fn child(&self, tag: &str) -> Option<&Node> {
        self.children.iter().find(|n| n.tag == tag)
    }
    fn value(&self, tag: &str) -> &str {
        self.child(tag).map(|n| n.text.trim()).unwrap_or("")
    }
    fn attr(&self, name: &str) -> &str {
        self.attrs.get(name).map(String::as_str).unwrap_or("")
    }
    fn unique_child(&self, tag: &str) -> Option<&Node> {
        let mut children = self.children.iter().filter(|n| n.tag == tag);
        let first = children.next()?;
        if children.next().is_some() {
            None
        } else {
            Some(first)
        }
    }
    fn unique_value(&self, tag: &str) -> Option<&str> {
        self.unique_child(tag).map(|n| n.text.trim())
    }
}

/// Decode the modern authenticated container using the MIT pktfile library.
/// The repository's narrowly patched MIT codec bounds decompression and checks
/// the declared XML length; this adapter also limits input and parser complexity.
pub fn load(path: &Path) -> Result<Analysis> {
    let started = Instant::now();
    ensure!(
        std::fs::metadata(path)?.len() <= MAX_BYTES as u64,
        "Input exceeds 64 MiB limit"
    );
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .with_context(|| format!("Read {}", path.display()))?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let read_ms = started.elapsed().as_secs_f64() * 1000.0;
    let decode_started = Instant::now();
    ensure!(bytes.len() <= MAX_BYTES, "Input exceeds 64 MiB limit");
    let plain = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    // An encrypted container can coincidentally start with '<'. Require both
    // valid UTF-8 and a recognizable XML prefix before selecting plain XML.
    let plain_xml = std::str::from_utf8(plain).ok().filter(|text| {
        let text = text.trim_start();
        text.starts_with("<?xml") || text.starts_with("<PACKETTRACER")
    });
    let xml = if let Some(text) = plain_xml {
        text.to_owned()
    } else {
        std::panic::catch_unwind(|| pktfile::decode(&bytes))
            .map_err(|_| anyhow::anyhow!("Native pktfile decoder rejected malformed container"))?
            .context("Native pktfile decode failed (damaged or unsupported container)")?
    };
    let decode_ms = decode_started.elapsed().as_secs_f64() * 1000.0;
    let mut analysis = parse(&xml)?;
    analysis.timings.push(("Read (ms)".into(), read_ms));
    analysis.timings.push(("Decode (ms)".into(), decode_ms));
    analysis.timings.push((
        "load total (ms)".into(),
        started.elapsed().as_secs_f64() * 1000.0,
    ));
    Ok(analysis)
}

fn document(xml: &str) -> Result<Node> {
    ensure!(xml.len() <= MAX_BYTES, "XML exceeds 64 MiB limit");
    let mut reader = Reader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let mut stack = vec![Node::default()];
    let mut count = 0;
    loop {
        let event = reader.read_event().context("Malformed XML")?;
        let empty = matches!(&event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                count += 1;
                ensure!(
                    count <= MAX_NODES && stack.len() <= MAX_DEPTH,
                    "XML complexity limit exceeded"
                );
                let mut node = Node {
                    tag: std::str::from_utf8(e.name().as_ref())?.to_owned(),
                    ..Default::default()
                };
                for attr in e.attributes() {
                    let attr = attr.context("Malformed or duplicate XML attribute")?;
                    node.attrs.insert(
                        std::str::from_utf8(attr.key.as_ref())?.to_owned(),
                        attr.decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )?
                        .into_owned(),
                    );
                }
                if empty {
                    stack.last_mut().unwrap().children.push(node);
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                ensure!(stack.len() > 1, "Unexpected closing tag");
                let node = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(node);
            }
            Event::Text(e) => {
                stack
                    .last_mut()
                    .unwrap()
                    .text
                    .push_str(&quick_xml::escape::unescape(
                        &e.xml_content(quick_xml::XmlVersion::Implicit1_0)?,
                    )?)
            }
            Event::CData(e) => stack
                .last_mut()
                .unwrap()
                .text
                .push_str(&e.xml_content(quick_xml::XmlVersion::Implicit1_0)?),
            Event::GeneralRef(e) => {
                let name = e.decode()?;
                let escaped = format!("&{name};");
                stack.last_mut().unwrap().text.push_str(
                    &quick_xml::escape::unescape(&escaped).context("Unknown XML entity")?,
                );
            }
            Event::DocType(_) => bail!("DTD and external entities are unsupported"),
            Event::Eof => break,
            _ => (),
        }
    }
    ensure!(stack.len() == 1, "Truncated XML");
    let mut doc = stack.pop().unwrap();
    ensure!(
        doc.children.len() == 1 && doc.text.trim().is_empty(),
        "Expected exactly one XML root"
    );
    Ok(doc.children.remove(0))
}

fn descendants<'a>(node: &'a Node, tag: &str, out: &mut Vec<&'a Node>) {
    if node.tag == tag {
        out.push(node);
    }
    for child in &node.children {
        descendants(child, tag, out);
    }
}
fn node_count(node: &Node) -> usize {
    1 + node.children.iter().map(node_count).sum::<usize>()
}
fn role(node: &Node, inherited: &str) -> String {
    let mut roles = Vec::new();
    for value in [
        node.tag.as_str(),
        node.attr("role"),
        node.attr("ROLE"),
        node.attr("type"),
        node.attr("TYPE"),
        node.value("ROLE"),
        inherited,
    ] {
        match value.to_ascii_lowercase().as_str() {
            "answer" | "answer_network" | "answernetwork" => roles.push("answer"),
            "initial" | "initial_network" | "initialnetwork" => roles.push("initial"),
            "current" | "working" | "current_network" | "currentnetwork" => roles.push("current"),
            _ => (),
        }
    }
    roles.sort();
    roles.dedup();
    if roles.len() == 1 {
        roles[0].into()
    } else if roles.is_empty() {
        inherited.into()
    } else {
        "ambiguous".into()
    }
}
fn snapshots<'a>(node: &'a Node, inherited: &str, out: &mut Vec<(&'a Node, String)>) {
    let r = role(node, inherited);
    if node.tag == "PACKETTRACER5" {
        out.push((node, r));
        return;
    }
    for child in &node.children {
        snapshots(child, &r, out);
    }
}

fn label(node: &Node) -> &str {
    let id = node.value("ID");
    if !id.is_empty() {
        id
    } else {
        node.value("NAME")
    }
}

/// Bound derived strings before assessment materialization. Small XML can
/// otherwise multiply a long ancestor label into thousands of full paths.
fn assessment_budget(
    node: &Node,
    path_bytes: usize,
    total: &mut usize,
    leaves: &mut usize,
) -> Result<()> {
    let mut path_bytes = path_bytes;
    if node.tag == "NODE" {
        ensure!(
            label(node).len() <= 512 && node.value("NAME").len() <= 512,
            "Assessment label exceeds 512 byte limit"
        );
        path_bytes = path_bytes.saturating_add(label(node).len() + 3);
        ensure!(
            path_bytes <= 4096,
            "Assessment path exceeds 4096 byte limit"
        );
        if !node.children.iter().any(|c| c.tag == "NODE") {
            *total = total.saturating_add(path_bytes.saturating_mul(3));
            *leaves += 1;
            ensure!(
                *total <= 8 * 1024 * 1024 && *leaves <= 50_000,
                "Assessment derived output budget exceeded"
            );
        }
        if let Some(name) = node.child("NAME") {
            ensure!(
                name.attr("nodeValue").len() <= 4096,
                "Assessment value exceeds 4096 byte limit"
            );
        }
    }
    for child in &node.children {
        assessment_budget(child, path_bytes, total, leaves)?;
    }
    Ok(())
}
fn assessment(
    node: &Node,
    ancestry: &mut Vec<String>,
    out: &mut Vec<Checkpoint>,
    diagnostics: &mut Vec<String>,
    inventory: &mut usize,
) {
    if node.tag != "NODE" {
        for c in &node.children {
            assessment(c, ancestry, out, diagnostics, inventory);
        }
        return;
    }
    ancestry.push(label(node).to_owned());
    let nested: Vec<_> = node.children.iter().filter(|n| n.tag == "NODE").collect();
    let empty_group = node
        .child("NAME")
        .is_some_and(|n| n.attr("headNode") == "true" && n.attr("nodeValue").is_empty())
        && node.value("POINTS").is_empty();
    if nested.is_empty() && !empty_group {
        let name = node.child("NAME");
        // checkType tracks Qt's 0/1/2 check states in observed PT trees.
        // Unchecked/partial terminal inventory is not an assessed checkpoint.
        if name.is_some_and(|n| {
            matches!(n.attr("checkType"), "0" | "1")
                || ["enabled", "selected", "checked"]
                    .iter()
                    .any(|a| matches!(n.attr(a), "false" | "no" | "0"))
        }) {
            *inventory += 1;
            ancestry.pop();
            return;
        }
        let weight = if node.value("POINTS").is_empty() {
            0.0
        } else {
            match node.value("POINTS").parse::<f64>() {
                Ok(w) if w.is_finite() && w >= 0.0 => w,
                _ => {
                    diagnostics.push(format!(
                        "Invalid weight at {}: excluded from score",
                        ancestry.join(" / ")
                    ));
                    0.0
                }
            }
        };
        let expected = name.map(|n| n.attr("nodeValue")).unwrap_or("").to_owned();
        let unsupported = name.is_some_and(|n| {
            n.attr("checkType") != "2"
                || (n.attr("variableEnabled") == "true" && !n.attr("variableName").is_empty())
                || n.attr("overrideDBGrading") == "true"
        }) || node.children.iter().filter(|n| n.tag == "NAME").count() != 1;
        let property = if node.value("NAME").is_empty() {
            label(node)
        } else {
            node.value("NAME")
        };
        out.push(Checkpoint {
            id: ancestry.join(" / "),
            path: ancestry.join(" / "),
            property: property.to_owned(),
            category: ancestry
                .get(ancestry.len().saturating_sub(2))
                .cloned()
                .unwrap_or_default(),
            expected,
            weight,
            explanation: if unsupported {
                "Custom, variable, disabled or ambiguous assessment requires Packet Tracer grading"
            } else {
                "No verified current snapshot evidence"
            }
            .into(),
            evidence: vec![if unsupported {
                "assessment:unsupported".into()
            } else {
                "assessment:compare".into()
            }],
            ..Default::default()
        });
    } else {
        for c in nested {
            assessment(c, ancestry, out, diagnostics, inventory);
        }
    }
    ancestry.pop();
}

/// Extract selected assessment leaves, including unknown checks. Never infer
/// snapshot roles from their order, scores, or similarity to expected values.
pub fn parse(xml: &str) -> Result<Analysis> {
    let started = Instant::now();
    let root = document(xml)?;
    let xml_parse_ms = started.elapsed().as_secs_f64() * 1000.0;
    let extraction_started = Instant::now();
    ensure!(
        matches!(
            root.tag.as_str(),
            "PACKETTRACER5" | "PACKETTRACER5_ACTIVITY" | "PACKETTRACER_ACTIVITY"
        ),
        "Unsupported Packet Tracer XML root"
    );
    let mut result = Analysis {
        xml: xml.to_owned(),
        ..Default::default()
    };
    let mut trees = Vec::new();
    descendants(&root, "COMPARISONS", &mut trees);
    let inventory_only = trees.is_empty();
    if inventory_only {
        descendants(&root, "INITIALSETUP", &mut trees);
    }
    let mut output_budget = 0;
    let mut assessment_leaves = 0;
    for tree in &trees {
        assessment_budget(tree, 0, &mut output_budget, &mut assessment_leaves)?;
    }
    let mut inventory = 0;
    for tree in trees {
        assessment(
            tree,
            &mut Vec::new(),
            &mut result.checkpoints,
            &mut result.diagnostics,
            &mut inventory,
        );
    }
    if inventory_only && !result.checkpoints.is_empty() {
        inventory += result.checkpoints.len();
        result.checkpoints.clear();
    }
    ensure!(
        result
            .checkpoints
            .iter()
            .map(|c| c.weight)
            .sum::<f64>()
            .is_finite(),
        "Aggregate assessment weight exceeds finite score range"
    );
    if inventory > 0 {
        result.diagnostics.push(format!("Excluded {inventory} unchecked/partial/disabled terminal inventory nodes from assessment counts and scores; original XML retained"));
    }
    if !result.checkpoints.is_empty() {
        result.diagnostics.push("Selection uses observed NAME checkType=2 (Qt checked state); Packet Tracer schema mapping is inferred, not a certified grading contract".into());
    }
    if inventory_only && inventory > 0 {
        result
            .diagnostics
            .push("No COMPARISONS tree; INITIALSETUP leaves are an unverified inventory".into());
    }
    let mut snaps = Vec::new();
    snapshots(&root, "", &mut snaps);
    let mut devices = Vec::new();
    for (snapshot, _) in &snaps {
        descendants(snapshot, "DEVICE", &mut devices);
    }
    ensure!(
        result.checkpoints.len().saturating_mul(devices.len()) <= 5_000_000,
        "Device/checkpoint matching complexity budget exceeded"
    );
    ensure!(
        result
            .checkpoints
            .len()
            .saturating_mul(snaps.iter().map(|(n, _)| node_count(n)).sum::<usize>())
            <= 50_000_000,
        "Snapshot matching operation budget exceeded"
    );
    for checkpoint in &mut result.checkpoints {
        let mut names: Vec<_> = devices
            .iter()
            .map(|d| d.child("ENGINE").unwrap_or(d).value("NAME"))
            .filter(|n| !n.is_empty() && checkpoint.path.split(" / ").any(|p| p == *n))
            .collect();
        names.sort();
        names.dedup();
        if names.len() == 1 {
            checkpoint.device = names[0].to_owned();
            let matching: Vec<_> = devices
                .iter()
                .filter(|d| d.child("ENGINE").unwrap_or(d).value("NAME") == checkpoint.device)
                .collect();
            let mut types: Vec<_> = matching
                .iter()
                .map(|d| d.child("ENGINE").unwrap_or(d).value("TYPE"))
                .filter(|s| !s.is_empty())
                .collect();
            types.sort();
            types.dedup();
            if types.len() == 1 {
                checkpoint.device_type = types[0].into();
            }
            let mut ports = Vec::new();
            for d in matching {
                descendants(d, "PORT", &mut ports);
            }
            let mut port_names: Vec<_> = ports
                .iter()
                .map(|p| p.value("NAME"))
                .filter(|n| !n.is_empty() && checkpoint.path.split(" / ").any(|p| p == *n))
                .collect();
            port_names.sort();
            port_names.dedup();
            if port_names.len() == 1 {
                checkpoint.interface = port_names[0].into();
            }
        }
        if checkpoint.interface.is_empty() {
            checkpoint.interface = checkpoint
                .path
                .split(" / ")
                .find(|s| interface_name(s))
                .unwrap_or("")
                .to_owned();
        }
    }
    let mut ids = HashMap::<String, usize>::new();
    for c in &result.checkpoints {
        *ids.entry(c.id.clone()).or_default() += 1;
    }
    for c in &mut result.checkpoints {
        if ids[&c.id] > 1 {
            c.evidence.push("assessment:ambiguous".into());
            c.explanation = "Duplicate assessment identity; retained without validation".into();
        }
    }
    attach_command_context(&mut result.checkpoints);
    if root.tag != "PACKETTRACER5" && snaps.iter().any(|(_, r)| r.is_empty() || r == "ambiguous") {
        result.diagnostics.push("Embedded snapshot roles are ambiguous; no positional role assumptions or automatic grading applied".into());
    }
    if result.checkpoints.is_empty() {
        result
            .diagnostics
            .push("No selected assessment leaves found; network state alone is not an assessment and custom/script scoring is not evaluated".into());
    }
    result.timings.push(("XML parse (ms)".into(), xml_parse_ms));
    result.timings.push((
        "Assessment extraction (ms)".into(),
        extraction_started.elapsed().as_secs_f64() * 1000.0,
    ));
    if snaps.iter().any(|(_, r)| r == "current") {
        let progress = Analysis {
            xml: xml.to_owned(),
            ..Default::default()
        };
        compare(&mut result, &progress);
    }
    result.timings.push((
        "parse (ms)".into(),
        started.elapsed().as_secs_f64() * 1000.0,
    ));
    Ok(result)
}

fn interface_name(s: &str) -> bool {
    [
        "FastEthernet",
        "GigabitEthernet",
        "Ethernet",
        "Serial",
        "Loopback",
        "Vlan",
    ]
    .iter()
    .any(|p| {
        s.starts_with(p)
            && s[p.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
    })
}
fn sentinel(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "" | "--"
            | "n/a"
            | "none"
            | "null"
            | "unknown"
            | "<not set>"
            | "not set"
            | "-1"
            | "65535"
            | "4294967295"
            | "0.0.0.0"
            | "::"
    ) || s.contains("${")
}
fn property_tag(property: &str, path: &str) -> Option<&'static str> {
    if crate::commands::is_static_route_property(property, path) {
        return Some("STATIC_ROUTE");
    }
    if crate::commands::is_ipv6_static_route_property(property, path) {
        return Some("IPV6_STATIC_ROUTE");
    }
    if crate::commands::is_ospf_network_property(property, path) {
        return Some("OSPF_NETWORK");
    }
    match property.trim().to_ascii_lowercase().as_str() {
        "ip address" | "ipv4 address" => Some("IP"),
        "subnet mask" => Some("SUBNET"),
        "default gateway" => Some("PORT_GATEWAY"),
        "hostname" | "host name" => Some("HOSTNAME"),
        "display name" => Some("NAME"),
        "ipv6 address" | "ipv6 addresses" => Some("IPV6_ADDRESSES"),
        "ipv6 prefix length" => Some("IPV6_PREFIX"),
        "link local" | "link local address" | "ipv6 link local address" => Some("IPV6_LINK_LOCAL"),
        "ipv6 default gateway" => Some("IPV6_PORT_GATEWAY"),
        "port status" => Some("POWER"),
        "ipv6 enable" => Some("IPV6_ENABLED"),
        "ipv6 auto config" => Some("IPV6_ADDRESS_AUTOCONFIG"),
        _ => None,
    }
}
fn subnet_mask(value: &str) -> bool {
    mask_prefix(value).is_some()
}
fn mask_prefix(value: &str) -> Option<u8> {
    let value = value.trim();
    if !value.contains('.') {
        return value
            .trim_start_matches('/')
            .parse::<u8>()
            .ok()
            .filter(|p| (1..=32).contains(p));
    }
    value.parse::<std::net::Ipv4Addr>().ok().and_then(|mask| {
        let inverse = !u32::from(mask);
        inverse
            .checked_add(1)
            .filter(|next| inverse & next == 0)
            .map(|_| u32::from(mask).count_ones() as u8)
    })
}
fn ipv6(value: &str) -> Option<(std::net::Ipv6Addr, Option<u8>)> {
    let (address, prefix) = if let Some((address, prefix)) = value.trim().split_once('/') {
        (
            address,
            Some(prefix.parse::<u8>().ok().filter(|p| *p <= 128)?),
        )
    } else {
        (value.trim(), None)
    };
    let address = address.parse::<std::net::Ipv6Addr>().ok()?;
    if address.is_unspecified() {
        return None;
    }
    Some((address, prefix))
}
fn semantic_equal(tag: &str, expected: &str, actual: &str) -> Option<bool> {
    match tag {
        "IP" | "PORT_GATEWAY" => Some(
            expected.parse::<std::net::Ipv4Addr>().ok()?
                == actual.parse::<std::net::Ipv4Addr>().ok()?,
        ),
        "SUBNET" => Some(mask_prefix(expected)? == mask_prefix(actual)?),
        "IPV6_ADDRESSES" | "IPV6_LINK_LOCAL" | "IPV6_PORT_GATEWAY" => {
            let (e, ep) = ipv6(expected)?;
            let (a, ap) = ipv6(actual)?;
            if ep.is_some() && ap.is_none() {
                return None;
            }
            Some(e == a && (ep.is_none() || ep == ap))
        }
        "IPV6_PREFIX" => Some(
            expected.parse::<u8>().ok().filter(|p| *p <= 128)?
                == actual.parse::<u8>().ok().filter(|p| *p <= 128)?,
        ),
        "POWER" | "IPV6_ENABLED" | "IPV6_ADDRESS_AUTOCONFIG" => {
            let boolean = |s: &str| match s.trim().to_ascii_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
            Some(boolean(expected)? == boolean(actual)?)
        }
        "HOSTNAME"
            if expected.contains(char::is_whitespace) || actual.contains(char::is_whitespace) =>
        {
            None
        }
        "STATIC_ROUTE" => {
            let exp_route = crate::network_values::ParsedStaticRoute::parse(expected).ok()?;
            let act_route = crate::network_values::ParsedStaticRoute::parse(actual).ok()?;
            if exp_route.destination.network != act_route.destination.network
                || exp_route.destination.prefix != act_route.destination.prefix
            {
                return Some(false);
            }
            if let Some(ref exp_nh) = exp_route.next_hop {
                if act_route.next_hop.as_ref() != Some(exp_nh) {
                    return Some(false);
                }
            }
            if let Some(ref exp_iface) = exp_route.exit_interface {
                let act_iface = act_route.exit_interface.as_deref().unwrap_or("");
                if !act_iface.eq_ignore_ascii_case(exp_iface) {
                    return Some(false);
                }
            }
            if let Some(exp_dist) = exp_route.distance {
                if exp_dist > 1 && act_route.distance != Some(exp_dist) {
                    return Some(false);
                }
            }
            Some(true)
        }
        "IPV6_STATIC_ROUTE" => {
            let exp = crate::network_values::ParsedIpv6StaticRoute::parse(expected)?;
            let act = crate::network_values::ParsedIpv6StaticRoute::parse(actual)?;
            if exp.destination != act.destination || exp.prefix != act.prefix {
                return Some(false);
            }
            if let Some(ref exp_nh) = exp.next_hop {
                if act.next_hop.as_ref() != Some(exp_nh) {
                    return Some(false);
                }
            }
            if let Some(ref exp_iface) = exp.exit_interface {
                let act_iface = act.exit_interface.as_deref().unwrap_or("");
                if !act_iface.eq_ignore_ascii_case(exp_iface) {
                    return Some(false);
                }
            }
            if let Some(exp_dist) = exp.distance {
                if exp_dist > 1 && act.distance != Some(exp_dist) {
                    return Some(false);
                }
            }
            Some(true)
        }
        "OSPF_NETWORK" => {
            let exp = crate::network_values::ParsedOspfNetwork::parse(expected).ok()?;
            let act = crate::network_values::ParsedOspfNetwork::parse(actual).ok()?;
            if exp.network != act.network || exp.wildcard_mask != act.wildcard_mask {
                return Some(false);
            }
            if let (Some(ea), Some(aa)) = (&exp.area, &act.area) {
                let norm_ea = if ea == "0.0.0.0" { "0" } else { ea.as_str() };
                let norm_aa = if aa == "0.0.0.0" { "0" } else { aa.as_str() };
                if norm_ea != norm_aa {
                    return Some(false);
                }
            } else if exp.area.is_some() != act.area.is_some() {
                return Some(false);
            }
            if let Some(exp_pid) = exp.process_id {
                if act.process_id.is_some() && act.process_id != Some(exp_pid) {
                    return Some(false);
                }
            }
            Some(true)
        }
        "NAME" | "HOSTNAME" => Some(expected.trim() == actual.trim()),
        _ => None,
    }
}
fn attach_command_context(checks: &mut [Checkpoint]) {
    let mut masks = HashMap::<(String, String, String), Vec<String>>::new();
    let mut addresses = HashMap::<(String, String, String), Vec<String>>::new();
    let mut prefixes = HashMap::<(String, String, String), Vec<u8>>::new();
    let context = |c: &Checkpoint| {
        (
            c.device.clone(),
            c.interface.clone(),
            c.path
                .rsplit_once(" / ")
                .map(|p| p.0)
                .unwrap_or("")
                .to_owned(),
        )
    };
    for c in checks.iter() {
        if property_tag(&c.property, &c.path) == Some("IP")
            && !c.device.is_empty()
            && !c.interface.is_empty()
            && c.evidence.iter().all(|e| e == "assessment:compare")
            && c.expected.parse::<std::net::Ipv4Addr>().is_ok()
            && !sentinel(&c.expected)
        {
            addresses
                .entry(context(c))
                .or_default()
                .push(c.expected.clone());
        }
        if property_tag(&c.property, &c.path) == Some("SUBNET")
            && !c.device.is_empty()
            && !c.interface.is_empty()
            && c.evidence.iter().all(|e| e == "assessment:compare")
            && subnet_mask(&c.expected)
            && !sentinel(&c.expected)
        {
            masks.entry(context(c)).or_default().push(
                std::net::Ipv4Addr::from(u32::MAX << (32 - mask_prefix(&c.expected).unwrap()))
                    .to_string(),
            );
        }
        if property_tag(&c.property, &c.path) == Some("IPV6_PREFIX")
            && !c.device.is_empty()
            && !c.interface.is_empty()
            && c.evidence.iter().all(|e| e == "assessment:compare")
            && let Ok(prefix) = c.expected.parse::<u8>()
            && prefix <= 128
        {
            prefixes.entry(context(c)).or_default().push(prefix);
        }
    }
    for c in checks {
        if property_tag(&c.property, &c.path) == Some("SUBNET")
            && c.evidence.iter().all(|e| e == "assessment:compare")
            && subnet_mask(&c.expected)
            && let Some(values) = addresses.get(&context(c)).filter(|v| v.len() == 1)
        {
            c.evidence
                .push(format!("commands:ipv4-address={}", values[0]));
        }
        if property_tag(&c.property, &c.path) == Some("IP")
            && c.evidence.iter().all(|e| e == "assessment:compare")
            && c.expected.parse::<std::net::Ipv4Addr>().is_ok()
            && !sentinel(&c.expected)
            && let Some(values) = masks.get(&context(c)).filter(|v| v.len() == 1)
        {
            c.evidence.push(format!("commands:ipv4-mask={}", values[0]));
        }
        if property_tag(&c.property, &c.path) == Some("IPV6_ADDRESSES")
            && !c.device.is_empty()
            && !c.interface.is_empty()
            && c.evidence.iter().all(|e| e == "assessment:compare")
            && let Some((_, explicit_prefix)) = ipv6(&c.expected)
        {
            if let Some(prefix) = explicit_prefix
                && prefixes
                    .get(&context(c))
                    .is_some_and(|values| values.len() != 1 || values[0] != prefix)
            {
                continue;
            }
            let prefix = explicit_prefix.or_else(|| {
                prefixes
                    .get(&context(c))
                    .filter(|p| p.len() == 1)
                    .map(|p| p[0])
            });
            if let Some(prefix) = prefix {
                c.evidence.push(format!("commands:ipv6-prefix={prefix}"));
            }
        }
    }
}
fn ios_observations(device: &Node, interface: &str, tag: &str) -> Vec<String> {
    let mut configs = Vec::new();
    descendants(device, "RUNNINGCONFIG", &mut configs);
    if configs.len() != 1 {
        return Vec::new();
    }
    let lines: Vec<_> = configs[0]
        .children
        .iter()
        .filter(|n| n.tag == "LINE")
        .map(|n| n.text.as_str())
        .collect();
    if lines.iter().any(|line| {
        [
            "banner ",
            "macro name ",
            "crypto pki certificate",
            "event manager applet ",
        ]
        .iter()
        .any(|prefix| line.trim_start().starts_with(prefix))
    }) {
        return Vec::new();
    }
    if tag == "HOSTNAME" && interface.is_empty() {
        return lines
            .iter()
            .filter_map(|line| line.trim().strip_prefix("hostname ").map(str::trim))
            .filter(|name| !name.is_empty() && !name.contains(char::is_whitespace))
            .map(str::to_owned)
            .collect();
    }
    if tag == "STATIC_ROUTE" && interface.is_empty() {
        return lines
            .iter()
            .filter_map(|line| line.trim().strip_prefix("ip route ").map(str::trim))
            .filter(|r| !r.is_empty())
            .map(str::to_owned)
            .collect();
    }
    if tag == "IPV6_STATIC_ROUTE" && interface.is_empty() {
        return lines
            .iter()
            .filter_map(|line| line.trim().strip_prefix("ipv6 route ").map(str::trim))
            .filter(|r| !r.is_empty())
            .map(str::to_owned)
            .collect();
    }
    if tag == "OSPF_NETWORK" && interface.is_empty() {
        let mut active_pid: Option<u32> = None;
        let mut statements = Vec::new();
        for line in &lines {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("router ospf ") {
                active_pid = rest.trim().parse::<u32>().ok().filter(|&p| p > 0 && p < 65535);
                continue;
            }
            if matches!(trimmed, "!" | "exit" | "end")
                || (!line.starts_with(char::is_whitespace) && !trimmed.is_empty())
            {
                active_pid = None;
            }
            if let Some(pid) = active_pid {
                if let Some(rest) = trimmed.strip_prefix("network ") {
                    let words: Vec<&str> = rest.split_whitespace().collect();
                    if words.len() == 4 && words[2] == "area" {
                        statements.push(format!("{pid} {} {} {}", words[0], words[1], words[3]));
                    }
                }
            }
        }
        return statements;
    }
    if interface.is_empty() {
        return Vec::new();
    }
    let mut active = false;
    let mut sections = 0;
    let mut values = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("interface ") {
            active = name.eq_ignore_ascii_case(interface);
            if active {
                sections += 1;
            }
            continue;
        }
        if matches!(trimmed, "!" | "exit" | "end")
            || (!line.starts_with(char::is_whitespace) && !trimmed.is_empty())
        {
            active = false;
        }
        if !active {
            continue;
        }
        let words: Vec<_> = trimmed.split_whitespace().collect();
        match tag {
            "IP" | "SUBNET" if words.starts_with(&["ip", "address"]) => {
                if words.len() != 4 {
                    return Vec::new();
                }
                values.push(words[if tag == "IP" { 2 } else { 3 }].to_owned());
            }
            "IPV6_ADDRESSES" | "IPV6_LINK_LOCAL" | "IPV6_PREFIX"
                if words.starts_with(&["ipv6", "address"]) =>
            {
                if words.len() == 4 && words[3] == "link-local" {
                    if tag == "IPV6_LINK_LOCAL" {
                        values.push(words[2].to_owned());
                    }
                } else if words.len() == 3 && words[2].contains('/') {
                    if tag == "IPV6_ADDRESSES" {
                        values.push(words[2].to_owned());
                    }
                    if tag == "IPV6_PREFIX"
                        && let Some((_, prefix)) = words[2].split_once('/')
                    {
                        values.push(prefix.into());
                    }
                } else {
                    return Vec::new();
                }
            }
            "POWER" if trimmed == "shutdown" => values.push("false".into()),
            "POWER" if trimmed == "no shutdown" => values.push("true".into()),
            "IPV6_ENABLED" if trimmed == "ipv6 enable" => values.push("true".into()),
            "IPV6_ENABLED" if trimmed == "no ipv6 enable" => values.push("false".into()),
            _ => (),
        }
    }
    if sections == 1 { values } else { Vec::new() }
}
fn observations(node: &Node, device: &str, interface: &str, tag: &str) -> Vec<String> {
    let mut devices = Vec::new();
    descendants(node, "DEVICE", &mut devices);
    let matching: Vec<_> = devices
        .into_iter()
        .filter(|d| {
            let engine = d.unique_child("ENGINE").unwrap_or(d);
            engine.unique_value("NAME") == Some(device)
        })
        .collect();
    if matching.len() != 1 {
        return Vec::new();
    }
    let engine = matching[0].unique_child("ENGINE").unwrap_or(matching[0]);
    if matches!(tag, "HOSTNAME" | "STATIC_ROUTE" | "IPV6_STATIC_ROUTE" | "OSPF_NETWORK") {
        return ios_observations(matching[0], interface, tag);
    }
    if interface.is_empty() {
        return engine
            .unique_value(tag)
            .map(|v| vec![v.into()])
            .unwrap_or_default();
    }
    let mut ports = Vec::new();
    descendants(engine, "PORT", &mut ports);
    let ports: Vec<_> = ports
        .into_iter()
        .filter(|p| p.unique_value("NAME") == Some(interface))
        .collect();
    if ports.is_empty() {
        return ios_observations(matching[0], interface, tag);
    }
    if ports.len() != 1 {
        return Vec::new();
    }
    if matches!(tag, "IPV6_ADDRESSES" | "IPV6_PREFIX") {
        let Some(addresses) = ports[0].unique_child("IPV6_ADDRESSES") else {
            return Vec::new();
        };
        let mut values = Vec::new();
        for address in &addresses.children {
            if address.tag != "IPV6_ADDRESS" {
                return Vec::new();
            }
            let (Some(ip), Some(prefix)) = (
                address.unique_value("ADDRESS"),
                address.unique_value("PREFIX"),
            ) else {
                return Vec::new();
            };
            if ipv6(&format!("{ip}/{prefix}")).is_none() {
                return Vec::new();
            }
            values.push(if tag == "IPV6_PREFIX" {
                prefix.to_owned()
            } else {
                format!("{ip}/{prefix}")
            });
        }
        return consistent_observations(values, ios_observations(matching[0], interface, tag), tag);
    }
    let values = ports[0]
        .unique_value(tag)
        .map(|v| vec![v.into()])
        .unwrap_or_default();
    consistent_observations(values, ios_observations(matching[0], interface, tag), tag)
}
fn consistent_observations(values: Vec<String>, ios: Vec<String>, tag: &str) -> Vec<String> {
    if !ios.is_empty()
        && (values.len() != ios.len()
            || values.iter().any(|value| {
                !ios.iter()
                    .any(|other| semantic_equal(tag, value, other) == Some(true))
            }))
    {
        Vec::new()
    } else {
        values
    }
}

/// Validate a small supported semantic subset against an explicitly identified
/// current network or standalone PKT. Missing/ambiguous/unsupported stays unknown.
pub fn compare(activity: &mut Analysis, progress: &Analysis) {
    let validation_started = Instant::now();
    for check in &mut activity.checkpoints {
        check.status = Status::Unverified;
        check.current = None;
        check
            .evidence
            .retain(|s| s.starts_with("assessment:") || s.starts_with("commands:"));
        check.explanation = "No unique supported current-state observation".into();
    }
    let root = match document(&progress.xml) {
        Ok(root) => root,
        Err(e) => {
            activity
                .diagnostics
                .push(format!("Progress XML invalid: {e}"));
            return;
        }
    };
    if activity.checkpoints.len().saturating_mul(node_count(&root)) > 50_000_000 {
        activity
            .diagnostics
            .push("Validation operation budget exceeded; results remain Unverified".into());
        return;
    }
    if !matches!(
        root.tag.as_str(),
        "PACKETTRACER5" | "PACKETTRACER5_ACTIVITY" | "PACKETTRACER_ACTIVITY"
    ) {
        activity
            .diagnostics
            .push("Progress has an unsupported XML root".into());
        return;
    }
    let mut snaps = Vec::new();
    snapshots(&root, "", &mut snaps);
    let current: Vec<_> = snaps
        .iter()
        .filter(|(_, r)| root.tag == "PACKETTRACER5" || r == "current")
        .collect();
    for check in &mut activity.checkpoints {
        if current.len() != 1
            || check.device.is_empty()
            || sentinel(&check.expected)
            || check.evidence.iter().any(|s| {
                matches!(
                    s.as_str(),
                    "assessment:unsupported" | "assessment:ambiguous" | "assessment:inventory"
                )
            })
        {
            continue;
        }
        let Some(tag) = property_tag(&check.property, &check.path) else {
            check.explanation = "Unsupported assessment check type; not evaluated".into();
            continue;
        };
        if matches!(
            tag,
            "NAME" | "HOSTNAME" | "STATIC_ROUTE" | "IPV6_STATIC_ROUTE" | "OSPF_NETWORK"
        ) && !check.interface.is_empty()
        {
            continue;
        }
        let values = observations(current[0].0, &check.device, &check.interface, tag);
        if values.is_empty() {
            if matches!(tag, "STATIC_ROUTE" | "IPV6_STATIC_ROUTE" | "OSPF_NETWORK") {
                let mut configs = Vec::new();
                descendants(current[0].0, "RUNNINGCONFIG", &mut configs);
                if configs.len() == 1 {
                    check.current = Some("none".into());
                    check.status = Status::Failed;
                    check.explanation = match tag {
                        "STATIC_ROUTE" => "No IPv4 static routes configured on device".into(),
                        "IPV6_STATIC_ROUTE" => "No IPv6 static routes configured on device".into(),
                        "OSPF_NETWORK" => "No OSPF network statements configured on device".into(),
                        _ => "No configuration found on device".into(),
                    };
                    check.evidence.push(format!(
                        "expected assessment / {}: {}",
                        check.path, check.expected
                    ));
                    check.evidence.push(format!(
                        "current network / {} / {tag}: none",
                        check.device
                    ));
                }
            }
            continue;
        }
        if values.len() > 1
            && !matches!(
                tag,
                "IPV6_ADDRESSES" | "STATIC_ROUTE" | "IPV6_STATIC_ROUTE" | "OSPF_NETWORK"
            )
        {
            continue;
        }
        if values.iter().any(|v| sentinel(v)) {
            continue;
        }
        let is_multi = matches!(
            tag,
            "IPV6_ADDRESSES" | "STATIC_ROUTE" | "IPV6_STATIC_ROUTE" | "OSPF_NETWORK"
        );
        let comparisons: Option<Vec<bool>> = values
            .iter()
            .map(|actual| semantic_equal(tag, check.expected.trim(), actual))
            .collect();
        let Some(comparisons) = comparisons else {
            if tag == "STATIC_ROUTE" {
                if let Ok(r) = crate::network_values::ParsedStaticRoute::parse(&check.expected) {
                    if !r.is_complete {
                        check.explanation = format!(
                            "Assessment checkpoint missing required route parameters: {}",
                            r.missing_fields.join(", ")
                        );
                    }
                }
            } else if tag == "OSPF_NETWORK"
                && let Ok(o) = crate::network_values::ParsedOspfNetwork::parse(&check.expected)
                && !o.is_complete
            {
                check.explanation = format!(
                    "Assessment checkpoint missing required OSPF parameters: {}",
                    o.missing_fields.join(", ")
                );
            }
            continue;
        };
        let actual = if is_multi {
            values.join("\n")
        } else {
            values.join(", ")
        };
        check.current = Some(actual.clone());
        check.status = if comparisons.into_iter().any(|equal| equal) {
            Status::Passed
        } else {
            Status::Failed
        };
        check.explanation = match tag {
            "STATIC_ROUTE" => "Compared IPv4 static route against device running-config".into(),
            "IPV6_STATIC_ROUTE" => {
                "Compared IPv6 static route against device running-config".into()
            }
            "OSPF_NETWORK" => {
                "Compared OSPF network statements against device running-config".into()
            }
            _ => {
                "Compared supported explicit values in a uniquely identified device/interface".into()
            }
        };
        check.evidence.push(format!(
            "expected assessment / {}: {}",
            check.path, check.expected
        ));
        check.evidence.push(
            if root.tag == "PACKETTRACER5" {
                "current role: standalone PKT"
            } else {
                "current role: explicit metadata"
            }
            .into(),
        );
        check.evidence.push(format!(
            "current network / {} / {} / {tag}: {actual}",
            check.device, check.interface
        ));
    }
    if current.len() != 1 {
        activity.diagnostics.push("Comparison requires exactly one explicitly identified current network or a standalone PKT".into());
    }
    activity.timings.push((
        "Validation (ms)".into(),
        validation_started.elapsed().as_secs_f64() * 1000.0,
    ));
}
