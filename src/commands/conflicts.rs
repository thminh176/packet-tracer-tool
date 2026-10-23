//! Device-wide discovery state dependencies, separate from scalar CLI conflicts.
use super::*;
pub(super) fn canonical(plan: &Plan) -> Plan {
    let mut plan = plan.clone();
    for context in &mut plan.context {
        if let Some(name) = context
            .strip_prefix("interface ")
            .and_then(crate::cisco::parser::interface_name)
        {
            *context = format!("interface {name}");
        }
    }
    for line in &mut plan.lines {
        let (prefix, name) = if let Some(name) = line.strip_prefix("passive-interface ") {
            ("", name)
        } else if let Some(name) = line.strip_prefix("no passive-interface ") {
            ("no ", name)
        } else {
            continue;
        };
        if let Some(name) = crate::cisco::parser::interface_name(name) {
            *line = format!("{prefix}passive-interface {name}");
        }
    }
    plan
}
pub(super) fn discovery_states(items: &[&Checkpoint]) -> HashSet<&'static str> {
    let mut enabled = HashSet::new();
    let mut disabled = HashSet::new();
    for c in items {
        let Some(plan) = conflict_plan(c) else {
            continue;
        };
        for line in &plan.lines {
            match line.as_str() {
                "lldp run" | "lldp transmit" | "lldp receive" => {
                    enabled.insert("lldp");
                }
                "no lldp run" => {
                    disabled.insert("lldp");
                }
                "cdp run" | "cdp enable" => {
                    enabled.insert("cdp");
                }
                "no cdp run" => {
                    disabled.insert("cdp");
                }
                _ => (),
            }
        }
    }
    enabled.intersection(&disabled).copied().collect()
}
pub(super) fn discovery_affected(plan: &Plan, conflicts: &HashSet<&str>) -> bool {
    plan.lines.iter().any(|line| match line.as_str() {
        "lldp run" | "no lldp run" | "lldp transmit" | "lldp receive" => conflicts.contains("lldp"),
        "cdp run" | "no cdp run" | "cdp enable" => conflicts.contains("cdp"),
        _ => false,
    })
}
