//! Explicit unicast NTP configuration, not synchronization or clock authority.
use super::*;
const SOURCE: &str =
    "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/bsm/command/bsm-cr-book/bsm-cr-n1.html";
pub fn specs() -> Vec<CommandSpec> {
    let server = CommandSpec {
        id: "basic.ntp_server",
        name_en: "IPv4/IPv6 NTP server association",
        name_vi: "Máy chủ NTP IPv4/IPv6",
        topic: "Basics and security",
        syntax: "ntp server <server> [version <version>] [prefer]",
        mode: "global",
        capability: Capability::Basic,
        parameters: vec![
            ParameterSpec {
                name: "server",
                kind: ParameterKind::IpAddress,
                required: true,
                description_en: "Explicit unicast time server",
                description_vi: "Máy chủ thời gian unicast rõ ràng",
                example: "192.0.2.123",
            },
            ParameterSpec {
                name: "version",
                kind: ParameterKind::Choice(&["2", "3", "4"]),
                required: false,
                description_en: "IPv6 requires explicit version 4; optional for IPv4",
                description_vi: "IPv6 bắt buộc chỉ định phiên bản 4; tùy chọn với IPv4",
                example: "3",
            },
            ParameterSpec {
                name: "prefer",
                kind: ParameterKind::Choice(&["true"]),
                required: false,
                description_en: "Explicitly prefer this server; removal is unsupported",
                description_vi: "Ưu tiên máy chủ này; chưa hỗ trợ gỡ ưu tiên",
                example: "true",
            },
        ],
        prerequisites: vec![
            "Verify a reachable trusted time server and UDP/123 return path; review authentication requirements separately",
            "Configuration presence does not establish synchronization. No ntp master, trusted key, service removal or local time is inferred",
        ],
        verification: vec![
            "show ntp associations",
            "show ntp status",
            "show clock detail",
            "show running-config",
        ],
        source: SOURCE,
        software: "Classic IOS system management command reference; hardware/image and PT support untested",
        removal: None,
        semantic_validation: false,
    };
    let mut source = server.clone();
    source.id = "basic.ntp_source";
    source.name_en = "Global NTP source interface";
    source.name_vi = "Cổng nguồn NTP toàn cục";
    source.syntax = "ntp source <interface>";
    source.parameters = vec![
        ParameterSpec {
            name: "interface",
            kind: ParameterKind::Interface,
            required: true,
            description_en: "Existing interface with the intended source address",
            description_vi: "Cổng hiện có với địa chỉ nguồn mong muốn",
            example: "Loopback0",
        },
        ParameterSpec {
            name: "address_present",
            kind: ParameterKind::Choice(&["yes"]),
            required: true,
            description_en: "Operator confirms a configured source address and return reachability",
            description_vi: "Người dùng xác nhận có địa chỉ nguồn và đường trả về",
            example: "yes",
        },
    ];
    source.prerequisites = vec![
        "Confirm the source interface has an address reachable by NTP servers; this assertion is not hardware discovery",
        "A per-server source option overrides this global source. No interface addressing or NTP association is created implicitly",
    ];
    vec![server, source]
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    let text = match req.family.as_str() {
        "basic.ntp_server" => {
            let server = req
                .required("server")?
                .parse::<std::net::IpAddr>()
                .map_err(|_| "Invalid NTP server address")?;
            match server {
                std::net::IpAddr::V4(ip)
                    if ip.octets()[0] == 0 || ip.is_loopback() || ip.octets()[0] >= 224 =>
                {
                    return Err("NTP server must be a usable remote unicast address".into());
                }
                std::net::IpAddr::V6(ip) => {
                    if ip.is_unspecified()
                        || ip.is_loopback()
                        || ip.is_multicast()
                        || ip.to_ipv4_mapped().is_some()
                        || (ip.segments()[0] & 0xffc0) == 0xfe80
                    {
                        return Err("Use a global/ULA unicast IPv6 NTP server; scoped link-local is unsupported".into());
                    }
                    if matches!(
                        req.profile,
                        DeviceProfile::GenericSwitch
                            | DeviceProfile::Catalyst2960
                            | DeviceProfile::Catalyst3560
                    ) {
                        return Err("IPv6 NTP support on Catalyst images is not established by this workflow".into());
                    }
                    if req.get("version").is_empty() {
                        return Err("Missing required parameter: version".into());
                    }
                    if req.get("version") != "4" {
                        return Err("IPv6 NTP requires explicit version 4".into());
                    }
                }
                _ => (),
            }
            let mut line = format!("ntp server {server}");
            match req.get("version") {
                "" => (),
                v @ ("2" | "3" | "4") => line.push_str(&format!(" version {v}")),
                _ => return Err("NTP version must be 2, 3 or 4".into()),
            }
            match req.get("prefer") {
                "true" => line.push_str(" prefer"),
                "" => (),
                _ => return Err("Only explicit NTP preference enablement is supported".into()),
            }
            line
        }
        "basic.ntp_source" => {
            if req.required("address_present")? != "yes" {
                return Err("Explicit source address context required".into());
            }
            let interface = parser::interface_name(req.required("interface")?)
                .ok_or("Invalid source interface")?;
            let lower = interface.to_ascii_lowercase();
            if matches!(
                req.profile,
                DeviceProfile::Catalyst2960 | DeviceProfile::GenericSwitch
            ) && !lower.starts_with("vlan")
            {
                return Err("Layer 2 switch NTP source requires a management SVI workflow".into());
            }
            if let Some(vlan) = lower.strip_prefix("vlan")
                && !vlan.parse::<u16>().is_ok_and(|n| (1..=4094).contains(&n))
            {
                return Err("Invalid management SVI VLAN ID".into());
            }
            format!("ntp source {interface}")
        }
        _ => return Err("Unknown NTP family".into()),
    };
    Ok(vec![at(ConfigMode::Global, text)])
}
