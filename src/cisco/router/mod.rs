use super::{
    Capability, CommandRequest, CommandSpec, ConfigMode, DeviceProfile, ModeCommand, ParameterKind,
    ParameterSpec, at,
};
use crate::network_values::{NormalizedNetwork, is_valid_interface_name};
mod interfaces;
mod ospf;
mod routing;
mod services;
mod wan;

pub fn specs() -> Vec<CommandSpec> {
    let mut all = interfaces::specs();
    all.extend(routing::specs());
    all.extend(services::specs());
    all.extend(wan::specs());
    all.extend(ospf::specs());
    all
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    let spec = super::catalog()
        .iter()
        .find(|s| s.id == req.family)
        .ok_or("Unknown router family")?;
    for field in &spec.parameters {
        if field.required {
            req.required(field.name)?;
        }
    }
    if req
        .parameters
        .keys()
        .any(|key| !spec.parameters.iter().any(|f| f.name == key))
    {
        return Err("Unknown parameter; no arbitrary CLI is accepted".into());
    }
    if req
        .parameters
        .values()
        .any(|v| !v.is_empty() && !super::safe_text(v))
    {
        return Err("Unsafe parameter".into());
    }
    if matches!(
        req.family.as_str(),
        "router.ospf.router_id" | "router.ospf.passive" | "router.ospf.reference_bandwidth"
    ) {
        ospf::generate(req)
    } else if matches!(
        req.family.as_str(),
        "router.dhcp.relay" | "router.serial.clock" | "router.serial.ppp"
    ) {
        wan::generate(req)
    } else if req.family.starts_with("router.interface.") {
        interfaces::generate(req)
    } else if req.family.starts_with("router.acl.")
        || req.family.starts_with("router.nat.")
        || req.family.starts_with("router.dhcp.")
    {
        services::generate(req)
    } else {
        routing::generate(req)
    }
}
fn param(
    name: &'static str,
    kind: ParameterKind,
    required: bool,
    en: &'static str,
    vi: &'static str,
    example: &'static str,
) -> ParameterSpec {
    ParameterSpec {
        name,
        kind,
        required,
        description_en: en,
        description_vi: vi,
        example,
    }
}
// A metadata record constructor keeps static family definitions readable.
#[allow(clippy::too_many_arguments)]
fn spec(
    id: &'static str,
    en: &'static str,
    vi: &'static str,
    syntax: &'static str,
    mode: &'static str,
    capability: Capability,
    parameters: Vec<ParameterSpec>,
    source: &'static str,
    software: &'static str,
    prerequisites: Vec<&'static str>,
    verification: Vec<&'static str>,
) -> CommandSpec {
    CommandSpec {
        id,
        name_en: en,
        name_vi: vi,
        topic: "Router",
        syntax,
        mode,
        capability,
        parameters,
        prerequisites,
        verification,
        source,
        software,
        removal: None,
        // Parameter validation is not saved-configuration verification.
        semantic_validation: false,
    }
}
fn iface(req: &CommandRequest) -> Result<String, String> {
    let value = req.required("interface")?;
    validate_interface(req, value)?;
    Ok(value.into())
}
fn validate_interface(req: &CommandRequest, value: &str) -> Result<(), String> {
    if value.len() > 128 || !is_valid_interface_name(value) {
        return Err("Invalid interface structure".into());
    }
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("se") && req.profile == DeviceProfile::Isr4321 {
        return Err("Serial interface requires device/module evidence".into());
    }
    // The shared assessment parser accepts structural names, including impossible
    // virtual-interface paths. Generation requires the stricter IOS context.
    if let Some(unit) = lower
        .strip_prefix("loopback")
        .or_else(|| lower.strip_prefix("lo"))
    {
        let n = unit
            .parse::<u32>()
            .map_err(|_| "Loopback requires a single unit number")?;
        if n > 2_147_483_647 {
            return Err("Loopback unit exceeds supported IOS range".into());
        }
    } else if let Some(unit) = lower.strip_prefix("vlan") {
        let n = unit
            .parse::<u32>()
            .map_err(|_| "VLAN interface requires a single VLAN ID")?;
        if !(1..=4094).contains(&n) {
            return Err("VLAN interface ID must be 1..=4094".into());
        }
    }
    Ok(())
}
fn router_device(req: &CommandRequest) -> Result<(), String> {
    if matches!(
        req.profile,
        DeviceProfile::GenericSwitch | DeviceProfile::Catalyst2960 | DeviceProfile::Catalyst3560
    ) {
        return Err(
            "Router-only family: switch routed ports and SVIs require a switch-specific workflow"
                .into(),
        );
    }
    Ok(())
}
fn number(req: &CommandRequest, key: &str, min: u32, max: u32) -> Result<u32, String> {
    let raw = req.required(key)?;
    if !raw.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Invalid {key}"));
    }
    let n = raw.parse::<u32>().map_err(|_| format!("Invalid {key}"))?;
    if !(min..=max).contains(&n) {
        return Err(format!("{key} must be {min}..={max}"));
    }
    Ok(n)
}
fn network(req: &CommandRequest, key: &str) -> Result<NormalizedNetwork, String> {
    let raw = req.required(key)?;
    let net =
        NormalizedNetwork::parse(raw).ok_or("Explicit IPv4 network prefix or mask required")?;
    // Network specifications must not silently truncate host bits.
    let address = raw
        .split(['/', '-', ' '])
        .next()
        .unwrap_or("")
        .parse::<std::net::Ipv4Addr>()
        .map_err(|_| "Invalid network address")?;
    if net.network != address {
        return Err("Network has host bits set".into());
    }
    Ok(net)
}
fn ipv4(req: &CommandRequest, key: &str) -> Result<std::net::Ipv4Addr, String> {
    let ip = req
        .required(key)?
        .parse::<std::net::Ipv4Addr>()
        .map_err(|_| format!("Invalid {key}"))?;
    if ip.octets()[0] == 0 || ip.is_loopback() || ip.octets()[0] >= 224 {
        return Err(format!("{key} must be unicast"));
    }
    Ok(ip)
}
fn interface_param() -> ParameterSpec {
    param(
        "interface",
        ParameterKind::Interface,
        true,
        "Explicit interface",
        "Cổng rõ ràng",
        "GigabitEthernet0/0",
    )
}
fn network_param() -> ParameterSpec {
    param(
        "network",
        ParameterKind::Ipv4Network,
        true,
        "Network with explicit prefix or mask",
        "Mạng có tiền tố hoặc mặt nạ",
        "10.0.0.0/24",
    )
}
