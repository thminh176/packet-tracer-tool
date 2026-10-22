//! Bounded, documented Catalyst commands. Context is caller evidence, not inferred state.
mod handlers;
use super::{Capability, CommandSpec, ParameterKind as K, ParameterSpec};
pub use handlers::generate;

const VLAN: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swvlan.html";
const TRUNK: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_01101.html";
const VOICE: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swvoip.html";
const STP: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_010000.html";
const GUARD: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swstpopt.html";
const SECURITY: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/m_1522e_sec_port_based_traffic_ctrl_2960_cg.html";
const CHANNEL: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_0100101.html";

fn p(
    name: &'static str,
    kind: K,
    required: bool,
    en: &'static str,
    vi: &'static str,
    example: &'static str,
) -> ParameterSpec {
    ParameterSpec {
        name,
        kind,
        required,
        description_en: en,
        description_vi: vi,
        example,
    }
}
fn interface() -> ParameterSpec {
    p(
        "interface",
        K::Interface,
        true,
        "Physical Ethernet port",
        "Cổng Ethernet vật lý",
        "GigabitEthernet0/1",
    )
}
fn mode() -> ParameterSpec {
    choice(
        "interface_mode",
        &["access", "trunk"],
        "Confirmed current static Layer 2 mode",
        "Chế độ lớp 2 tĩnh đã xác nhận",
        "access",
    )
}
fn vlans() -> ParameterSpec {
    p(
        "vlans",
        K::VlanList,
        true,
        "VLAN IDs, comma lists or ascending ranges",
        "Số VLAN, danh sách hoặc dải tăng dần",
        "10,20-22",
    )
}
fn vlan() -> ParameterSpec {
    number("vlan", 1, 4094, "VLAN identifier", "Mã VLAN", "10")
}
fn choice(
    name: &'static str,
    values: &'static [&'static str],
    en: &'static str,
    vi: &'static str,
    example: &'static str,
) -> ParameterSpec {
    p(name, K::Choice(values), true, en, vi, example)
}
fn number(
    name: &'static str,
    lo: u32,
    hi: u32,
    en: &'static str,
    vi: &'static str,
    example: &'static str,
) -> ParameterSpec {
    p(name, K::Number(lo, hi), true, en, vi, example)
}
// Keep each catalog entry self-contained: identity, bilingual labels, syntax,
// source, typed inputs, verification and prerequisites are reviewed together.
#[allow(clippy::too_many_arguments)]
fn spec(
    id: &'static str,
    en: &'static str,
    vi: &'static str,
    syntax: &'static str,
    mode: &'static str,
    source: &'static str,
    parameters: Vec<ParameterSpec>,
    verification: Vec<&'static str>,
    prerequisites: Vec<&'static str>,
) -> CommandSpec {
    CommandSpec {
        id,
        name_en: en,
        name_vi: vi,
        topic: "Switching",
        syntax,
        mode,
        capability: Capability::Switching,
        parameters,
        prerequisites,
        verification,
        source,
        software: "Catalyst 2960 IOS 15.2(2)E / Catalyst 3560 IOS 15.0(1)SE; documented, not PT-tested",
        removal: None,
        semantic_validation: true,
    }
}
pub fn specs() -> Vec<CommandSpec> {
    let ports = vec![
        "show interfaces <interface> switchport",
        "show running-config interface <interface>",
    ];
    let trunk_ports = vec![
        "show interfaces trunk",
        "show interfaces <interface> switchport",
        "show running-config interface <interface>",
    ];
    let stp = vec!["show spanning-tree", "show running-config"];
    let mut result = vec![
        spec(
            "switch.vlan",
            "Create and name a normal VLAN",
            "Tạo và đặt tên VLAN thông thường",
            "vlan <vlan>; name <name>",
            "global / config-vlan",
            VLAN,
            vec![
                number(
                    "vlan",
                    2,
                    1001,
                    "Writable normal-range VLAN",
                    "VLAN dải thường có thể tạo",
                    "10",
                ),
                p(
                    "name",
                    K::Identifier,
                    true,
                    "Safe VLAN name, up to 32 ASCII characters",
                    "Tên VLAN an toàn, tối đa 32 ký tự ASCII",
                    "SALES",
                ),
                choice(
                    "vtp_mode",
                    &["server", "transparent"],
                    "VTP mode permitting local creation",
                    "Chế độ VTP cho phép tạo VLAN",
                    "transparent",
                ),
                choice(
                    "vtp_version",
                    &["1", "2"],
                    "Confirmed VTP version; version 3 primary role excluded",
                    "Phiên bản VTP đã xác nhận; không gồm vai trò primary phiên bản 3",
                    "2",
                ),
            ],
            vec!["show vlan brief"],
            vec!["VTP server or transparent; excludes VLAN 1 and reserved VLANs 1002-1005"],
        ),
        spec(
            "switch.port_mode",
            "Set a static access or trunk port",
            "Đặt cổng access hoặc trunk tĩnh",
            "switchport mode <mode>",
            "interface",
            VLAN,
            vec![
                interface(),
                choice(
                    "mode",
                    &["access", "trunk"],
                    "Requested static Layer 2 mode",
                    "Chế độ lớp 2 tĩnh cần đặt",
                    "access",
                ),
                choice(
                    "current_mode",
                    &["access", "trunk", "dynamic_auto", "dynamic_desirable"],
                    "Confirmed current Layer 2 mode",
                    "Chế độ lớp 2 hiện tại đã xác nhận",
                    "dynamic_auto",
                ),
                choice(
                    "port_security",
                    &["disabled"],
                    "Security disabled before mode change",
                    "Tắt bảo mật trước khi đổi chế độ",
                    "disabled",
                ),
                number(
                    "current_group",
                    0,
                    6,
                    "Existing group; zero means none",
                    "Nhóm hiện tại; 0 là chưa có",
                    "0",
                ),
            ],
            ports.clone(),
            vec![
                "Physical Layer 2 port outside EtherChannel; trunk setup needs known Catalyst encapsulation profile",
            ],
        ),
        spec(
            "switch.access_vlan",
            "Assign an access VLAN",
            "Gán VLAN dữ liệu cho cổng access",
            "switchport access vlan <vlan>",
            "interface",
            VLAN,
            vec![
                interface(),
                mode(),
                vlan(),
                choice(
                    "vlan_active",
                    &["yes"],
                    "VLAN exists and is active",
                    "VLAN tồn tại và hoạt động",
                    "yes",
                ),
            ],
            ports.clone(),
            vec!["Confirmed static access port; VLAN exists and is active"],
        ),
        spec(
            "switch.voice_vlan",
            "Assign a phone voice VLAN",
            "Gán VLAN thoại cho điện thoại IP",
            "switchport voice vlan <vlan>",
            "interface",
            VOICE,
            vec![
                interface(),
                mode(),
                vlan(),
                choice(
                    "vlan_active",
                    &["yes"],
                    "Voice VLAN exists and is active",
                    "VLAN thoại tồn tại và hoạt động",
                    "yes",
                ),
                choice(
                    "cdp_enabled",
                    &["yes"],
                    "CDP enabled globally and on port",
                    "CDP đã bật toàn cục và trên cổng",
                    "yes",
                ),
                choice(
                    "port_security",
                    &["disabled"],
                    "Security disabled in bounded voice subset",
                    "Tắt bảo mật trong tập con thoại",
                    "disabled",
                ),
                choice(
                    "endpoint",
                    &["phone"],
                    "Confirmed phone endpoint",
                    "Đầu cuối điện thoại đã xác nhận",
                    "phone",
                ),
            ],
            ports.clone(),
            vec![
                "Access port, active VLAN and CDP; IOS also enables PortFast; QoS is configured separately",
            ],
        ),
        spec(
            "switch.trunk_native",
            "Set the untagged trunk VLAN",
            "Đặt VLAN không gắn thẻ trên trunk",
            "switchport trunk native vlan <vlan>",
            "interface",
            TRUNK,
            vec![
                interface(),
                mode(),
                vlan(),
                number(
                    "peer_native_vlan",
                    1,
                    4094,
                    "Confirmed peer native VLAN",
                    "VLAN native phía đối diện",
                    "10",
                ),
            ],
            trunk_ports.clone(),
            vec!["Confirmed trunk; native VLANs at both ends must match"],
        ),
        spec(
            "switch.trunk_allowed",
            "Change trunk VLAN membership",
            "Đổi danh sách VLAN được phép trên trunk",
            "switchport trunk allowed vlan [add|remove|except] <vlans> | all | none",
            "interface",
            TRUNK,
            vec![
                interface(),
                mode(),
                choice(
                    "operation",
                    &["replace", "add", "remove", "except", "all", "none"],
                    "Allowed-list operation",
                    "Thao tác danh sách VLAN",
                    "replace",
                ),
                p(
                    "vlans",
                    K::VlanList,
                    false,
                    "List required except for all or none",
                    "Cần danh sách trừ all hoặc none",
                    "10,20-22",
                ),
            ],
            trunk_ports,
            vec!["Confirmed trunk; empty, descending and overlapping ranges are rejected"],
        ),
        spec(
            "switch.stp_mode",
            "Select spanning-tree protocol",
            "Chọn giao thức cây bao trùm",
            "spanning-tree mode <mode>",
            "global",
            STP,
            vec![choice(
                "mode",
                &["pvst", "rapid-pvst", "mst"],
                "Spanning-tree protocol",
                "Giao thức cây bao trùm",
                "rapid-pvst",
            )],
            stp.clone(),
            vec!["MST region and instance mapping configured separately"],
        ),
        spec(
            "switch.stp_root",
            "Set primary or backup VLAN root",
            "Đặt cầu gốc chính hoặc dự phòng",
            "spanning-tree vlan <vlans> root <role>",
            "global",
            STP,
            vec![
                vlans(),
                choice(
                    "role",
                    &["primary", "secondary"],
                    "Root election role",
                    "Vai trò bầu cầu gốc",
                    "primary",
                ),
                choice(
                    "stp_mode",
                    &["pvst", "rapid-pvst"],
                    "Confirmed per-VLAN STP mode",
                    "Chế độ STP theo VLAN đã xác nhận",
                    "pvst",
                ),
            ],
            stp.clone(),
            vec!["PVST or rapid PVST; MST instance commands excluded"],
        ),
        spec(
            "switch.stp_priority",
            "Set VLAN bridge priority",
            "Đặt độ ưu tiên cầu theo VLAN",
            "spanning-tree vlan <vlans> priority <priority>",
            "global",
            STP,
            vec![
                vlans(),
                number(
                    "priority",
                    0,
                    61440,
                    "Bridge priority in steps of 4096",
                    "Độ ưu tiên cầu theo bước 4096",
                    "8192",
                ),
                choice(
                    "stp_mode",
                    &["pvst", "rapid-pvst"],
                    "Confirmed per-VLAN STP mode",
                    "Chế độ STP theo VLAN đã xác nhận",
                    "pvst",
                ),
            ],
            stp.clone(),
            vec!["Priority is a multiple of 4096"],
        ),
        spec(
            "switch.stp_portfast",
            "Enable PortFast for an end station",
            "Bật PortFast cho trạm cuối",
            "spanning-tree portfast",
            "interface",
            GUARD,
            vec![
                interface(),
                mode(),
                choice(
                    "endpoint",
                    &["end_station"],
                    "Confirmed single end station",
                    "Đã xác nhận một trạm cuối",
                    "end_station",
                ),
            ],
            stp.clone(),
            vec!["Access port with a single end station; trunk PortFast excluded"],
        ),
        spec(
            "switch.stp_bpduguard",
            "Enable per-port BPDU guard",
            "Bật bảo vệ BPDU trên cổng",
            "spanning-tree bpduguard enable",
            "interface",
            GUARD,
            vec![
                interface(),
                mode(),
                choice(
                    "portfast",
                    &["enabled"],
                    "Confirmed PortFast state",
                    "Trạng thái PortFast đã xác nhận",
                    "enabled",
                ),
            ],
            vec![
                "show spanning-tree",
                "show interfaces status err-disabled",
                "show running-config",
            ],
            vec!["Bounded subset requires a PortFast access port"],
        ),
        spec(
            "switch.stp_rootguard",
            "Prevent a downstream root election",
            "Ngăn phía dưới trở thành cầu gốc",
            "spanning-tree guard root",
            "interface",
            GUARD,
            vec![
                interface(),
                mode(),
                choice(
                    "loop_guard",
                    &["disabled"],
                    "Loop guard must be disabled",
                    "Phải tắt loop guard",
                    "disabled",
                ),
            ],
            vec![
                "show spanning-tree",
                "show spanning-tree inconsistentports",
                "show running-config",
            ],
            vec!["Root guard and loop guard are mutually exclusive"],
        ),
        spec(
            "switch.stp_cost",
            "Set port STP path cost",
            "Đặt chi phí đường đi STP của cổng",
            "spanning-tree cost <cost>",
            "interface",
            STP,
            vec![
                interface(),
                mode(),
                number(
                    "cost",
                    1,
                    200000000,
                    "Positive path cost",
                    "Chi phí đường đi dương",
                    "19",
                ),
            ],
            stp.clone(),
            vec!["Confirmed static Layer 2 interface"],
        ),
        spec(
            "switch.stp_port_priority",
            "Set STP port priority",
            "Đặt độ ưu tiên cổng STP",
            "spanning-tree port-priority <priority>",
            "interface",
            STP,
            vec![
                interface(),
                mode(),
                number(
                    "priority",
                    0,
                    240,
                    "Port priority in steps of 16",
                    "Độ ưu tiên cổng theo bước 16",
                    "128",
                ),
            ],
            stp,
            vec!["Priority is a multiple of 16"],
        ),
    ];
    result.extend(advanced_specs());
    result.extend(ip_specs());
    result
}

fn advanced_specs() -> Vec<CommandSpec> {
    vec![
        spec(
            "switch.port_security",
            "Configure bounded access-port security",
            "Cấu hình bảo mật giới hạn cho cổng access",
            "switchport port-security [maximum|mac-address|violation|aging]",
            "interface",
            SECURITY,
            vec![
                interface(),
                mode(),
                number(
                    "maximum",
                    1,
                    32,
                    "Secure MAC limit; subset cap 32",
                    "Giới hạn MAC; tập con tối đa 32",
                    "2",
                ),
                number(
                    "secure_count",
                    0,
                    32,
                    "Secure MAC count including requested static MAC",
                    "Số MAC gồm MAC tĩnh yêu cầu",
                    "0",
                ),
                number(
                    "available_capacity",
                    1,
                    32,
                    "Confirmed available total capacity including existing entries",
                    "Tổng dung lượng khả dụng gồm mục hiện có",
                    "32",
                ),
                choice(
                    "voice_vlan",
                    &["disabled"],
                    "Voice VLAN excluded in security subset",
                    "Không gồm VLAN thoại trong tập con bảo mật",
                    "disabled",
                ),
                choice(
                    "span_destination",
                    &["no"],
                    "Port is not a SPAN destination",
                    "Cổng không phải đích SPAN",
                    "no",
                ),
                number(
                    "current_group",
                    0,
                    6,
                    "Existing group, must be zero",
                    "Nhóm hiện tại, phải bằng 0",
                    "0",
                ),
                choice(
                    "violation",
                    &["protect", "restrict", "shutdown"],
                    "Action when unknown source exceeds limit",
                    "Hành động khi nguồn lạ vượt giới hạn",
                    "restrict",
                ),
                choice(
                    "sticky",
                    &["enabled", "disabled"],
                    "Requested sticky learning state",
                    "Trạng thái học MAC sticky cần đặt",
                    "enabled",
                ),
                p(
                    "static_mac",
                    K::Identifier,
                    false,
                    "Unicast MAC in xxxx.xxxx.xxxx form",
                    "MAC unicast dạng xxxx.xxxx.xxxx",
                    "0011.2233.4455",
                ),
                p(
                    "aging_minutes",
                    K::Number(0, 1440),
                    false,
                    "Aging timer; zero disables aging",
                    "Thời gian hết hạn; 0 để tắt",
                    "5",
                ),
                p(
                    "aging_type",
                    K::Choice(&["absolute", "inactivity"]),
                    false,
                    "Required with aging_minutes",
                    "Bắt buộc khi có aging_minutes",
                    "inactivity",
                ),
                p(
                    "aging_static",
                    K::Choice(&["enabled", "disabled"]),
                    false,
                    "Required with aging_minutes",
                    "Bắt buộc khi có aging_minutes",
                    "disabled",
                ),
            ],
            vec![
                "show port-security interface <interface>",
                "show port-security address",
                "show running-config interface <interface>",
            ],
            vec![
                "Static access port, no voice VLAN or EtherChannel, not a SPAN destination",
                "Maximum cannot be below secure count or above evidenced available capacity",
                "Sticky secure addresses do not age; nonzero aging requires disabled sticky",
            ],
        ),
        spec(
            "switch.etherchannel",
            "Join a consistent Layer 2 EtherChannel",
            "Tham gia EtherChannel lớp 2 đồng nhất",
            "channel-group <group> mode <mode>",
            "interface",
            CHANNEL,
            vec![
                interface(),
                mode(),
                number(
                    "group",
                    1,
                    6,
                    "Channel group; portable subset 1-6",
                    "Nhóm kênh; tập con 1-6",
                    "1",
                ),
                number(
                    "current_group",
                    0,
                    6,
                    "Existing group; zero or requested group",
                    "Nhóm hiện tại; 0 hoặc nhóm yêu cầu",
                    "0",
                ),
                choice(
                    "mode",
                    &["active", "passive", "auto", "desirable", "on"],
                    "Local LACP, PAgP or static mode",
                    "Chế độ LACP, PAgP hoặc tĩnh tại cổng",
                    "active",
                ),
                choice(
                    "peer_mode",
                    &["active", "passive", "auto", "desirable", "on"],
                    "Confirmed peer mode",
                    "Chế độ phía đối diện đã xác nhận",
                    "passive",
                ),
                choice(
                    "group_mode",
                    &["active", "passive", "auto", "desirable", "on"],
                    "Existing local group mode",
                    "Chế độ nhóm hiện tại trên switch",
                    "active",
                ),
                choice(
                    "group_interface_mode",
                    &["access", "trunk"],
                    "Confirmed local group Layer 2 mode",
                    "Chế độ lớp 2 của nhóm đã xác nhận",
                    "access",
                ),
                choice(
                    "group_port_type",
                    &["fastethernet", "gigabitethernet"],
                    "Confirmed physical port type shared by group members",
                    "Loại cổng vật lý chung của nhóm đã xác nhận",
                    "gigabitethernet",
                ),
                number(
                    "resulting_member_count",
                    1,
                    8,
                    "Confirmed total member count including this port; subset cap eight",
                    "Tổng số thành viên gồm cổng này; tập con tối đa tám",
                    "2",
                ),
                choice(
                    "span_destination",
                    &["no"],
                    "Member is not a SPAN destination",
                    "Thành viên không phải đích SPAN",
                    "no",
                ),
                choice(
                    "member_enabled",
                    &["yes"],
                    "Member port is administratively enabled",
                    "Cổng thành viên đã bật quản trị",
                    "yes",
                ),
                choice(
                    "group_stp_consistent",
                    &["yes"],
                    "Per-VLAN STP cost, priority and PortFast checked against group",
                    "Đã đối chiếu cost, priority STP theo VLAN và PortFast với nhóm",
                    "yes",
                ),
                choice(
                    "speed",
                    &["10", "100", "1000"],
                    "Confirmed member speed in Mbps",
                    "Tốc độ thành viên, Mbps",
                    "1000",
                ),
                choice(
                    "group_speed",
                    &["10", "100", "1000"],
                    "Confirmed group speed in Mbps",
                    "Tốc độ nhóm, Mbps",
                    "1000",
                ),
                choice(
                    "duplex",
                    &["full"],
                    "Member full duplex required",
                    "Thành viên phải full duplex",
                    "full",
                ),
                choice(
                    "group_duplex",
                    &["full"],
                    "Confirmed group duplex",
                    "Chế độ song công của nhóm",
                    "full",
                ),
                choice(
                    "port_security",
                    &["disabled"],
                    "Security disabled in channel subset",
                    "Tắt bảo mật trong tập con EtherChannel",
                    "disabled",
                ),
                p(
                    "access_vlan",
                    K::Number(1, 4094),
                    false,
                    "Required for access members",
                    "Bắt buộc cho thành viên access",
                    "10",
                ),
                p(
                    "group_access_vlan",
                    K::Number(1, 4094),
                    false,
                    "Group access VLAN",
                    "VLAN access của nhóm",
                    "10",
                ),
                p(
                    "native_vlan",
                    K::Number(1, 4094),
                    false,
                    "Required for trunk members",
                    "Bắt buộc cho thành viên trunk",
                    "1",
                ),
                p(
                    "group_native_vlan",
                    K::Number(1, 4094),
                    false,
                    "Group native VLAN",
                    "VLAN native của nhóm",
                    "1",
                ),
                p(
                    "allowed_vlans",
                    K::VlanList,
                    false,
                    "Explicit trunk allowed list",
                    "Danh sách VLAN trunk rõ ràng",
                    "1-4094",
                ),
                p(
                    "group_allowed_vlans",
                    K::VlanList,
                    false,
                    "Explicit group allowed list",
                    "Danh sách VLAN cho phép của nhóm",
                    "1-4094",
                ),
            ],
            vec![
                "show etherchannel summary",
                "show etherchannel <group> detail",
                "show interfaces <interface> switchport",
            ],
            vec![
                "Physical Layer 2 members with matching speed, duplex, VLAN and trunk settings",
                "Peer negotiation must form a channel; passive/passive and auto/auto cannot",
                "One member per request; context describes existing or explicitly planned group; routed channels excluded",
            ],
        ),
    ]
}

fn ip_specs() -> Vec<CommandSpec> {
    const ADDRESS: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swipaddr.html";
    const ROUTED: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swint.html";
    const ROUTING: &str = "https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swiprout.html";
    let ip = || {
        p(
            "address",
            K::Ipv4,
            true,
            "Unicast host IPv4 address",
            "Địa chỉ IPv4 unicast của thiết bị",
            "192.168.10.2",
        )
    };
    let mask = || {
        p(
            "mask",
            K::Ipv4,
            true,
            "Contiguous /1 through /30 subnet mask",
            "Mặt nạ liên tục từ /1 đến /30",
            "255.255.255.0",
        )
    };
    let image = || {
        choice(
            "image",
            &["ip_base", "ip_services"],
            "Confirmed installed 3560 software image",
            "Image phần mềm 3560 đã xác nhận",
            "ip_base",
        )
    };
    let mut routed = spec(
        "switch.routed_port",
        "Configure a 3560 routed physical port",
        "Cấu hình cổng vật lý định tuyến 3560",
        "no switchport; ip address <address> <mask>; no shutdown",
        "interface",
        ROUTED,
        vec![
            interface(),
            ip(),
            mask(),
            choice(
                "current_mode",
                &["access", "trunk", "routed"],
                "Confirmed current interface mode",
                "Chế độ cổng hiện tại đã xác nhận",
                "access",
            ),
            image(),
            number(
                "current_group",
                0,
                6,
                "Existing group; must be zero",
                "Nhóm hiện tại; phải bằng 0",
                "0",
            ),
            choice(
                "port_security",
                &["disabled"],
                "Security disabled before conversion",
                "Tắt bảo mật trước khi chuyển đổi",
                "disabled",
            ),
            choice(
                "conversion_approved",
                &["yes"],
                "Layer 2 configuration loss and link reset acknowledged",
                "Đã chấp nhận mất cấu hình lớp 2 và khởi tạo lại liên kết",
                "yes",
            ),
        ],
        vec![
            "show ip interface brief",
            "show interfaces <interface> switchport",
            "show running-config interface <interface>",
        ],
        vec![
            "Catalyst3560 with IP base or IP services; no EtherChannel",
            "no switchport can reset the link and discard previous Layer 2 configuration; IP routing configured separately",
        ],
    );
    routed.capability = Capability::Multilayer;
    let mut routing = spec(
        "switch.ip_routing",
        "Enable 3560 IPv4 forwarding",
        "Bật chuyển tiếp IPv4 trên 3560",
        "ip routing",
        "global",
        ROUTING,
        vec![image()],
        vec!["show ip route", "show running-config"],
        vec![
            "Catalyst3560 with IP base or IP services; addressed routed ports or SVIs needed for connected routes",
        ],
    );
    routing.capability = Capability::Multilayer;
    vec![
        spec(
            "switch.svi.ipv4",
            "Address and enable a VLAN interface",
            "Đặt địa chỉ và bật giao diện VLAN",
            "interface Vlan<vlan>; ip address <address> <mask>; no shutdown",
            "interface",
            ADDRESS,
            vec![
                p(
                    "interface",
                    K::Interface,
                    true,
                    "SVI matching the VLAN number",
                    "SVI trùng với mã VLAN",
                    "Vlan10",
                ),
                vlan(),
                ip(),
                mask(),
                choice(
                    "vlan_active",
                    &["yes"],
                    "VLAN exists and is active",
                    "VLAN tồn tại và hoạt động",
                    "yes",
                ),
                choice(
                    "forwarding_port",
                    &["yes"],
                    "At least one member port is up and STP forwarding",
                    "Có ít nhất một cổng thành viên up và STP forwarding",
                    "yes",
                ),
            ],
            vec![
                "show ip interface brief",
                "show interfaces vlan <vlan>",
                "show vlan brief",
            ],
            vec![
                "SVI number must match VLAN; active VLAN and an up forwarding member required for line protocol",
                "2960 management SVI does not enable inter-VLAN routing",
            ],
        ),
        spec(
            "switch.default_gateway",
            "Set a switch management gateway",
            "Đặt cổng mặc định quản trị switch",
            "ip default-gateway <gateway>",
            "global",
            ADDRESS,
            vec![
                p(
                    "gateway",
                    K::Ipv4,
                    true,
                    "Directly connected unicast next hop",
                    "Địa chỉ unicast của chặng kế tiếp trực tiếp",
                    "192.168.10.1",
                ),
                ip(),
                mask(),
                choice(
                    "ip_routing",
                    &["disabled"],
                    "Confirmed IP routing disabled",
                    "Đã xác nhận tắt định tuyến IP",
                    "disabled",
                ),
            ],
            vec!["show ip default-gateway", "show running-config"],
            vec![
                "For management when IP routing is disabled; gateway must be in the management subnet and differ from address",
            ],
        ),
        routed,
        routing,
    ]
}
