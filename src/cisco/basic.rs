//! Small shared IOS fundamentals; explicit parameters only, no key regeneration.
use super::*;
const SECURITY: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/sec_usr_cfg/configuration/15-mt/sec-usr-cfg-15-mt-book/sec-cfg-sec-4cli.html";
const SSH: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/sec_usr_ssh/configuration/15-mt/sec-usr-ssh-15-mt-book/sec-secure-shell-v2.html";
const BANNER: &str = "https://www.cisco.com/c/en/us/td/docs/ios/fundamentals/configuration/guide/TIPs_Conversion/cf_15_1s_book/cf_connections.html";
const FILES: &str = "https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/config-mgmt/configuration/15mt/config-mgmt-15-mt-book/cm-config-files.html";
fn field(name: &'static str, kind: ParameterKind, example: &'static str) -> ParameterSpec {
    ParameterSpec {
        name,
        kind,
        required: true,
        description_en: match name {
            "local_user_present" => "Local user already configured",
            "aaa_mode" => "Existing AAA mode",
            "ssh_enabled" => "SSH already enabled",
            "first" => "First VTY line",
            "last" => "Last VTY line",
            _ => name,
        },
        description_vi: match name {
            "hostname" => "Tên máy IOS",
            "secret" => "Mật khẩu bí mật",
            "username" => "Tên người dùng",
            "privilege" => "Mức đặc quyền",
            "message" => "Nội dung thông báo",
            "local_user_present" => "Đã có người dùng cục bộ",
            "aaa_mode" => "Chế độ AAA hiện tại",
            "minutes" => "Số phút",
            "seconds" => "Số giây",
            "first" => "Dòng VTY đầu",
            "last" => "Dòng VTY cuối",
            "ssh_enabled" => "SSH đã được bật",
            _ => name,
        },
        example,
    }
}
pub fn specs() -> Vec<CommandSpec> {
    use ParameterKind::*;
    let mut v = vec![];
    for (id, en, vi, syntax, mode, parameters, source, verify) in [
        (
            "basic.hostname",
            "Hostname",
            "Tên máy IOS",
            "hostname <hostname>",
            "global",
            vec![field("hostname", Identifier, "Lab-Device")],
            FILES,
            "show running-config",
        ),
        (
            "basic.enable_secret",
            "Privileged EXEC secret",
            "Mật khẩu đặc quyền",
            "enable secret 0 <secret>",
            "global",
            vec![field("secret", Identifier, "ExampleOnly123")],
            SECURITY,
            "show running-config",
        ),
        (
            "basic.password_encryption",
            "Password obfuscation",
            "Che mật khẩu dạng rõ",
            "service password-encryption",
            "global",
            vec![],
            SECURITY,
            "show running-config",
        ),
        (
            "basic.local_user",
            "Local login user",
            "Người dùng cục bộ",
            "username <username> privilege <privilege> secret 0 <secret>",
            "global",
            vec![
                field("username", Identifier, "labuser"),
                field("privilege", Number(0, 15), "1"),
                field("secret", Identifier, "ExampleOnly123"),
            ],
            SECURITY,
            "show running-config",
        ),
        (
            "basic.banner_motd",
            "MOTD banner",
            "Thông báo đăng nhập",
            "banner motd #<message>#",
            "global",
            vec![field("message", Text, "Authorized lab access only")],
            BANNER,
            "show running-config",
        ),
        (
            "basic.console",
            "Console local login and timeout",
            "Đăng nhập console và thời gian chờ",
            "line console 0; login local; exec-timeout <minutes> <seconds>",
            "line console 0",
            vec![
                field("local_user_present", Choice(&["yes"]), "yes"),
                field("aaa_mode", Choice(&["disabled"]), "disabled"),
                field("minutes", Number(1, 1440), "10"),
                field("seconds", Number(0, 59), "0"),
            ],
            SECURITY,
            "show running-config",
        ),
        (
            "basic.vty_ssh",
            "VTY SSH-only access",
            "VTY chỉ cho phép SSH",
            "line vty <first> <last>; login local; transport input ssh",
            "line vty",
            vec![
                field("first", Number(0, 15), "0"),
                field("last", Number(0, 15), "4"),
                field("local_user_present", Choice(&["yes"]), "yes"),
                field("ssh_enabled", Choice(&["yes"]), "yes"),
                field("aaa_mode", Choice(&["disabled"]), "disabled"),
            ],
            SSH,
            "show ip ssh",
        ),
    ] {
        v.push(CommandSpec{id,name_en:en,name_vi:vi,topic:"Basics and security",syntax,mode,capability:Capability::Basic,parameters,prerequisites:vec!["Review existing authentication and management access before changing configuration"],verification:vec![verify],source,software:"Classic IOS 15M&T / 15.1S; exact PT implementation untested",removal:None,semantic_validation:false});
    }
    v.push(CommandSpec{id:"basic.save",name_en:"Save startup configuration",name_vi:"Lưu cấu hình khởi động",topic:"Basics and security",syntax:"copy running-config startup-config",mode:"privileged EXEC",capability:Capability::Basic,parameters:vec![],prerequisites:vec!["Review running configuration before replacing startup configuration; device may prompt for destination confirmation"],verification:vec!["show startup-config"],source:FILES,software:"Classic IOS 15M&T; exact PT execution untested",removal:None,semantic_validation:false});
    v
}
pub fn generate(req: &CommandRequest) -> Result<Vec<ModeCommand>, String> {
    use ConfigMode::*;
    let result = match req.family.as_str() {
        "basic.save" => vec![at(Exec, "copy running-config startup-config")],
        "basic.hostname" => {
            let name = req.required("hostname")?;
            if name.len() > 63
                || !name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                || !name
                    .chars()
                    .last()
                    .is_some_and(|c| c.is_ascii_alphanumeric())
                || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return Err(
                    "Hostname must start with a letter and contain only letters, digits or hyphens"
                        .into(),
                );
            }
            vec![at(Global, format!("hostname {name}"))]
        }
        "basic.enable_secret" => vec![at(
            Global,
            // Explicit plaintext type prevents a numeric/keyword credential from
            // being parsed as IOS's encryption-type or level option.
            format!("enable secret 0 {}", credential(req.required("secret")?)?),
        )],
        "basic.password_encryption" => vec![at(Global, "service password-encryption")],
        "basic.local_user" => {
            let user = credential(req.required("username")?)?;
            let privilege = req
                .required("privilege")?
                .parse::<u8>()
                .map_err(|_| "Invalid privilege")?;
            if privilege > 15 {
                return Err("Invalid privilege".into());
            }
            vec![at(
                Global,
                format!(
                    "username {user} privilege {privilege} secret 0 {}",
                    credential(req.required("secret")?)?
                ),
            )]
        }
        "basic.banner_motd" => {
            let message = req.required("message")?;
            if !safe_text(message) || message.contains('#') || !message.is_ascii() {
                return Err("Banner must be one ASCII line without delimiter #".into());
            }
            vec![at(Global, format!("banner motd #{message}#"))]
        }
        "basic.console" => {
            if req.get("local_user_present") != "yes" || req.get("aaa_mode") != "disabled" {
                return Err(
                    "Console login requires verified local user and disabled AAA for this subset"
                        .into(),
                );
            }
            let minutes = req
                .get("minutes")
                .parse::<u16>()
                .map_err(|_| "Invalid minutes")?;
            let seconds = req
                .get("seconds")
                .parse::<u8>()
                .map_err(|_| "Invalid seconds")?;
            if !(1..=1440).contains(&minutes) || seconds > 59 {
                return Err("Invalid timeout".into());
            }
            vec![
                at(Line("console 0".into()), "login local"),
                at(
                    Line("console 0".into()),
                    format!("exec-timeout {minutes} {seconds}"),
                ),
            ]
        }
        "basic.vty_ssh" => {
            let first = req
                .get("first")
                .parse::<u8>()
                .map_err(|_| "Invalid VTY range")?;
            let last = req
                .get("last")
                .parse::<u8>()
                .map_err(|_| "Invalid VTY range")?;
            if first > last
                || last > 15
                || req.get("local_user_present") != "yes"
                || req.get("ssh_enabled") != "yes"
                || req.get("aaa_mode") != "disabled"
            {
                return Err(
                    "VTY range, existing SSH, local user and AAA state must be explicit and valid"
                        .into(),
                );
            }
            let mode = Line(format!("vty {first} {last}"));
            vec![
                at(mode.clone(), "login local"),
                at(mode, "transport input ssh"),
            ]
        }
        _ => return Err("Unknown shared IOS family".into()),
    };
    Ok(result)
}
fn credential(s: &str) -> Result<&str, String> {
    if !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
    {
        Ok(s)
    } else {
        Err("This subset requires a single ASCII credential token, maximum 64 characters".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plaintext_secret_tokens_are_not_ios_options() {
        for secret in ["5", "9", "level", "Example123"] {
            let req = CommandRequest {
                family: "basic.enable_secret".into(),
                device: "Lab".into(),
                parameters: [("secret".into(), secret.into())].into(),
                ..Default::default()
            };
            let plan = super::super::generate(&req);
            assert!(plan.executable());
            assert!(
                plan.script()
                    .contains(&format!("enable secret 0 {secret}\n"))
            );
        }
    }
    #[test]
    fn examples_and_injection_are_checked() {
        for spec in specs() {
            let mut req = CommandRequest {
                family: spec.id.into(),
                device: "Lab".into(),
                parameters: spec
                    .parameters
                    .iter()
                    .map(|p| (p.name.into(), p.example.into()))
                    .collect(),
                ..Default::default()
            };
            assert!(super::super::generate(&req).executable(), "{}", spec.id);
            if let Some(p) = spec.parameters.first() {
                req.parameters.insert(p.name.into(), "bad\nreload".into());
                assert!(!super::super::generate(&req).executable());
            }
        }
    }
    #[test]
    fn vty_does_not_invent_authentication_dependencies() {
        let r = CommandRequest {
            family: "basic.vty_ssh".into(),
            device: "Lab".into(),
            ..Default::default()
        };
        let p = super::super::generate(&r);
        assert_eq!(p.classification, Classification::MissingParameters);
        assert!(p.script().is_empty());
    }
    #[test]
    fn banner_delimiter_and_hostnames_are_bounded() {
        let req = CommandRequest {
            family: "basic.banner_motd".into(),
            device: "Lab".into(),
            parameters: [("message".into(), "bad#text".into())].into(),
            ..Default::default()
        };
        assert!(!super::super::generate(&req).executable());
    }
}
