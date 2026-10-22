# Cisco Catalyst Switch Command Correctness Audit (v0.2)

## 1. Executive Summary

This correctness audit evaluates the bounded Cisco IOS Catalyst switch command handlers in `pka-lab-inspector` (`src/cisco/switch/*`) against official Cisco IOS 15.0(1)SE / 15.2(2)E documentation and designated Vietnamese networking references:
* `Cisco_IOS_IOS_XE_Huong_Dan_Cau_Hinh_Tieng_Viet.pdf` (34 pages)
* `So_tay_Cisco_A4_Tra_nhanh_lenh_theo_nhom.pdf` (6 pages)
* `So_tay_Cisco_A4_Tra_nhanh_theo_nhom.pdf` (6 pages)
* `So_tay_lenh_Cisco_Thuc_hanh_Thiet_bi_that.pdf` (21 pages)

All 20 bounded Catalyst command specifications across Switching and Multilayer capabilities were audited for syntax fidelity, configuration mode context, prerequisite dependencies, verification command accuracy, and device platform compatibility. Concrete errors in error diagnostics, verification commands, and reserved/prohibited VLAN validation were resolved, and regression tests were added in `tests/cisco_switch.rs`.

---

## 2. Methodology & Boundaries

* **Owned Scope**: `src/cisco/switch/handlers.rs`, `src/cisco/switch/mod.rs`, `tests/cisco_switch.rs`, and `docs/SWITCH_CORRECTNESS_AUDIT.md`.
* **Preservation**: Shared interfaces (`CommandRequest`, `CommandPlan`, `DeviceProfile`, `ConfigMode`, `ParameterSpec`) and other modules (`router`, `basic`, `mapping`, `ui`) were strictly preserved.
* **Evidence-Based Context**: Per design, command generation is derived entirely from explicit caller evidence rather than assumed switch state.
* **Execution Disclaimer**: Commands are audited and documented against Cisco IOS references; no actual Packet Tracer execution or runtime emulation is claimed. Protected textual excerpts are not copied into source control.

---

## 3. Audited Command Catalog & Reference Mapping

| Command Family | IOS Syntax | Config Mode | Primary Cisco Source & Guidelines | Secondary PDF Reference |
|---|---|---|---|---|
| `switch.vlan` | `vlan <vlan>; name <name>` | `global` / `config-vlan` | [Cisco 3560 SCG - VLANs](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swvlan.html) | Guide pp. 8, 28; So tay pp. 7, 28; A4 pp. 3, 5 |
| `switch.port_mode` | `switchport mode {access \| trunk}` | `interface` | [Cisco 2960 CG - Interface Trunking](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_01101.html) | Guide pp. 8, 9, 21, 28; So tay pp. 8, 17; A4 pp. 3, 5 |
| `switch.access_vlan` | `switchport access vlan <vlan>` | `interface` | [Cisco 3560 SCG - VLANs](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swvlan.html) | Guide pp. 8, 21, 28; So tay pp. 7, 8; A4 pp. 3, 5 |
| `switch.voice_vlan` | `switchport voice vlan <vlan>` | `interface` | [Cisco 3560 SCG - Voice VLAN](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swvoip.html) | Guide p. 8; A4 p. 3 |
| `switch.trunk_native` | `switchport trunk native vlan <vlan>` | `interface` | [Cisco 2960 CG - Trunking](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_01101.html) | Guide pp. 9, 28; So tay p. 8; A4 p. 3 |
| `switch.trunk_allowed` | `switchport trunk allowed vlan [add\|remove\|except] <vlans> \| all \| none` | `interface` | [Cisco 2960 CG - Trunking](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_01101.html) | Guide pp. 9, 21, 28; So tay p. 8; A4 p. 3 |
| `switch.stp_mode` | `spanning-tree mode {pvst \| rapid-pvst \| mst}` | `global` | [Cisco 2960 CG - STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_010000.html) | Guide pp. 8, 20; So tay pp. 7, 17; A4 pp. 2, 5 |
| `switch.stp_root` | `spanning-tree vlan <vlans> root {primary \| secondary}` | `global` | [Cisco 2960 CG - STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_010000.html) | Guide p. 28; So tay p. 8; A4 p. 2 |
| `switch.stp_priority` | `spanning-tree vlan <vlans> priority <priority>` | `global` | [Cisco 2960 CG - STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_010000.html) | Guide p. 28; So tay p. 17; A4 p. 2 |
| `switch.stp_portfast` | `spanning-tree portfast` | `interface` | [Cisco 3560 SCG - Optional STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swstpopt.html) | Guide pp. 8, 21; So tay pp. 7, 17; A4 p. 4 |
| `switch.stp_bpduguard` | `spanning-tree bpduguard enable` | `interface` | [Cisco 3560 SCG - Optional STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swstpopt.html) | So tay p. 17; A4 p. 5 |
| `switch.stp_rootguard` | `spanning-tree guard root` | `interface` | [Cisco 3560 SCG - Optional STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swstpopt.html) | Guide p. 28; A4 p. 2 |
| `switch.stp_cost` | `spanning-tree cost <cost>` | `interface` | [Cisco 2960 CG - STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_010000.html) | Guide p. 20; So tay p. 17; A4 p. 2 |
| `switch.stp_port_priority` | `spanning-tree port-priority <priority>` | `interface` | [Cisco 2960 CG - STP](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_010000.html) | Guide p. 20; A4 p. 2 |
| `switch.port_security` | `switchport port-security [maximum \| mac-address \| violation \| aging]` | `interface` | [Cisco 2960 CG - Port Security](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/m_1522e_sec_port_based_traffic_ctrl_2960_cg.html) | Guide p. 20; So tay p. 17; A4 pp. 2, 4 |
| `switch.etherchannel` | `channel-group <group> mode {active \| passive \| auto \| desirable \| on}` | `interface` | [Cisco 2960 CG - EtherChannel](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_0100101.html) | Guide pp. 12, 13, 29; So tay p. 8; A4 p. 2 |
| `switch.svi.ipv4` | `interface Vlan<vlan>; ip address <address> <mask>; no shutdown` | `interface` | [Cisco 3560 SCG - IP Addressing](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swipaddr.html) | Guide p. 10; So tay pp. 7, 9; A4 p. 3 |
| `switch.default_gateway` | `ip default-gateway <gateway>` | `global` | [Cisco 3560 SCG - IP Addressing](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swipaddr.html) | Guide pp. 10, 30; So tay p. 9; A4 p. 3 |
| `switch.routed_port` | `no switchport; ip address <address> <mask>; no shutdown` | `interface` | [Cisco 3560 SCG - L3 Interfaces](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swint.html) | Guide pp. 11, 12; So tay p. 7; A4 p. 3 |
| `switch.ip_routing` | `ip routing` | `global` | [Cisco 3560 SCG - IP Routing](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swiprout.html) | Guide pp. 11, 30; So tay p. 7; A4 p. 3 |

---

## 4. Findings & Concrete Correctness Fixes

### 4.1. Coordinator correction: preserve documented gateway verification
The worker initially proposed replacing `show ip redirects`. Integration review rejected that proposal: Cisco's [2960 IOS 15.2(2)E gateway procedure](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_0101010.html) explicitly uses this command to verify the default gateway. The final candidate retains `show ip redirects` and `show running-config`. Simulator support remains untested.

### 4.2. Defect Fix: Erroneous Error Diagnostic in `switch.stp_port_priority`
* **Defect**: When an invalid priority not divisible by 16 was supplied to `switch.stp_port_priority`, the handler returned `"Bridge priority must be a multiple of 16"`. Bridge priority (per-VLAN STP root bridge priority) is stepped in increments of 4096 (0..61440), whereas interface port priority is stepped in increments of 16 (0..240).
* **Fix**: Corrected error message to `"Port priority must be a multiple of 16"`.
* **References**: Cisco Catalyst 2960 Software Configuration Guide (Configuring STP Port Priority: 0 to 240 in steps of 16); `tests/cisco_switch.rs` regression assertion added.

### 4.3. Semantic Fix: Rejection of Reserved VLANs (1002–1005) on Access & Voice Ports
* **Defect**: Cisco IOS explicitly rejects assigning access ports or voice ports to legacy reserved VLANs 1002 through 1005 (`% Access VLAN cannot be 1002-1005. Configure a different VLAN.`). Previously, `switch.access_vlan` and `switch.voice_vlan` allowed inputs in 1002..=1005.
* **Fix**:
  * In `src/cisco/switch/handlers.rs`, added explicit validation rejecting VLANs 1002..=1005 for both `switch.access_vlan` and `switch.voice_vlan`.
  * Coordinator review removed the proposed VLAN 1 prohibition: the [Catalyst voice VLAN guide](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_4_e/configurationguide/b_1524e_consolidated_2960p_2960c_cg/m_1522e_vlan_voice_2960s_cg.html) documents IDs 1–4094. Small Business switch restrictions must not be applied to Catalyst models. The regression now preserves VLAN 1 with the existing required context.
* **References**: Cisco 3560 SCG (VLAN Architecture & Voice VLAN Configuration); `So_tay_lenh_Cisco_Thuc_hanh_Thiet_bi_that.pdf` p. 7.

### 4.4. Semantic Fix: Rejection of Reserved VLANs (1002–1005) on Management SVI
* **Defect**: VLANs 1002–1005 are hard-coded for legacy Token Ring/FDDI (`act/unsup`). Creating an IPv4 management interface (`interface Vlan1002`) with addressing and routing is unsupported on Catalyst switches.
* **Fix**: Added validation in `switch.svi.ipv4` handler rejecting VLANs 1002..=1005.
* **References**: Cisco 3560 SCG (Configuring SVI and IP Addressing).

### 4.5. Verification Completeness: 802.1Q Trunk Status & Diagnostic Inconsistencies
* **Defect**: `switch.trunk_native` and `switch.trunk_allowed` previously used generic physical port checks (`show interfaces <interface> switchport`) without the primary trunk verification command cited across all Vietnamese reference guides.
* **Fix**:
  * Added `show interfaces trunk` to verification vectors for `switch.trunk_native` and `switch.trunk_allowed`.
  * Added `show interfaces status err-disabled` to `switch.stp_bpduguard` verification (diagnosing ports transitioned to err-disabled state upon BPDU receipt).
  * Added `show spanning-tree inconsistentports` to `switch.stp_rootguard` verification (diagnosing root-inconsistent state).
  * Updated `switch.trunk_allowed` syntax specification to accurately reflect `all` and `none` options: `switchport trunk allowed vlan [add|remove|except] <vlans> | all | none`.
* **References**: `So_tay_lenh_Cisco_Thuc_hanh_Thiet_bi_that.pdf` p. 8; `Cisco_IOS_IOS_XE_Huong_Dan_Cau_Hinh_Tieng_Viet.pdf` pp. 9, 21; `So_tay_Cisco_A4_Tra_nhanh_lenh_theo_nhom.pdf` pp. 2, 5.

---

## 5. Shared-Module Defects Identified (Outside Ownership Boundary)

The following two defects were discovered in shared code and formally escalated to the coordinator:
1. **`src/cisco/parser.rs` (`interface_name`)**: When parsing interface names containing internal whitespace between the interface type and port identifier (e.g., `"FastEthernet 0/1"` or `"vlan 10"`), `split_at(first)` preserves the trailing whitespace in `prefix`, causing match failures against `"fastethernet"` / `"vlan"`. Trimming whitespace from `prefix` would safely permit space-separated interface inputs.
2. **Semantic validation flags:** Coordinator review confirms the narrowed catalog flags are intentional. Input validation and syntax generation do not establish semantic comparison support. Families without implemented saved-configuration validators remain marked false.

---

## 6. Verification & Test Regression Results

Scoped unit and regression tests were executed with parallel build resource constraints respected:
```text
cargo test --test cisco_switch -j 2
```
Results:
* `access_and_voice_require_explicit_access_context`: worker baseline passed. Coordinator updated it to preserve documented Catalyst voice VLAN 1 and retain reserved-VLAN checks; integrated results are recorded separately.
* `etherchannel_consistency_and_conditional_metadata`: **PASSED** (verifies L2 port consistency, member counts, PAgP/LACP speed/duplex matching).
* `etherchannel_protocol_matrix_requires_a_forming_peer`: **PASSED** (verifies active/passive, desirable/auto, on/on matrices).
* `every_required_dependency_is_enforced_and_inputs_are_closed`: **PASSED** (ensures closed parameter validation).
* `every_spec_has_independent_bilingual_metadata_and_a_working_typed_example`: **PASSED** (verifies all 20 specs).
* `l2_commands_reject_svis_routed_and_interface_range_injection`: **PASSED** (verifies physical interface validation, blocks interface-range injection).
* `mode_conversion_uses_device_encapsulation_and_never_guesses_l3`: **PASSED** (verifies Catalyst 3560 dot1q encapsulation vs Catalyst 2960).
* `multilayer_operations_require_3560_and_confirmed_image`: **PASSED** (verifies 3560 requirement for routed ports and IP routing).
* `port_security_sticky_static_and_aging_order`: **PASSED** (verifies sticky learning, static unicast MAC, aging timer/type/static ordering).
* `router_device_profiles_never_emit_switch_configuration`: **PASSED** (verifies router profiles reject switch commands).
* `stp_enums_steps_and_boundaries`: **PASSED** (verifies bridge priority step 4096, port priority step 16, diagnostic message verification).
* `stp_guards_reject_missing_or_conflicting_context`: **PASSED** (verifies portfast/BPDU guard/root guard context and err-disabled/inconsistentports verification commands).
* `svi_management_and_gateway_validate_subnet_hosts`: worker baseline passed. Coordinator corrected its verification assertion to preserve the documented `show ip redirects`; integrated results are recorded separately.
* `trunk_operations_and_native_peer_consistency`: **PASSED** (verifies trunk allowed operations, native VLAN matching, `show interfaces trunk` verification).
* `vlan_is_rendered_once_in_vlan_mode`: **PASSED** (verifies VLAN creation and naming).

**Overall Suite Outcome**: 15 passed; 0 failed; 0 ignored; duration 0.01s.
