//! Exact selected scalar assignments, distinct from verified procedural workflows.
//! Never manufacture current port mode, VLAN activity, peer state or other requirements.
use super::*;

pub(super) fn plan(c: &Checkpoint) -> Option<Result<Plan, &'static str>> {
    let p = property_key(c);
    if !matches!(
        p.as_str(),
        "accessvlan" | "nativevlan" | "allowedvlans" | "vlanname"
    ) {
        return None;
    }
    Some(build(c, &p))
}
fn vlan(s: &str) -> Option<u16> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse::<u16>()
        .ok()
        .filter(|v| (1..=4094).contains(v) && !(1002..=1005).contains(v))
}
pub(super) fn vlan_context(c: &Checkpoint) -> Option<u16> {
    let parts: Vec<_> = c.path.split(" / ").collect();
    let ids: Vec<_> = parts
        .windows(2)
        .filter(|pair| key(pair[0]) == "vlans")
        .filter_map(|pair| vlan(pair[1]))
        .collect();
    (ids.len() == 1).then(|| ids[0])
}
fn build(c: &Checkpoint, property: &str) -> Result<Plan, &'static str> {
    use crate::cisco::DeviceProfile;
    if let Some(request) = crate::cisco::mapping::request(c, &c.expected)
        && let Some(spec) = crate::cisco::catalog()
            .iter()
            .find(|s| s.id == request.family)
        && request.parameters.iter().any(|(name, value)| {
            !value.is_empty()
                && spec
                    .parameters
                    .iter()
                    .find(|field| field.name == name)
                    .is_none_or(|field| !crate::cisco::valid_parameter(&field.kind, value))
        })
    {
        return Err("Invalid explicit parameter evidence; no conditional command generated");
    }
    if !matches!(
        crate::cisco::mapping::profile(c),
        DeviceProfile::GenericSwitch | DeviceProfile::Catalyst2960 | DeviceProfile::Catalyst3560
    ) {
        return Err("A known switch identity is required for a switchport assignment");
    }
    let mut plan = Plan::default();
    if property == "vlanname" {
        let id = vlan_context(c)
            .ok_or("VLAN name requires one explicit VLAN ID in the assessment branch")?;
        if c.expected.is_empty()
            || c.expected.len() > 32
            || !c
                .expected
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err("VLAN name requires 1..32 ASCII letters, digits, underscore or hyphen");
        }
        plan.context.push(format!("vlan {id}"));
        plan.lines.push(format!("name {}", c.expected));
        return Ok(plan);
    }
    if !crate::cisco::switch::physical_port(&c.interface) {
        return Err("An explicit physical switch Ethernet port is required");
    }
    if c.evidence
        .iter()
        .any(|e| e == "commands:parameter:vlan_active=no")
    {
        return Err(
            "VLAN is explicitly evidenced inactive; review VLAN creation or activation first",
        );
    }
    if property == "accessvlan"
        && c.evidence.iter().any(|e| {
            matches!(
                e.as_str(),
                "commands:parameter:interface_mode=trunk"
                    | "commands:parameter:interface_mode=routed"
            )
        })
    {
        return Err("Access VLAN requirement conflicts with explicitly evidenced port mode");
    }
    if matches!(property, "nativevlan" | "allowedvlans")
        && c.evidence.iter().any(|e| {
            matches!(
                e.as_str(),
                "commands:parameter:interface_mode=access"
                    | "commands:parameter:interface_mode=routed"
            )
        })
    {
        return Err("Native VLAN requirement conflicts with explicitly evidenced port mode");
    }
    if property == "nativevlan"
        && c.evidence
            .iter()
            .filter_map(|e| e.strip_prefix("commands:parameter:peer_native_vlan="))
            .any(|peer| vlan(peer) != vlan(c.expected.trim()))
    {
        return Err("Native VLAN requirement conflicts with explicit peer VLAN");
    }
    if property == "allowedvlans"
        && c.evidence
            .iter()
            .filter_map(|e| e.strip_prefix("commands:parameter:operation="))
            .any(|operation| operation != "replace")
    {
        return Err(
            "A complete allowed VLAN assignment cannot silently change an explicit list operation",
        );
    }
    plan.context.push(format!("interface {}", c.interface));
    plan.lines.push(match property {
        "accessvlan" => format!(
            "switchport access vlan {}",
            vlan(c.expected.trim()).ok_or("Invalid Ethernet VLAN ID")?
        ),
        "nativevlan" => format!(
            "switchport trunk native vlan {}",
            vlan(c.expected.trim()).ok_or("Invalid Ethernet VLAN ID")?
        ),
        "allowedvlans" => {
            let list = c.expected.split_whitespace().collect::<Vec<_>>().join(",");
            crate::cisco::vlan_list(&list).ok_or("Invalid complete allowed VLAN list")?;
            format!("switchport trunk allowed vlan {list}")
        }
        _ => return Err("Unsupported scalar assignment"),
    });
    Ok(plan)
}
