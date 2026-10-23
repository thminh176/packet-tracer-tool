//! Aggregate-only local probe for permitted copies. No answers or device names are printed.
use pka_lab_inspector::{commands, inspector};
use std::{collections::BTreeMap, path::PathBuf};
fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("Supply a permitted copy or directory of copies"))?,
    );
    let mut paths = if root.is_dir() {
        std::fs::read_dir(&root)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .is_some_and(|s| matches!(s.to_str(), Some("pka" | "pkt" | "xml")))
            })
            .collect::<Vec<_>>()
    } else {
        vec![root]
    };
    anyhow::ensure!(paths.len() <= 128, "At most 128 explicitly supplied copies");
    paths.sort();
    let mut counts = BTreeMap::<&str, [usize; 2]>::new();
    let mut encap_with_vlan = 0;
    let mut decoded = 0;
    for path in paths {
        let a = inspector::load(&path)?;
        decoded += 1;
        for c in &a.checkpoints {
            let key = c
                .property
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>();
            let family = match key.as_str() {
                "ipv6unicastrouting" => "ipv6_forwarding",
                "encapsulation" if c.expected == "EthernetEncapProcess" => "dot1q_object",
                "vlanid" if c.path.to_ascii_lowercase().contains("802.1q") => "dot1q_vlan_leaf",
                _ => continue,
            };
            let n = counts.entry(family).or_default();
            n[0] += 1;
            n[1] += usize::from(commands::generate(c).supported);
            if family == "dot1q_object"
                && a.checkpoints.iter().any(|other| {
                    other.device == c.device
                        && other.interface == c.interface
                        && other.property.eq_ignore_ascii_case("VLAN ID")
                        && other.path.to_ascii_lowercase().contains("802.1q")
                })
            {
                encap_with_vlan += 1;
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"decoded_files":decoded,"requirement_and_command_counts":counts,"encap_objects_with_selected_vlan_sibling":encap_with_vlan})
    );
    Ok(())
}
