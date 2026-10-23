//! Independent OSPFv2 controls; process identity is always explicit.
use super::*;
const REFERENCE: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/iproute_ospf/command/iro-cr-book/ospf-i1.html";
const COST: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/iproute_ospf/command/iro-cr-book/ospf-a1.html";
const PASSIVE: &str =
    "https://www.cisco.com/c/en/us/td/docs/ios/iproute_pi/command/reference/iri_book/iri_pi1.html";
pub fn specs() -> Vec<CommandSpec> {
    let pid = || {
        param(
            "process_id",
            ParameterKind::Number(1, 65534),
            true,
            "Explicit OSPFv2 process ID (65535 assessment sentinel excluded)",
            "Process ID OSPFv2 rõ ràng (loại sentinel 65535)",
            "20",
        )
    };
    vec![
        spec(
            "router.ospf.router_id",
            "OSPFv2 configured router ID",
            "Router ID cấu hình OSPFv2",
            "router ospf <process_id>; router-id <router_id>",
            "router ospf",
            Capability::DynamicRouting,
            vec![
                pid(),
                param(
                    "router_id",
                    ParameterKind::Ipv4,
                    true,
                    "Unique nonzero dotted 32-bit identifier, not an interface assignment",
                    "Định danh 32 bit dạng IPv4, khác 0 và duy nhất; không gán IP cổng",
                    "192.0.2.9",
                ),
            ],
            REFERENCE,
            "Classic IOS OSPFv2 command reference; PT execution untested",
            vec![
                "Verify ID uniqueness in this OSPF domain; it need not be assigned to an interface",
                "An active process may keep its effective ID until restart. No clear process, reload or disruptive activation command is generated",
            ],
            vec![
                "show ip ospf",
                "show ip ospf neighbor",
                "show running-config",
            ],
        ),
        spec(
            "router.ospf.passive",
            "OSPFv2 passive interface",
            "Cổng passive OSPFv2",
            "router ospf <process_id>; [no] passive-interface <interface>",
            "router ospf",
            Capability::DynamicRouting,
            vec![
                pid(),
                interface_param(),
                param(
                    "state",
                    ParameterKind::Choice(&["passive", "active"]),
                    true,
                    "Explicit per-interface OSPF passive state",
                    "Trạng thái passive OSPF rõ ràng cho cổng",
                    "passive",
                ),
            ],
            PASSIVE,
            "Classic IOS protocol-independent command reference; OSPFv2 router-only subset; PT untested",
            vec![
                "Verify the interface belongs to the intended process; changing passive state affects Hellos and adjacency formation",
                "Connected prefixes may remain advertised; this is not an ACL. No passive-interface default or network statements are inferred",
            ],
            vec![
                "show ip protocols",
                "show ip ospf interface",
                "show ip ospf neighbor",
            ],
        ),
        spec(
            "router.ospf.reference_bandwidth",
            "OSPFv2 reference bandwidth",
            "Băng thông tham chiếu OSPFv2",
            "router ospf <process_id>; auto-cost reference-bandwidth <mbps>",
            "router ospf",
            Capability::DynamicRouting,
            vec![
                pid(),
                param(
                    "mbps",
                    ParameterKind::Number(1, 4294967),
                    true,
                    "Reference bandwidth in Mbps (not physical line speed)",
                    "Băng thông tham chiếu Mbps (không đổi tốc độ đường truyền)",
                    "1000",
                ),
            ],
            COST,
            "Classic IOS OSPFv2 command reference; PT execution untested",
            vec![
                "Coordinate the reference value across the OSPF domain; changes can alter route selection",
                "Explicit interface cost can override computed cost. No interface bandwidth or peer configuration is changed",
            ],
            vec![
                "show ip ospf",
                "show ip ospf interface",
                "show running-config",
            ],
        ),
    ]
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    router_device(req)?;
    let process = number(req, "process_id", 1, 65534)?;
    let text = match req.family.as_str() {
        "router.ospf.router_id" => {
            let id = req
                .required("router_id")?
                .parse::<std::net::Ipv4Addr>()
                .map_err(|_| "Invalid dotted OSPF router ID")?;
            if id.is_unspecified() {
                return Err("OSPF router ID must be nonzero".into());
            }
            format!("router-id {id}")
        }
        "router.ospf.passive" => {
            let interface = iface(req)?;
            let prefix = match req.required("state")? {
                "passive" => "",
                "active" => "no ",
                _ => return Err("Explicit passive/active state required".into()),
            };
            format!("{prefix}passive-interface {interface}")
        }
        "router.ospf.reference_bandwidth" => format!(
            "auto-cost reference-bandwidth {}",
            number(req, "mbps", 1, 4294967)?
        ),
        _ => return Err("Unknown OSPF control family".into()),
    };
    Ok(vec![at(
        ConfigMode::Router(format!("ospf {process}")),
        text,
    )])
}
