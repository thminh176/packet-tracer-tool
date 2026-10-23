//! Narrow WAN and DHCP relay workflows. Hardware assertions are explicit user inputs.
use super::*;
const CLOCK: &str =
    "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/interface/command/ir-cr-book/ir-c2.html";
const SERIAL: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/interface/configuration/15-mt/ir-15-mt-book/ir-cfg-ser-if.html";
const RELAY: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/ipaddr_dhcp/configuration/15-mt/dhcp-15-mt-book/config-dhcp-relay-agent.html";
pub fn specs() -> Vec<CommandSpec> {
    let module = || {
        param(
            "module_present",
            ParameterKind::Choice(&["yes"]),
            true,
            "Serial module verified present",
            "Đã xác nhận có mô-đun Serial",
            "yes",
        )
    };
    let serial_interface = || {
        param(
            "interface",
            ParameterKind::Interface,
            true,
            "Physical synchronous serial interface",
            "Cổng Serial đồng bộ vật lý",
            "Serial0/0/0",
        )
    };
    vec![
        spec(
            "router.dhcp.relay",
            "DHCP relay destination",
            "Địa chỉ chuyển tiếp DHCP",
            "ip helper-address <server>",
            "interface",
            Capability::Routing,
            vec![
                interface_param(),
                param(
                    "server",
                    ParameterKind::Ipv4,
                    true,
                    "Explicit reachable DHCP server",
                    "Địa chỉ máy chủ DHCP rõ ràng",
                    "192.0.2.10",
                ),
            ],
            RELAY,
            "Classic IOS 15M&T; router-only subset; PT untested",
            vec![
                "Apply on the client-facing routed interface; verify server reachability and return routing",
                "ip helper-address also forwards other default UDP broadcasts; review ip forward-protocol and security policy; no VRF/Option 82 changes are inferred",
            ],
            vec!["show ip interface", "show running-config"],
        ),
        spec(
            "router.serial.clock",
            "DCE serial clock",
            "Tốc độ clock cổng Serial DCE",
            "clock rate <bps>",
            "interface",
            Capability::Serial,
            vec![
                serial_interface(),
                module(),
                param(
                    "cable_role",
                    ParameterKind::Choice(&["dce"]),
                    true,
                    "Verified DCE end of the cable",
                    "Đã xác nhận đầu cáp DCE",
                    "dce",
                ),
                param(
                    "bps",
                    ParameterKind::Choice(&[
                        "1200", "2400", "4800", "9600", "19200", "38400", "56000", "64000",
                    ]),
                    true,
                    "Conservative standard clock rate in bits/sec",
                    "Tốc độ clock tiêu chuẩn, bit/giây",
                    "64000",
                ),
            ],
            CLOCK,
            "Classic IOS serial reference; installed hardware/cable dependent; PT untested",
            vec![
                "Confirm DCE cable role and serial module with show controllers serial; this subset does not infer hardware from a model name",
                "Hardware may round clock rates; only a conservative standard subset is offered; changing timing can interrupt the link",
            ],
            vec![
                "show controllers serial",
                "show interfaces serial",
                "show running-config",
            ],
        ),
        spec(
            "router.serial.ppp",
            "Synchronous serial PPP",
            "PPP trên cổng Serial đồng bộ",
            "encapsulation ppp",
            "interface",
            Capability::Serial,
            vec![
                serial_interface(),
                module(),
                param(
                    "peer_encapsulation",
                    ParameterKind::Choice(&["ppp"]),
                    true,
                    "Peer explicitly uses PPP",
                    "Đầu bên kia được xác nhận dùng PPP",
                    "ppp",
                ),
            ],
            SERIAL,
            "Classic IOS 15M&T; synchronous serial subset; PT untested",
            vec![
                "Both ends must use PPP and the physical serial module must exist; changing encapsulation can interrupt traffic",
                "No CHAP/PAP credentials, authentication policy, clock rate or no shutdown is invented",
            ],
            vec!["show interfaces serial", "show running-config"],
        ),
    ]
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    router_device(req)?;
    if req.family == "router.dhcp.relay" {
        let interface = iface(req)?;
        if interface.to_ascii_lowercase().starts_with("lo") {
            return Err(
                "This relay subset requires a client-facing interface, not a loopback".into(),
            );
        }
        return Ok(vec![at(
            ConfigMode::Interface(interface),
            format!("ip helper-address {}", ipv4(req, "server")?),
        )]);
    }
    // ISR serial validation needs module-specific evidence beyond this classic-IOS
    // workflow. Keep it blocked instead of claiming the model has a serial port.
    if req.profile == DeviceProfile::Isr4321 {
        return Err(
            "ISR 4321 serial NIM/profile support is not established by this workflow".into(),
        );
    }
    let interface = iface(req)?;
    let canonical =
        crate::cisco::parser::interface_name(&interface).ok_or("Invalid serial identity")?;
    if !canonical.to_ascii_lowercase().starts_with("serial") || canonical.contains('.') {
        return Err("Physical synchronous Serial interface required".into());
    }
    if req.required("module_present")? != "yes" {
        return Err("Verify the installed serial module first".into());
    }
    let text = match req.family.as_str() {
        "router.serial.clock" => {
            if req.required("cable_role")? != "dce" {
                return Err("Clock rate requires a verified DCE cable end".into());
            }
            let rate = req.required("bps")?;
            if ![
                "1200", "2400", "4800", "9600", "19200", "38400", "56000", "64000",
            ]
            .contains(&rate)
            {
                return Err("Clock rate is outside the documented conservative subset".into());
            }
            format!("clock rate {rate}")
        }
        "router.serial.ppp" => {
            if req.required("peer_encapsulation")? != "ppp" {
                return Err("Explicit peer PPP encapsulation required".into());
            }
            "encapsulation ppp".into()
        }
        _ => return Err("Unknown WAN family".into()),
    };
    Ok(vec![at(ConfigMode::Interface(interface), text)])
}
