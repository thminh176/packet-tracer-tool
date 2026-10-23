//! Explicit scalar routing controls. No default state or operational outcome is inferred.
use super::*;
use crate::cisco::{self, CommandRequest};
pub(super) fn validate(c: &Checkpoint, config: &parser::ParsedConfig) -> Option<ValidationResult> {
    let property = crate::commands::property_key(c);
    let (parameter, prefix) = match property.as_str() {
        "ospfrouterid" => ("router_id", "router-id "),
        "ospfreferencebandwidth" => ("mbps", "auto-cost reference-bandwidth "),
        "eigrprouterid" => ("router_id", "eigrp router-id "),
        "ripversion" => ("version", "version "),
        "ripautosummary" => ("enabled", ""),
        "ospfpassiveinterface" | "rippassiveinterface" | "eigrppassiveinterface" => ("state", ""),
        _ => return None,
    };
    Some(check(c, config, parameter, prefix))
}
fn check(
    c: &Checkpoint,
    config: &parser::ParsedConfig,
    parameter: &str,
    prefix: &str,
) -> ValidationResult {
    let Ok(resolved) = crate::commands::explicit_value(c) else {
        return unknown("Unsupported or ambiguous routing requirement");
    };
    let Some(request) = cisco::mapping::request(c, &resolved) else {
        return unknown("No reliable routing control mapping");
    };
    if cisco::mapping::parameter_conflict(c, &resolved) {
        return unknown("Conflicting routing process or parameter evidence");
    }
    if !matches!(
        request.profile,
        cisco::DeviceProfile::GenericRouter
            | cisco::DeviceProfile::Router1941
            | cisco::DeviceProfile::Router2911
            | cisco::DeviceProfile::Isr4321
    ) {
        return unknown("A routing-capable device identity is required for this validator subset");
    }
    let required = cisco::generate(&request);
    if !required.executable() || required.commands.len() != 1 {
        return unknown("Routing control lacks complete supported parameters");
    }
    let command = &required.commands[0];
    let entries = config.in_mode(&command.mode);
    let mut candidates = Vec::<String>::new();
    for line in entries {
        if !prefix.is_empty() {
            if let Some(value) = line.strip_prefix(prefix) {
                candidates.push(value.into());
            }
        } else if parameter == "enabled" {
            match line {
                "auto-summary" => candidates.push("true".into()),
                "no auto-summary" => candidates.push("false".into()),
                _ => (),
            }
        } else {
            let (value, name) = if let Some(name) = line.strip_prefix("passive-interface ") {
                ("passive", name)
            } else if let Some(name) = line.strip_prefix("no passive-interface ") {
                ("active", name)
            } else {
                continue;
            };
            if parser::interface_name(name) == parser::interface_name(request.get("interface")) {
                candidates.push(value.into());
            }
        }
    }
    if candidates.len() != 1 {
        return unknown(
            "No unique explicit value in the required routing process; defaults are not inferred",
        );
    }
    let mut observed: CommandRequest = request.clone();
    observed
        .parameters
        .insert(parameter.into(), candidates[0].clone());
    let actual = cisco::generate(&observed);
    if !actual.executable() || actual.commands.len() != 1 {
        return unknown("Current routing control is outside the supported syntax subset");
    }
    let actual_command = &actual.commands[0];
    ValidationResult {
        status: if actual_command == command {Status::Passed} else {Status::Failed},
        current: Some(actual_command.text.clone()),
        explanation: "Compared the explicit configured value in the identified routing process; neighbor/forwarding operation is not verified".into(),
        evidence: vec![format!("expected assessment / {}: {}",c.path,command.text), format!("current configuration / {:?}: {}",actual_command.mode,actual_command.text)],
    }
}
