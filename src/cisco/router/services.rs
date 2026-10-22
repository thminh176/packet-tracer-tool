use super::*;
use std::net::Ipv4Addr;
const ACL: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/sec_data_acl/configuration/15-sy/sec-data-acl-15-sy-book/sec-create-ip-apply.html";
const NAT: &str = "https://www.cisco.com/c/en/us/td/docs/routers/ios/config/17-x/ip-addressing/b-ip-addressing/m_iadnat-addr-consv-xe.html";
const DHCP: &str = "https://www.cisco.com/c/en/us/td/docs/routers/ios/config/17-x/ip-addressing/b-ip-addressing/m_config-dhcp-server-xe.html";
pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec(
            "router.acl.standard",
            "Append standard numbered ACL rule",
            "Thêm quy tắc ACL chuẩn",
            "access-list <number> <action> <source>",
            "global configuration",
            Capability::Security,
            vec![
                param(
                    "number",
                    ParameterKind::Number(1, 1999),
                    true,
                    "Standard ACL number 1–99 or 1300–1999",
                    "Số ACL chuẩn 1–99 hoặc 1300–1999",
                    "10",
                ),
                param(
                    "action",
                    ParameterKind::Choice(&["permit", "deny"]),
                    true,
                    "Explicit action",
                    "Hành động rõ ràng",
                    "permit",
                ),
                param(
                    "source",
                    ParameterKind::Text,
                    true,
                    "any, host address, or address wildcard",
                    "any, host địa chỉ, hoặc địa chỉ wildcard",
                    "192.0.2.0 0.0.0.255",
                ),
            ],
            ACL,
            "Cisco IOS 15SY",
            vec!["Review existing ACL order and implicit deny; source-only filtering"],
            vec!["show access-lists"],
        ),
        spec(
            "router.acl.apply",
            "Apply IPv4 ACL to interface",
            "Áp dụng ACL IPv4 cho cổng",
            "ip access-group <acl> <direction>",
            "interface configuration",
            Capability::Security,
            vec![
                interface_param(),
                param(
                    "acl",
                    ParameterKind::Identifier,
                    true,
                    "Existing ACL name or number",
                    "Tên hoặc số ACL đã có",
                    "10",
                ),
                param(
                    "direction",
                    ParameterKind::Choice(&["in", "out"]),
                    true,
                    "Explicit traffic direction",
                    "Hướng lưu lượng rõ ràng",
                    "in",
                ),
            ],
            ACL,
            "Cisco IOS 15SY",
            vec!["ACL already defined; review traffic and management reachability"],
            vec!["show ip interface", "show access-lists"],
        ),
        spec(
            "router.nat.static",
            "Add static inside source NAT",
            "Thêm NAT tĩnh nguồn trong",
            "ip nat inside source static <local> <global>",
            "global configuration",
            Capability::Routing,
            vec![
                param(
                    "local",
                    ParameterKind::Ipv4,
                    true,
                    "Inside local address",
                    "Địa chỉ trong cục bộ",
                    "10.0.0.10",
                ),
                param(
                    "global",
                    ParameterKind::Ipv4,
                    true,
                    "Inside global address",
                    "Địa chỉ trong toàn cục",
                    "192.0.2.10",
                ),
            ],
            NAT,
            "Cisco classic IOS syntax; cross-checked IOS XE 17.x",
            vec![
                "Inside/outside interface roles configured; global address routed; no conflicting translations",
            ],
            vec!["show ip nat translations", "show ip nat statistics"],
        ),
        spec(
            "router.nat.interface",
            "Set NAT interface role",
            "Đặt vai trò NAT cho cổng",
            "ip nat <role>",
            "interface configuration",
            Capability::Routing,
            vec![
                interface_param(),
                param(
                    "role",
                    ParameterKind::Choice(&["inside", "outside"]),
                    true,
                    "Explicit NAT role",
                    "Vai trò NAT rõ ràng",
                    "inside",
                ),
            ],
            NAT,
            "Cisco classic IOS syntax; cross-checked IOS XE 17.x",
            vec!["Routed interface and explicit inside/outside topology"],
            vec!["show ip nat statistics"],
        ),
        spec(
            "router.dhcp.pool",
            "Configure explicit DHCP pool",
            "Cấu hình pool DHCP rõ ràng",
            "ip dhcp pool <pool>; network <network> <mask>",
            "DHCP pool configuration",
            Capability::Routing,
            vec![
                param(
                    "pool",
                    ParameterKind::Identifier,
                    true,
                    "DHCP pool name",
                    "Tên pool DHCP",
                    "LAN",
                ),
                network_param(),
                param(
                    "default_router",
                    ParameterKind::Ipv4,
                    false,
                    "Client default router",
                    "Cổng mặc định cho máy khách",
                    "10.0.0.1",
                ),
                param(
                    "dns_server",
                    ParameterKind::Ipv4,
                    false,
                    "Client DNS server",
                    "Máy chủ DNS cho máy khách",
                    "192.0.2.53",
                ),
            ],
            DHCP,
            "Cisco classic IOS syntax; cross-checked IOS XE 17.x",
            vec![
                "Review existing pool and exclusions; DHCP service enabled; no competing server",
                "Receiving interface subnet or relay giaddr must select this pool; exclude externally assigned gateway and server addresses explicitly",
            ],
            vec!["show ip dhcp pool", "show ip dhcp binding"],
        ),
        spec(
            "router.dhcp.exclude",
            "Exclude DHCP address range",
            "Loại trừ dải địa chỉ DHCP",
            "ip dhcp excluded-address <start> [end]",
            "global configuration",
            Capability::Routing,
            vec![
                param(
                    "start",
                    ParameterKind::Ipv4,
                    true,
                    "First excluded address",
                    "Địa chỉ loại trừ đầu",
                    "10.0.0.1",
                ),
                param(
                    "end",
                    ParameterKind::Ipv4,
                    false,
                    "Last excluded address",
                    "Địa chỉ loại trừ cuối",
                    "10.0.0.20",
                ),
            ],
            DHCP,
            "Cisco classic IOS syntax; cross-checked IOS XE 17.x",
            vec!["Range belongs to intended DHCP subnet"],
            vec!["show running-config", "show ip dhcp pool"],
        ),
    ]
}
fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
}
// Client pools and exclusions must not span reserved or loopback space even
// when both supplied endpoints independently look like unicast addresses.
fn client_range(first: Ipv4Addr, last: Ipv4Addr) -> bool {
    let first = u32::from(first);
    let last = u32::from(last);
    first >= u32::from(Ipv4Addr::new(1, 0, 0, 0))
        && last < u32::from(Ipv4Addr::new(224, 0, 0, 0))
        && (last < u32::from(Ipv4Addr::new(127, 0, 0, 0))
            || first >= u32::from(Ipv4Addr::new(128, 0, 0, 0)))
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    router_device(req)?;
    match req.family.as_str() {
        "router.acl.standard" => {
            let number = number(req, "number", 1, 1999)?;
            if !(number <= 99 || number >= 1300) {
                return Err("Standard ACL number must be 1–99 or 1300–1999".into());
            }
            let action = req.required("action")?;
            if !matches!(action, "permit" | "deny") {
                return Err("ACL action must be permit or deny".into());
            }
            let tokens: Vec<_> = req.required("source")?.split_whitespace().collect();
            let source = match tokens.as_slice() {
                ["any"] => "any".into(),
                ["host", address] => {
                    let ip: Ipv4Addr = address.parse().map_err(|_| "Invalid ACL host")?;
                    format!("host {ip}")
                }
                [address, wildcard] => {
                    let ip: Ipv4Addr = address.parse().map_err(|_| "Invalid ACL source")?;
                    let mask: Ipv4Addr = wildcard.parse().map_err(|_| "Invalid ACL wildcard")?;
                    format!("{ip} {mask}")
                }
                _ => {
                    return Err("ACL source requires any, host address, or address wildcard".into());
                }
            };
            Ok(vec![at(
                ConfigMode::Global,
                format!("access-list {number} {action} {source}"),
            )])
        }
        "router.acl.apply" => {
            let interface = iface(req)?;
            let acl = req.required("acl")?;
            if !name(acl) {
                return Err("Invalid ACL identifier".into());
            }
            if acl.chars().all(|c| c.is_ascii_digit()) {
                let n = number(req, "acl", 1, 2699)?;
                if !(n <= 199 || n >= 1300) {
                    return Err("Invalid IPv4 ACL number range".into());
                }
            } else if !acl.as_bytes()[0].is_ascii_alphabetic() {
                return Err("Named IPv4 ACL must begin with an alphabetic character".into());
            }
            let direction = req.required("direction")?;
            if !matches!(direction, "in" | "out") {
                return Err("Explicit ACL direction must be in or out".into());
            }
            Ok(vec![at(
                ConfigMode::Interface(interface),
                format!("ip access-group {acl} {direction}"),
            )])
        }
        "router.nat.static" => {
            let local = ipv4(req, "local")?;
            let global = ipv4(req, "global")?;
            if local == global {
                return Err("Static NAT addresses must differ".into());
            }
            Ok(vec![at(
                ConfigMode::Global,
                format!("ip nat inside source static {local} {global}"),
            )])
        }
        "router.nat.interface" => {
            let interface = iface(req)?;
            let role = req.required("role")?;
            if !matches!(role, "inside" | "outside") {
                return Err("NAT role must be inside or outside".into());
            }
            Ok(vec![at(
                ConfigMode::Interface(interface),
                format!("ip nat {role}"),
            )])
        }
        "router.dhcp.pool" => {
            let pool = req.required("pool")?;
            if !name(pool) {
                return Err("Invalid DHCP pool name".into());
            }
            let net = network(req, "network")?;
            let last = Ipv4Addr::from(u32::from(net.network) | !u32::from(net.subnet_mask));
            if net.prefix > 30 || !client_range(net.network, last) {
                return Err("DHCP requires a usable unicast client subnet".into());
            }
            let mode = ConfigMode::DhcpPool(pool.into());
            let mut commands = vec![at(
                mode.clone(),
                format!("network {}", net.network_and_mask()),
            )];
            if !req.get("default_router").is_empty() {
                let router = ipv4(req, "default_router")?;
                let host = u32::from(router) & !u32::from(net.subnet_mask);
                if u32::from(router) & u32::from(net.subnet_mask) != u32::from(net.network)
                    || host == 0
                    || host == !u32::from(net.subnet_mask)
                {
                    return Err(
                        "DHCP default router must be a usable address inside pool subnet".into(),
                    );
                }
                commands.push(at(mode.clone(), format!("default-router {router}")));
            }
            if !req.get("dns_server").is_empty() {
                commands.push(at(mode, format!("dns-server {}", ipv4(req, "dns_server")?)));
            }
            Ok(commands)
        }
        "router.dhcp.exclude" => {
            let start = ipv4(req, "start")?;
            let text = if req.get("end").is_empty() {
                format!("ip dhcp excluded-address {start}")
            } else {
                let end = ipv4(req, "end")?;
                if u32::from(end) < u32::from(start) || !client_range(start, end) {
                    return Err("DHCP exclusion range is reversed or spans reserved space".into());
                }
                format!("ip dhcp excluded-address {start} {end}")
            };
            Ok(vec![at(ConfigMode::Global, text)])
        }
        _ => Err("Unknown services family".into()),
    }
}
