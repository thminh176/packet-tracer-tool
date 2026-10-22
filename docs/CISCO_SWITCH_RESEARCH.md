# Bounded Cisco switch command research

Researched independently against official Cisco documentation on 2026-10-22.
This is an offline command generator, not evidence of Packet Tracer execution.
No private PKA or original CC material was opened. No dependencies were added.

## Sources and implementation decisions

| Cisco source | Release / modes | Applied constraints |
| --- | --- | --- |
| [3560 VLAN configuration](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swvlan.html) | Catalyst3560 IOS 15.0(1)SE; global, config-vlan, interface | Create/name bounded to VLAN 2–1001 and a 32-character ASCII token. Require VTP server or transparent. Static access/trunk selection requires explicit current Layer 2 mode. 3560 trunk selection sets dot1q encapsulation first. |
| [2960 VLAN trunks](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_01101.html) | Catalyst2960 family IOS 15.2(2)E; interface | VLAN references 1–4094. Allowed lists support replace/add/remove/except and separate all/none operations. Native VLAN requires confirmed matching peer. |
| [3560 voice VLAN](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swvoip.html) | IOS 15.0(1)SE; interface | Access port, active VLAN, phone endpoint and CDP required. IOS also enables PortFast. QoS configuration is separate. Voice plus port security is excluded from this subset because capacity requirements depend on phone/data MAC use. |
| [2960 STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_010000.html) | IOS 15.2(2)E; global / interface | Modes pvst, rapid-pvst, mst. VLAN root/priority requires PVST or rapid PVST. Bridge priority 0–61440 by 4096; port priority 0–240 by 16; cost 1–200000000. MST region/instance commands excluded. |
| [3560 optional STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swstpopt.html) | IOS 15.0(1)SE; interface | PortFast limited to a confirmed single end station on access. BPDU guard limited to confirmed PortFast access ports. Root guard requires loop guard disabled. |
| [2960 port-based security](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/m_1522e_sec_port_based_traffic_ctrl_2960_cg.html) and [3560 counterpart](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swtrafc.html) | IOS 15.2(2)E / 15.0(1)SE; interface | Static access only in this subset; no voice, SPAN destination or EtherChannel. Secure limit deliberately capped at 32, additionally bounded by caller-evidenced available capacity and secure count. Supports sticky, one unicast static MAC, protect/restrict/shutdown, aging 0–1440 with absolute/inactivity and static-aging toggle. Sticky addresses cannot age. |
| [2960 EtherChannels](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_0100101.html) and [3560 EtherChannels](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swethchl.html) | IOS 15.2(2)E / 15.0(1)SE; physical interface | Supports LACP active/passive, PAgP auto/desirable, static on. Passive/passive, auto/auto, cross-protocol and unilateral static combinations rejected. Requires matching local group mode, speed, duplex and VLAN settings. Range deliberately bounded 1–6 although these guides document 24/48 respectively. |
| [2960 management addressing](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/m_1522e_assign_switch_ip_address_default_gateway_2960_cg.html) and [3560 addressing](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swipaddr.html) | IOS 15.2(2)E / 15.0(1)SE; SVI / global | SVI number must match VLAN; require active VLAN and forwarding member. Management gateway requires routing disabled and a different host in the same subnet. |
| [3560 interface characteristics](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swint.html) and [3560 unicast routing](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swiprout.html) | IOS 15.0(1)SE; interface / global | Routed ports require 3560 and confirmed IP base or IP services image. `no switchport` can reset the link and discard Layer 2 configuration; explicit conversion acknowledgment required. `ip routing` enables forwarding separately. |

Some 3560 15.0(2)SE chapter URLs redirected to the generic support page during
research; the linked 15.0(1)SE chapters above were accessible. The 2960 trunk
chapter says trunks cannot be secure while its security chapter permits static
trunks; this generator deliberately supports access-only port security. These
are bounded application policies, not claims that all excluded IOS features are
unsupported on real hardware.

## Family IDs and typed context

All inputs are declared `ParameterSpec` fields. None accepts free-form CLI.
Unknown fields, control characters, unsafe values, unknown enums, invalid numeric
ranges, malformed/overlapping VLAN lists, SVIs in physical-port commands, and
missing required or conditional context produce no executable script.

| Family | Parameters / context |
| --- | --- |
| `switch.vlan` | `vlan`, `name`, `vtp_mode`, `vtp_version=1/2` (VTP3 primary role excluded) |
| `switch.port_mode` | `interface`, requested `mode`, `current_mode`, `port_security=disabled`, `current_group=0` |
| `switch.access_vlan` | `interface`, `interface_mode=access`, `vlan`, `vlan_active=yes` |
| `switch.voice_vlan` | `interface`, `interface_mode=access`, `vlan`, `vlan_active=yes`, `cdp_enabled=yes`, `port_security=disabled`, `endpoint=phone` |
| `switch.trunk_native` | `interface`, `interface_mode=trunk`, `vlan`, matching `peer_native_vlan` |
| `switch.trunk_allowed` | `interface`, `interface_mode=trunk`, `operation`; `vlans` required except all/none, forbidden with those operations |
| `switch.stp_mode` | `mode=pvst/rapid-pvst/mst` |
| `switch.stp_root` | `vlans`, `role=primary/secondary`, `stp_mode=pvst/rapid-pvst` |
| `switch.stp_priority` | `vlans`, `priority`, `stp_mode=pvst/rapid-pvst` |
| `switch.stp_portfast` | `interface`, `interface_mode=access`, `endpoint=end_station` |
| `switch.stp_bpduguard` | `interface`, `interface_mode=access`, `portfast=enabled` |
| `switch.stp_rootguard` | `interface`, `interface_mode=access/trunk`, `loop_guard=disabled` |
| `switch.stp_cost` / `switch.stp_port_priority` | `interface`, `interface_mode=access/trunk`, `cost` / `priority` |
| `switch.port_security` | `interface`, `interface_mode=access`, `maximum`, `secure_count` (including requested static MAC), `available_capacity` (total including current entries), `voice_vlan=disabled`, `span_destination=no`, `current_group=0`, `violation`, `sticky`; optional `static_mac`; `aging_minutes` requires `aging_type` and `aging_static` |
| `switch.etherchannel` | `interface`, `interface_mode`, `group`, `current_group`, local `mode`, `peer_mode`, local `group_mode`, `group_interface_mode`, `group_port_type=fastethernet/gigabitethernet`, `resulting_member_count=1..8`, `span_destination=no`, `member_enabled=yes`, `group_stp_consistent=yes`, `speed`, `group_speed`, `duplex=full`, `group_duplex=full`, `port_security=disabled`; access requires `access_vlan/group_access_vlan`; trunk requires `native_vlan/group_native_vlan` and `allowed_vlans/group_allowed_vlans` |
| `switch.svi.ipv4` | `interface=VlanN`, matching `vlan`, `address`, `mask`, `vlan_active=yes`, `forwarding_port=yes` |
| `switch.default_gateway` | `gateway`, management `address`, `mask`, `ip_routing=disabled` |
| `switch.routed_port` | `interface`, `address`, `mask`, `current_mode=access/trunk/routed`, `image=ip_base/ip_services`, `current_group=0`, `port_security=disabled`, `conversion_approved=yes` |
| `switch.ip_routing` | `image=ip_base/ip_services` |

Context is supplied evidence, not live discovery. The mapper must not invent
access/trunk mode, endpoint identity, available MAC capacity, or EtherChannel
consistency. One EtherChannel member is configured per request; the caller must
provide the existing or explicitly planned group's settings. Equivalent explicit
VLAN lists compare by membership. Routed EtherChannels, interface ranges,
port-channel configuration, static voice MAC selectors, and sticky-static MAC
variants are excluded. Host addressing deliberately supports /1 through /30;
/31 and /32 need separate semantics.

EtherChannel additionally requires evidenced equal physical member type,
administratively enabled membership, no SPAN destination role, and checked
per-VLAN STP cost/priority and PortFast consistency. The local member total is
bounded to eight for every negotiation protocol in this implementation, excluding
LACP standby members. `group_stp_consistent=yes` is an explicit caller assertion
about the documented settings, not an assertion inferred from interface names.

## Coordinator integration and verification

Shared `src/cisco/mod.rs`, parser, profile support and UI remain coordinator-owned.
All Layer 2 and management specs use `Capability::Switching`; Layer 3 conversion
and forwarding use `Capability::Multilayer` plus a handler-level 3560 check.
Router profiles must never emit these commands. Unknown/generic profiles must
not receive profile-specific trunk encapsulation, SVI setup or Layer 3 commands.
Core support metadata must remain documented/untested, never verified in PT.

Each spec carries its Cisco source, bilingual labels/parameter descriptions,
mode, prerequisites, and suggested show commands. Verification templates retain
angle-bracket placeholders for a caller to display or substitute, and are not
automatically executed. Tests use only synthetic `CommandRequest` inputs through
`cisco::generate` and assert commands, rendering, device restrictions, required
context, injection/range rejection, STP increments, MAC validity, capacity/aging
conflicts, and the full 25-pair EtherChannel protocol matrix.

Validation commands: `cargo test --test cisco_switch`
(15 tests, including loops over all 20 specs and positive/negative contexts).
`cargo test --lib` also passed 15 existing library tests, and
`cargo clippy --lib --test cisco_switch -- -D warnings` passed.
Owned Rust files are formatted directly with rustfmt; shared files are unchanged.
