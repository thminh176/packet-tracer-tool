//! CDP context is explicit; enabling an interface never silently enables the device.
use super::*;
const SOURCE: &str =
    "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/cdp/command/cdp-cr-book/cdp-cr-a1.html";
pub fn specs() -> Vec<CommandSpec> {
    let mut global = CommandSpec {
        id: "basic.cdp_global",
        name_en: "Global Cisco Discovery Protocol",
        name_vi: "Cisco Discovery Protocol toàn cục",
        topic: "Basics and security",
        syntax: "[no] cdp run",
        mode: "global",
        capability: Capability::Basic,
        parameters: vec![field(
            "enabled",
            ParameterKind::Choice(&["true", "false"]),
            "true",
        )],
        prerequisites: vec![
            "Review disclosure of neighbor/device information on untrusted links; disabling CDP can affect ODR and discovery-dependent features",
        ],
        verification: vec!["show cdp", "show cdp neighbors", "show running-config"],
        source: SOURCE,
        software: "Classic IOS CDP command reference; device/image support must be checked; PT execution untested",
        removal: Some("no cdp run"),
        semantic_validation: false,
    };
    global.parameters[0].description_en = "Explicit CDP administrative state";
    global.parameters[0].description_vi = "Trạng thái quản trị CDP rõ ràng";
    let mut interface = global.clone();
    interface.id = "basic.cdp_interface";
    interface.name_en = "Physical Ethernet interface CDP";
    interface.name_vi = "CDP trên cổng Ethernet vật lý";
    interface.syntax = "interface <interface>; [no] cdp enable";
    interface.mode = "interface";
    interface.removal = Some("no cdp enable");
    interface.parameters.extend([
        ParameterSpec {
            name: "interface",
            kind: ParameterKind::Interface,
            required: true,
            description_en: "Explicit physical Ethernet interface",
            description_vi: "Cổng Ethernet vật lý rõ ràng",
            example: "GigabitEthernet0/1",
        },
        ParameterSpec {
            name: "global_enabled",
            kind: ParameterKind::Choice(&["yes"]),
            required: true,
            description_en: "CDP already enabled globally",
            description_vi: "Đã xác nhận CDP bật toàn cục",
            example: "yes",
        },
    ]);
    interface.prerequisites = vec![
        "This bounded interface workflow requires CDP already enabled globally; it does not emit cdp run",
        "Physical Ethernet subset only; no SVI, loopback, subinterface, serial or port-channel support is claimed here",
    ];
    interface.verification = vec![
        "show cdp interface",
        "show cdp neighbors",
        "show running-config",
    ];
    vec![global, interface]
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    let prefix = match req.required("enabled")? {
        "true" => "",
        "false" => "no ",
        _ => return Err("Explicit CDP true/false state required".into()),
    };
    match req.family.as_str() {
        "basic.cdp_global" => Ok(vec![at(ConfigMode::Global, format!("{prefix}cdp run"))]),
        "basic.cdp_interface" => {
            if req.required("global_enabled")? != "yes" {
                return Err("CDP must already be enabled globally".into());
            }
            let interface = req.required("interface")?;
            let canonical =
                parser::interface_name(interface).ok_or("Invalid interface identity")?;
            let lower = canonical.to_ascii_lowercase();
            if lower.contains('.')
                || ![
                    "ethernet",
                    "fastethernet",
                    "gigabitethernet",
                    "tengigabitethernet",
                ]
                .iter()
                .any(|p| lower.starts_with(p))
            {
                return Err("CDP workflow requires a physical Ethernet interface".into());
            }
            Ok(vec![at(
                ConfigMode::Interface(canonical),
                format!("{prefix}cdp enable"),
            )])
        }
        _ => Err("Unknown discovery family".into()),
    }
}
