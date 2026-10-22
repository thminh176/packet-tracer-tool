use super::*;
use std::net::{Ipv4Addr, Ipv6Addr};
const SOURCE: &str = "https://www.cisco.com/c/en/us/td/docs/routers/ios/config/17-x/ip-addressing/b-ip-addressing/m_config-ipv4-addr-0.html";
const DESCRIPTION: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/lanswitch/configuration/15-mt/lsw-15-mt-book/lsw-hwic-ethsw-ic.html";
const SHUTDOWN: &str = "https://www.cisco.com/E-Learning/bulk/public/tac/cim/cib/using_cisco_ios_software/cmdrefs/shutdown.htm";
const IPV6: &str =
    "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/ipv6/command/ipv6-cr-book/ipv6-i1.html";
const DOT1Q: &str = "https://www.cisco.com/c/en/us/td/docs/routers/ios/config/17-x/lan-wan/b-lan-wan/m_lsw-conf-rout-vlan.html";
pub fn specs() -> Vec<CommandSpec> {
    let mut result = vec![];
    for (id, en, vi, syntax, fields, source, capability) in [
        (
            "router.interface.ipv4",
            "Set interface IPv4 address",
            "Đặt địa chỉ IPv4 cho cổng",
            "ip address <address> <mask>",
            vec![
                param(
                    "address",
                    ParameterKind::Ipv4,
                    true,
                    "Host IPv4 address",
                    "Địa chỉ IPv4",
                    "192.0.2.1",
                ),
                param(
                    "mask",
                    ParameterKind::Ipv4,
                    true,
                    "Contiguous subnet mask",
                    "Mặt nạ liên tục",
                    "255.255.255.0",
                ),
            ],
            SOURCE,
            Capability::Routing,
        ),
        (
            "router.interface.ipv6",
            "Set interface IPv6 address",
            "Đặt địa chỉ IPv6 cho cổng",
            "ipv6 address <address/prefix>",
            vec![param(
                "address",
                ParameterKind::Ipv6Prefix,
                true,
                "IPv6 address and prefix",
                "Địa chỉ IPv6 và tiền tố",
                "2001:db8::1/64",
            )],
            IPV6,
            Capability::Ipv6Routing,
        ),
        (
            "router.interface.link_local",
            "Set IPv6 link-local address",
            "Đặt địa chỉ IPv6 link-local",
            "ipv6 address <address> link-local",
            vec![param(
                "address",
                ParameterKind::Ipv6,
                true,
                "FE80::/10 address",
                "Địa chỉ FE80::/10",
                "fe80::1",
            )],
            IPV6,
            Capability::Ipv6Routing,
        ),
        (
            "router.interface.description",
            "Describe interface",
            "Mô tả cổng",
            "description <description>",
            vec![param(
                "description",
                ParameterKind::Text,
                true,
                "Single-line description",
                "Mô tả một dòng",
                "LAN uplink",
            )],
            SOURCE,
            Capability::Basic,
        ),
        (
            "router.interface.shutdown",
            "Set administrative state",
            "Đặt trạng thái quản trị",
            "[no] shutdown",
            vec![param(
                "state",
                ParameterKind::Choice(&["shutdown", "no shutdown"]),
                true,
                "Explicit administrative state",
                "Trạng thái quản trị rõ ràng",
                "no shutdown",
            )],
            SOURCE,
            Capability::Basic,
        ),
        (
            "router.interface.dot1q",
            "Encapsulate Ethernet subinterface",
            "Đóng gói VLAN cho cổng con",
            "encapsulation dot1Q <vlan> [native]",
            vec![
                param(
                    "vlan",
                    ParameterKind::Number(1, 4094),
                    true,
                    "802.1Q VLAN",
                    "VLAN 802.1Q",
                    "10",
                ),
                param(
                    "native",
                    ParameterKind::Choice(&["true", "false"]),
                    false,
                    "Native VLAN explicitly requested",
                    "Yêu cầu VLAN native",
                    "false",
                ),
            ],
            DOT1Q,
            Capability::Routing,
        ),
    ] {
        let mut parameters = vec![interface_param()];
        parameters.extend(fields);
        let mut prerequisites = vec![
            "Existing supported interface; addressing requires a router routed interface",
            "Review current configuration before applying",
        ];
        if matches!(id, "router.interface.ipv4" | "router.interface.ipv6") {
            prerequisites.push("Ethernet subinterface addressing requires VLAN encapsulation configured first; parent must be operational");
        }
        if id == "router.interface.ipv4" {
            prerequisites.push("A /31 IPv4 subnet requires a point-to-point link; /32 generation is limited to loopbacks");
        }
        if id == "router.interface.dot1q" {
            prerequisites.push("Parent Ethernet interface must be operational; peer trunk VLAN/native configuration must match");
        }
        result.push(spec(
            id,
            en,
            vi,
            syntax,
            "interface configuration",
            capability,
            parameters,
            match id {
                "router.interface.description" => DESCRIPTION,
                "router.interface.shutdown" => SHUTDOWN,
                _ => source,
            },
            "Cisco classic IOS (command reference); interface descriptions IOS 15M&T",
            prerequisites,
            vec!["show interfaces", "show running-config"],
        ));
    }
    result
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    let interface = iface(req)?;
    let text = match req.family.as_str() {
        "router.interface.ipv4" => {
            router_device(req)?;
            let ip = ipv4(req, "address")?;
            let mask: Ipv4Addr = req
                .required("mask")?
                .parse()
                .map_err(|_| "Invalid subnet mask")?;
            let prefix = crate::network_values::subnet_to_prefix(mask)
                .ok_or("Non-contiguous subnet mask")?;
            if prefix == 0 || (prefix == 32 && !interface.to_ascii_lowercase().starts_with("lo")) {
                return Err(
                    "Use a nonzero interface mask; /32 requires a loopback interface".into(),
                );
            }
            if prefix < 31 {
                let host = u32::from(ip) & !u32::from(mask);
                if host == 0 || host == !u32::from(mask) {
                    return Err("Interface IPv4 is network or broadcast address".into());
                }
            }
            format!("ip address {ip} {mask}")
        }
        "router.interface.ipv6" => {
            router_device(req)?;
            let (address, prefix) = req
                .required("address")?
                .split_once('/')
                .ok_or("Explicit IPv6 prefix required")?;
            let ip: Ipv6Addr = address.parse().map_err(|_| "Invalid IPv6 address")?;
            let length: u8 = prefix.parse().map_err(|_| "Invalid IPv6 prefix")?;
            if length > 128
                || ip.is_unspecified()
                || ip.is_loopback()
                || ip.is_multicast()
                || ip.to_ipv4_mapped().is_some()
                || (ip.segments()[0] & 0xffc0) == 0xfe80
            {
                return Err("Use a unicast IPv6 address; link-local has a separate family".into());
            }
            format!("ipv6 address {ip}/{length}")
        }
        "router.interface.link_local" => {
            router_device(req)?;
            let ip: Ipv6Addr = req
                .required("address")?
                .parse()
                .map_err(|_| "Invalid IPv6 address")?;
            if (ip.segments()[0] & 0xffc0) != 0xfe80 {
                return Err("Link-local address must be FE80::/10".into());
            }
            format!("ipv6 address {ip} link-local")
        }
        "router.interface.description" => {
            let description = req.required("description")?;
            if description.len() > 240 {
                return Err("Interface description must be at most 240 bytes".into());
            }
            format!("description {description}")
        }
        "router.interface.shutdown" => match req.required("state")? {
            "shutdown" => "shutdown".into(),
            "no shutdown" => "no shutdown".into(),
            _ => return Err("Explicit shutdown or no shutdown required".into()),
        },
        "router.interface.dot1q" => {
            router_device(req)?;
            let (base, sub) = interface
                .split_once('.')
                .ok_or("802.1Q requires an explicit Ethernet subinterface")?;
            let lower = base.to_ascii_lowercase();
            if ![
                "gigabitethernet",
                "fastethernet",
                "ethernet",
                "tengigabitethernet",
                "gi",
                "fa",
                "eth",
                "te",
            ]
            .iter()
            .any(|p| lower.starts_with(p))
                || sub.parse::<u32>().ok().filter(|n| *n > 0).is_none()
            {
                return Err("802.1Q requires an Ethernet subinterface".into());
            }
            let vlan = number(req, "vlan", 1, 4094)?;
            let native = match req.get("native") {
                "" | "false" => "",
                "true" => " native",
                _ => return Err("native must be true or false".into()),
            };
            format!("encapsulation dot1Q {vlan}{native}")
        }
        _ => return Err("Unknown interface family".into()),
    };
    Ok(vec![at(ConfigMode::Interface(interface), text)])
}
