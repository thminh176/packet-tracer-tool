use super::*;
use crate::network_values::{
    ParsedIpv6StaticRoute, ParsedOspfNetwork, ParsedStaticRoute, classful_prefix,
};
use std::net::Ipv4Addr;
const OSPF: &str = "https://www.cisco.com/c/en/us/td/docs/routers/ios/config/17-x/ip-routing/b-ip-routing/m_iro-cfg-0.html";
const STATIC: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/iproute_pi/command/iri-cr-book/iri-cr-a1.html";
const IPV6: &str = "https://www.cisco.com/c/en/us/td/docs/routers/ios/config/17-x/ip-routing/b-ip-routing/m_ip6-route-static-xe.html";
const RIP: &str = "https://www.cisco.com/c/en/us/td/docs/routers/ios/config/17-x/ip-routing/b-ip-routing/m_irr-cfg-info-prot-0.html";
const EIGRP: &str = "https://www.cisco.com/c/en/us/td/docs/routers/ios/config/17-x/ip-routing/b-ip-routing/m_ire-enhanced-igrp.html";
pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec(
            "router.ipv6.routing",
            "Set IPv6 forwarding",
            "Đặt chuyển tiếp IPv6",
            "[no] ipv6 unicast-routing",
            "global configuration",
            Capability::Ipv6Routing,
            vec![param(
                "enabled",
                ParameterKind::Choice(&["true", "false"]),
                true,
                "Explicit forwarding state",
                "Trạng thái chuyển tiếp rõ ràng",
                "true",
            )],
            "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/ipv6/command/ipv6-cr-book/ipv6-i5.html",
            "Cisco classic IOS IPv6",
            vec!["IPv6-capable router; disabling forwarding affects all IPv6 transit traffic"],
            vec!["show ipv6 route", "show running-config"],
        ),
        spec(
            "router.route.ipv4",
            "Add IPv4 static route",
            "Thêm tuyến tĩnh IPv4",
            "ip route <network> <mask> <target> [distance]",
            "global configuration",
            Capability::Routing,
            vec![param(
                "route",
                ParameterKind::Text,
                true,
                "Complete structured static route",
                "Tuyến tĩnh có đủ tham số",
                "10.0.0.0 255.0.0.0 192.0.2.2",
            )],
            STATIC,
            "Cisco IOS protocol-independent command reference; cross-checked IOS XE 17.x",
            vec![
                "Destination has no host bits; next hop reachable; existing exit interface",
                "Ethernet interface-only routes assume every destination is on-link and may require proxy ARP; prefer a next hop for remote destinations",
                "Distance 255 is accepted syntax but prevents route installation",
            ],
            vec!["show ip route", "show running-config"],
        ),
        spec(
            "router.route.ipv6",
            "Add IPv6 static route",
            "Thêm tuyến tĩnh IPv6",
            "ipv6 route <prefix> <target> [distance]",
            "global configuration",
            Capability::Ipv6Routing,
            vec![param(
                "route",
                ParameterKind::Text,
                true,
                "Explicit prefix and target; interface for link-local",
                "Tiền tố và đích rõ ràng; cổng cho link-local",
                "2001:db8::/64 2001:db8:1::2",
            )],
            IPV6,
            "Cisco classic IOS IPv6",
            vec![
                "IPv6 forwarding enabled; next hop reachable; link-local next hop requires interface",
                "Exit interface must be up and IPv6-enabled; interface-only targets assume on-link destinations",
            ],
            vec!["show ipv6 route"],
        ),
        spec(
            "router.ospf.network",
            "Enable OSPF on network",
            "Bật OSPF trên mạng",
            "router ospf <process_id>; network <network> <wildcard> area <area>",
            "router configuration",
            Capability::DynamicRouting,
            vec![
                param(
                    "process_id",
                    ParameterKind::Number(1, 65534),
                    true,
                    "Explicit local OSPF process (65535 is assessment sentinel)",
                    "Tiến trình OSPF rõ ràng",
                    "10",
                ),
                network_param(),
                param(
                    "area",
                    ParameterKind::Identifier,
                    true,
                    "Explicit decimal or dotted area ID",
                    "Vùng OSPF rõ ràng",
                    "0",
                ),
            ],
            OSPF,
            "Cisco classic IOS syntax; cross-checked IOS XE 17.x",
            vec![
                "Explicit process and area; configured IPv4 interfaces and intended area design",
                "A stable unique router ID and compatible peer area, timers, authentication and network type are required for adjacencies",
            ],
            vec![
                "show ip ospf",
                "show ip ospf interface",
                "show ip ospf neighbor",
            ],
        ),
        spec(
            "router.eigrp.network",
            "Enable classic EIGRP on network",
            "Bật EIGRP cổ điển trên mạng",
            "router eigrp <as_number>; network <network> <wildcard>",
            "router configuration",
            Capability::DynamicRouting,
            vec![
                param(
                    "as_number",
                    ParameterKind::Number(1, 65535),
                    true,
                    "Explicit classic EIGRP AS",
                    "AS EIGRP rõ ràng",
                    "100",
                ),
                network_param(),
            ],
            EIGRP,
            "Cisco classic autonomous-system mode; cross-checked IOS XE 17.x",
            vec!["Matching EIGRP AS and configured IPv4 interfaces; no named mode inference"],
            vec!["show ip eigrp neighbors", "show ip protocols"],
        ),
        spec(
            "router.rip.network",
            "Enable RIP on classful network",
            "Bật RIP trên mạng classful",
            "router rip; version <version>; network <network>",
            "router configuration",
            Capability::DynamicRouting,
            vec![
                param(
                    "version",
                    ParameterKind::Choice(&["1", "2"]),
                    true,
                    "Explicit RIP version",
                    "Phiên bản RIP rõ ràng",
                    "2",
                ),
                network_param(),
            ],
            RIP,
            "Cisco classic IOS syntax; cross-checked IOS XE 17.x",
            vec![
                "Explicit classful network prefix; configured IPv4 interface; existing summary policy preserved",
            ],
            vec!["show ip protocols", "show ip route rip"],
        ),
    ]
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    router_device(req)?;
    match req.family.as_str() {
        "router.ipv6.routing" => {
            let text = match req.required("enabled")? {
                "true" => "ipv6 unicast-routing",
                "false" => "no ipv6 unicast-routing",
                _ => return Err("enabled must be true or false".into()),
            };
            Ok(vec![at(ConfigMode::Global, text)])
        }
        "router.route.ipv4" => {
            let route = ParsedStaticRoute::parse(req.required("route")?)?;
            let text = route.to_cisco_command().ok_or("Incomplete IPv4 route")?;
            if let Some(interface) = route.exit_interface.as_deref() {
                validate_interface(req, interface)?;
            }
            Ok(vec![at(ConfigMode::Global, text)])
        }
        "router.route.ipv6" => {
            let route =
                ParsedIpv6StaticRoute::parse(req.required("route")?).ok_or("Invalid IPv6 route")?;
            if let Some(interface) = route.exit_interface.as_deref() {
                validate_interface(req, interface)?;
            }
            if route
                .next_hop
                .is_some_and(|ip| ip.to_ipv4_mapped().is_some())
            {
                return Err("IPv4-mapped IPv6 next hop is not supported".into());
            }
            let text = route
                .to_cisco_command()
                .ok_or("Incomplete IPv6 route or missing interface for link-local next hop")?;
            Ok(vec![at(ConfigMode::Global, text)])
        }
        "router.ospf.network" => {
            let process = number(req, "process_id", 1, 65534)?;
            let net = network(req, "network")?;
            let area = req.required("area")?;
            if !area.bytes().all(|b| b.is_ascii_digit()) && area.parse::<Ipv4Addr>().is_err() {
                return Err("OSPF area must be unsigned decimal or dotted IPv4 notation".into());
            }
            let ospf = ParsedOspfNetwork::parse(&net.cidr())?
                .with_process_id(process)
                .with_area(area);
            let command = ospf
                .to_cisco_command()
                .ok_or("Invalid explicit OSPF process or area")?;
            Ok(vec![at(
                ConfigMode::Router(format!("ospf {process}")),
                command,
            )])
        }
        "router.eigrp.network" => {
            let as_number = number(req, "as_number", 1, 65535)?;
            let net = network(req, "network")?;
            Ok(vec![at(
                ConfigMode::Router(format!("eigrp {as_number}")),
                format!("network {}", net.network_and_wildcard()),
            )])
        }
        "router.rip.network" => {
            let version = req.required("version")?;
            if !matches!(version, "1" | "2") {
                return Err("RIP version must be explicit 1 or 2".into());
            }
            let net = network(req, "network")?;
            if net.prefix != classful_prefix(net.network)
                || net.network.octets()[0] == 0
                || net.network.octets()[0] == 127
                || net.network.octets()[0] >= 224
                || net.network == Ipv4Addr::LOCALHOST
            {
                return Err("RIP requires the explicitly supplied classful major network; no automatic widening".into());
            }
            let mode = ConfigMode::Router("rip".into());
            Ok(vec![
                at(mode.clone(), format!("version {version}")),
                at(mode, format!("network {}", net.network)),
            ])
        }
        _ => Err("Unknown routing family".into()),
    }
}
