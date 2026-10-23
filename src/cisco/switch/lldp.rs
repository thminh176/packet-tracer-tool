//! Documented Catalyst LLDP subset; no operational state or defaults are inferred.
use super::{choice, interface, mode, number, spec};
use crate::cisco::{CommandRequest, CommandSpec, ConfigMode, DeviceProfile, ModeCommand, at};

const SOURCE: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/m_1522e_int_lldp_2960s_cg.html";

fn boolean(name: &'static str, en: &'static str, vi: &'static str) -> crate::cisco::ParameterSpec {
    choice(name, &["true", "false"], en, vi, "true")
}

pub(super) fn specs() -> Vec<CommandSpec> {
    let mut result = vec![
        spec(
            "switch.lldp.global",
            "Set global LLDP state",
            "Đặt trạng thái LLDP toàn cục",
            "[no] lldp run",
            "global",
            SOURCE,
            vec![boolean(
                "enabled",
                "Requested global LLDP state",
                "Trạng thái LLDP toàn cục cần đặt",
            )],
            vec!["show lldp", "show running-config"],
            vec![
                "Identified Catalyst 2960 or 3560 with documented IOS; global state is separate from interface transmit/receive",
            ],
        ),
        spec(
            "switch.lldp.interface",
            "Set port LLDP transmit and receive",
            "Đặt truyền và nhận LLDP trên cổng",
            "[no] lldp transmit; [no] lldp receive",
            "interface",
            SOURCE,
            vec![
                interface(),
                mode(),
                boolean(
                    "global_enabled",
                    "Confirmed current global LLDP state",
                    "Trạng thái LLDP toàn cục hiện tại đã xác nhận",
                ),
                boolean(
                    "transmit",
                    "Requested LLDP transmit state",
                    "Trạng thái truyền LLDP cần đặt",
                ),
                boolean(
                    "receive",
                    "Requested LLDP receive state",
                    "Trạng thái nhận LLDP cần đặt",
                ),
            ],
            vec![
                "show lldp",
                "show lldp interface <interface>",
                "show running-config interface <interface>",
            ],
            vec![
                "Identified Catalyst 2960 or 3560; confirmed physical static access/trunk port, excluding tunnel ports; enabling either direction requires explicit global_enabled=true",
            ],
        ),
        spec(
            "switch.lldp.timers",
            "Set LLDP timers in seconds",
            "Đặt bộ định thời LLDP theo giây",
            "lldp holdtime <holdtime>; lldp reinit <reinit>; lldp timer <timer>",
            "global",
            SOURCE,
            vec![
                number(
                    "holdtime",
                    0,
                    65535,
                    "Advertised neighbor retention time in seconds (default 120)",
                    "Thời gian lưu thông tin được quảng bá, giây (mặc định 120)",
                    "120",
                ),
                number(
                    "reinit",
                    2,
                    5,
                    "Port LLDP initialization delay in seconds (default 2)",
                    "Độ trễ khởi tạo LLDP trên cổng, giây (mặc định 2)",
                    "2",
                ),
                number(
                    "timer",
                    5,
                    65534,
                    "LLDP update interval in seconds (default 30)",
                    "Chu kỳ cập nhật LLDP, giây (mặc định 30)",
                    "30",
                ),
            ],
            vec!["show lldp", "show running-config"],
            vec![
                "Identified Catalyst 2960 or 3560 with documented IOS; timer configuration does not enable LLDP globally or on a port",
            ],
        ),
    ];
    // Typed generation is validated here; saved-configuration comparison lives
    // in the shared validator and is not implemented by these catalog entries.
    for spec in &mut result {
        spec.semantic_validation = false;
    }
    result
}

pub(super) fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    if !matches!(
        req.profile,
        DeviceProfile::Catalyst2960 | DeviceProfile::Catalyst3560
    ) {
        return Err(
            "LLDP requires an identified Catalyst2960 or Catalyst3560 and compatible IOS".into(),
        );
    }
    // The parent switch handler has already validated every typed field, raw
    // whitespace, unknown key and required input before entering this module.
    match req.family.as_str() {
        "switch.lldp.global" => Ok(vec![at(
            ConfigMode::Global,
            if req.get("enabled") == "true" {
                "lldp run"
            } else {
                "no lldp run"
            },
        )]),
        "switch.lldp.interface" => {
            if !super::physical_port(req.get("interface")) {
                return Err("LLDP interface workflow requires a physical Ethernet port".into());
            }
            if req.get("global_enabled") != "true"
                && (req.get("transmit") == "true" || req.get("receive") == "true")
            {
                return Err("Enabling interface LLDP requires explicit global_enabled=true; configure lldp run separately".into());
            }
            let mode = ConfigMode::Interface(req.get("interface").into());
            Ok(["transmit", "receive"]
                .iter()
                .map(|direction| {
                    at(
                        mode.clone(),
                        format!(
                            "{}lldp {direction}",
                            if req.get(direction) == "true" {
                                ""
                            } else {
                                "no "
                            }
                        ),
                    )
                })
                .collect())
        }
        "switch.lldp.timers" => Ok(["holdtime", "reinit", "timer"]
            .iter()
            .map(|key| at(ConfigMode::Global, format!("lldp {key} {}", req.get(key))))
            .collect()),
        _ => Err("Unknown LLDP family".into()),
    }
}
