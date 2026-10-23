//! Classic IPv4 RIP/EIGRP controls: no implicit network, AS or default policy.
use super::*;
const RIP: &str =
    "https://www.cisco.com/c/en/us/td/docs/ios/iproute_rip/command/reference/irr_book/irr_rip.html";
const PASSIVE: &str =
    "https://www.cisco.com/c/en/us/td/docs/ios/iproute_pi/command/reference/iri_book/iri_pi1.html";
const EIGRP: &str = "https://www.cisco.com/c/en/us/td/docs/ios/iproute_eigrp/command/reference/ire_book/ire_a1.html";
const EIGRP_PASSIVE: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/iproute_eigrp/command/ire-cr-book/ire-i1.html";
pub fn specs() -> Vec<CommandSpec> {
    let passive = || {
        param(
            "state",
            ParameterKind::Choice(&["passive", "active"]),
            true,
            "Explicit per-interface passive state",
            "Trạng thái passive rõ ràng của cổng",
            "passive",
        )
    };
    let as_number = || {
        param(
            "as_number",
            ParameterKind::Number(1, 65535),
            true,
            "Explicit classic IPv4 EIGRP AS",
            "AS EIGRP IPv4 cổ điển rõ ràng",
            "100",
        )
    };
    let mut result = Vec::new();
    for (id, en, vi, syntax, params, source, prereq, verify) in [
        (
            "router.rip.version",
            "Global RIP version",
            "Phiên bản RIP toàn cục",
            "router rip; version <version>",
            vec![param(
                "version",
                ParameterKind::Choice(&["1", "2"]),
                true,
                "Explicit RIP version",
                "Phiên bản RIP rõ ràng",
                "2",
            )],
            RIP,
            "Select the intended IPv4 RIP process; interface send/receive overrides remain separate. No network or auto-summary policy is inferred",
            "show ip protocols",
        ),
        (
            "router.rip.passive",
            "RIP passive interface",
            "Cổng passive RIP",
            "router rip; [no] passive-interface <interface>",
            vec![interface_param(), passive()],
            PASSIVE,
            "Confirm this interface participates in RIP; passive suppresses outgoing updates, not all incoming traffic. Explicit neighbors/unicast updates require separate review",
            "show ip protocols",
        ),
        (
            "router.rip.auto_summary",
            "RIP automatic summarization",
            "Tổng hợp mạng tự động RIP",
            "router rip; [no] auto-summary",
            vec![param(
                "enabled",
                ParameterKind::Choice(&["true", "false"]),
                true,
                "Explicit desired summarization policy",
                "Chính sách tổng hợp mạng rõ ràng",
                "false",
            )],
            RIP,
            "Review discontiguous subnets and intended classful-boundary behavior. No version or network changes are added",
            "show ip protocols",
        ),
        (
            "router.eigrp.passive",
            "Classic EIGRP passive interface",
            "Cổng passive EIGRP cổ điển",
            "router eigrp <as_number>; [no] passive-interface <interface>",
            vec![as_number(), interface_param(), passive()],
            EIGRP_PASSIVE,
            "Confirm the interface belongs to the specified classic IPv4 AS; passive suppresses Hellos and updates and affects adjacencies. Named/address-family EIGRP is outside this workflow",
            "show ip eigrp interfaces",
        ),
        (
            "router.eigrp.router_id",
            "Classic EIGRP router ID",
            "Router ID EIGRP cổ điển",
            "router eigrp <as_number>; eigrp router-id <router_id>",
            vec![
                as_number(),
                param(
                    "router_id",
                    ParameterKind::Ipv4,
                    true,
                    "Unique dotted 32-bit ID, excluding all-zero/all-ones",
                    "Định danh 32 bit duy nhất, không toàn 0 hoặc toàn 1",
                    "192.0.2.9",
                ),
            ],
            EIGRP,
            "Verify ID uniqueness in the intended AS; this is an identifier, not an IP assignment. No process deletion or restart is generated",
            "show ip protocols",
        ),
    ] {
        result.push(spec(
            id,
            en,
            vi,
            syntax,
            "router configuration",
            Capability::DynamicRouting,
            params,
            source,
            "Classic IPv4 IOS command references; named EIGRP/IPv6/PT execution untested",
            vec![prereq],
            vec![verify, "show running-config"],
        ));
    }
    result
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    router_device(req)?;
    let process = if req.family.starts_with("router.eigrp.") {
        format!("eigrp {}", number(req, "as_number", 1, 65535)?)
    } else {
        "rip".into()
    };
    let text = match req.family.as_str() {
        "router.rip.version" => match req.required("version")? {
            v @ ("1" | "2") => format!("version {v}"),
            _ => return Err("Explicit RIP version 1 or 2 required".into()),
        },
        "router.rip.auto_summary" => match req.required("enabled")? {
            "true" => "auto-summary".into(),
            "false" => "no auto-summary".into(),
            _ => return Err("Explicit true/false summarization policy required".into()),
        },
        "router.rip.passive" | "router.eigrp.passive" => {
            let interface = iface(req)?;
            let prefix = match req.required("state")? {
                "passive" => "",
                "active" => "no ",
                _ => return Err("Explicit passive/active state required".into()),
            };
            format!("{prefix}passive-interface {interface}")
        }
        "router.eigrp.router_id" => {
            let id = req
                .required("router_id")?
                .parse::<std::net::Ipv4Addr>()
                .map_err(|_| "Invalid router ID")?;
            if id.is_unspecified() || id.is_broadcast() {
                return Err("EIGRP router ID cannot be all-zero or all-ones".into());
            }
            format!("eigrp router-id {id}")
        }
        _ => return Err("Unknown classic routing control".into()),
    };
    Ok(vec![at(ConfigMode::Router(process), text)])
}
