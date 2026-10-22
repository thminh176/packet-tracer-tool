# Cisco Command Library UI Specification & Documentation

## Overview

The **Cisco Command Library** tab is a native, bilingual egui interface embedded within the PKA Lab Inspector (`pka-lab-inspector`). It provides students and network engineers with an offline, typed reference catalog and CLI generator for Cisco IOS command families across routers and switches.

The UI leverages typed, read-only shared contracts from `pka_lab_inspector::cisco` (`src/cisco/mod.rs`):
- `cisco::registry() -> Vec<CommandSpec>`: Offline repository of registered command specifications.
- `cisco::DeviceProfile::ALL`: Supported device hardware profiles with explicit capabilities and support mapping.
- `cisco::CommandRequest { family, device, profile, parameters }`: Input specification for script synthesis.
- `cisco::generate(req) -> CommandPlan`: Defensive generator computing executable CLI mode commands, missing parameters, warnings, and evidence.
- `plan.executable() -> bool` & `plan.script() -> String`: Strict execution gate preventing partial or unsupported script emission.

---

## 1. Navigation & Tab Hierarchy

The Command Library is integrated as **Tab 7** in the inspector application without disrupting the existing six assessment workflows:

| Tab Index | Shortcut | Key / Catalog String | Description |
|---|---|---|---|
| 1 | `Ctrl+1` | `nav.overview` | Assessment Overview & Summary Score |
| 2 | `Ctrl+2` | `nav.checkpoints` | Virtualized Checkpoints Table & Detail Panel |
| 3 | `Ctrl+3` | `nav.next_actions` | Pending Remediation Actions |
| 4 | `Ctrl+4` | `nav.device_commands` | Assessment Device Configuration Scripts |
| 5 | `Ctrl+5` | `nav.raw_xml` | Virtualized Raw PKA/PKT XML Viewer |
| 6 | `Ctrl+6` | `nav.diagnostics` | Engine & Parser Diagnostics |
| **7** | **`Ctrl+7`** | **`nav.command_library`** | **Native Bilingual Cisco Command Library & Script Generator** |

Unlike assessment-bound tabs (which display `empty.welcome` until a `.pka`/`.pkt` file is loaded), the Cisco Command Library functions as an offline reference tool accessible both with and without an active lab loaded.

---

## 2. Search, Filters & Controls

The library interface is organized with a top toolbar and a two-pane master-detail view:

### 2.1 Search Bar
- **Multi-field matching**: Filters command specifications case-insensitively across:
  - English display name (`spec.name_en`)
  - Vietnamese display name (`spec.name_vi`)
  - Family identifier (`spec.id`)
  - Syntax pattern (`spec.syntax`)
  - Topic category (`spec.topic`)
- **Quick clear button (`✖`)**: Clears search text immediately.

### 2.2 Device Category Filter
- **All Devices**: Shows all registered command specifications, including common basic commands.
- **Routers**: Filters to router command families (`router.*`, `Capability::Routing`, `DynamicRouting`, `Ipv6Routing`) plus common `basic.*` commands.
- **Switches**: Filters to Catalyst switch families (`switch.*`, `Capability::Switching`, `Multilayer`) plus common `basic.*` commands.

### 2.3 Topic Filter
- Dynamically populated combo box containing all unique topics extracted from the registry (e.g. `interface`, `vlan`, `trunk`, `ospf`, `routing`, `services`, `management`).

### 2.4 Device Profile Selector
- Selects target device hardware profile from `cisco::DeviceProfile::ALL`:
  - `Unknown IOS` (fallback profile)
  - `Generic router`
  - `Generic switch`
  - `Catalyst 2960`
  - `Catalyst 3560`
  - `Cisco 1941`
  - `Cisco 2911`
  - `ISR 4321 (IOS XE)`
- Switching profile immediately re-evaluates support status and command generation.

### 2.5 Target Device Name Input
- Configures target hostname / device identifier (`library_target_device`, defaults to `"Router0"`).
- Parameter validation enforces safe characters and bounds (`<= 128` characters, alphanumeric and standard separators `._-/:`).

---

## 3. Command Specification Master-Detail View

### 3.1 Left Master List Panel
- Displays filtered command items in a scrollable list.
- Each item displays:
  - Localized title (`name_vi` if language is Vietnamese, `name_en` otherwise)
  - Unique ID badge (e.g. `switch.port_mode`, `router.interface_ip`)
  - Topic badge
  - Compact profile support badge evaluated for the active `DeviceProfile`.
- Selection updates `library_selected_id` and prepares the parameter form.

### 3.2 Right Detail & Generator Panel

#### A. Header & Metadata
- Full localized title and family ID.
- Configuration Mode: e.g. `Global`, `Interface`, `Router`, `Vlan`, `Line`, `Exec`.
- Capability class: e.g. `Basic`, `Switching`, `Routing`, `Multilayer`, `Security`.

#### B. Profile Support State & Evidence Guardrails
The support state for `(profile, spec.capability)` is evaluated defensively:
- **`VerifiedPacketTracer`** (Green): Labeled as `"Packet Tracer Verified"` / `"Đã xác minh Packet Tracer"` **only** when empirical execution verification exists. The UI strictly adheres to the rule: *never label PT verified unless support is actual `VerifiedPacketTracer`*.
- **`DocumentedNotTested`** (Blue): Labeled as `"Documented (Not PT Tested)"` / `"Chuẩn tài liệu (Chưa kiểm thử PT)"`. Indicates standard Cisco IOS syntax that has not been execution-tested in Packet Tracer.
- **`Unsupported`** (Red): Labeled as `"Unsupported on Selected Profile"` / `"Không hỗ trợ trên hồ sơ này"`.
- **`Unknown`** (Gray): Labeled as `"Unknown Profile Compatibility"` / `"Chưa xác định độ tương thích"`.

#### C. Syntax & Source Documentation
- Command syntax rendered in a monospace container with a quick Copy button.
- Source documentation link:
  - If a valid URL (`http://` or `https://`), rendered as a clickable hyperlink (`ui.hyperlink_to`).
  - Textual reference displayed clearly with an adjacent "Copy reference" button.

#### D. Prerequisites & Verification
- **Prerequisites**: Clear list of prerequisite configuration modes or dependencies.
- **Verification Commands**: List of show/verification commands (e.g. `show interfaces status`, `show ip interface brief`, `show ip route`) with individual copy buttons and a "Copy all verification commands" option.

#### E. Parameter Configuration Form
- Dynamically rendered from `spec.parameters`:
  - Parameter name and status badge (`[Required]` in red vs `[Optional]`).
  - Localized parameter description (`description_vi` / `description_en`).
  - Safe example reference value from `param.example`.
  - Individual `"Use example"` button to populate the field with the safe example.
  - Form-wide `"Fill all safe examples"` button to populate all parameters instantly.
  - `"Clear parameters"` button to reset inputs.

#### F. CLI Script Preview & Execution Gate
The plan is synthesized via `cisco::generate(&req)`:
- **When `plan.executable() == true`**:
  - Full CLI script rendered in a syntax-highlighted monospace editor.
  - Active `"Copy CLI script"` button with immediate toast notification and status update.
  - Operator safety advisory: `"Paste into the target device CLI in Packet Tracer. Never run in PowerShell or a system terminal."`
- **When `plan.executable() == false`**:
  - Script output and copy button are gated.
  - Visual status notice: `"Script generation incomplete: please resolve warnings and missing parameters below."`
  - Explicit diagnostic panels:
    - **Missing Parameters**: Clear bulleted list of unfilled required fields.
    - **Warnings**: Safety warnings (e.g. unsupported hardware, missing execution evidence).
    - **Conflicts**: Conflicting mode or configuration requirements.
    - **Supporting Evidence**: Documented reference citations.

---

## 4. Internationalization & Localization

All text elements are localized into both English (`en`) and Vietnamese (`vi`) in `locales/en.json` and `locales/vi.json`. Key catalog parity and token placeholders are strictly verified by unit tests:

| Key Category | English Example | Vietnamese Example |
|---|---|---|
| `nav.command_library` | Cisco Library | Thư viện Cisco |
| `cisco.search_placeholder` | Search commands by name, ID, or syntax… | Tìm kiếm theo tên, mã lệnh, hoặc cú pháp… |
| `cisco.filter_router` | Routers | Router |
| `cisco.filter_switch` | Switches | Switch |
| `cisco.support.verified_pt` | Packet Tracer Verified | Đã xác minh Packet Tracer |
| `cisco.support.documented_not_tested` | Documented (Not PT Tested) | Chuẩn tài liệu (Chưa kiểm thử PT) |
| `cisco.support.unsupported` | Unsupported on Selected Profile | Không hỗ trợ trên hồ sơ này |
| `cisco.support.unknown` | Unknown Profile Compatibility | Chưa xác định độ tương thích |
| `cisco.fill_all_examples` | Fill safe examples | Điền tất cả ví dụ an toàn |
| `cisco.copy_script` | Copy CLI script | Sao chép tập lệnh CLI |
| `cisco.missing_parameters` | Missing required parameters: | Thiếu các tham số bắt buộc: |

Cisco syntax keywords (e.g. `interface`, `switchport mode trunk`, `ip address`, `router ospf`) remain unmodified technical standards in both locales.

---

## 5. Verification & Testing

The UI implementation is covered by tests in `src/ui.rs`:
- `test_tab_metadata_and_shortcuts`: Validates all 7 tabs have non-empty titles and valid `Ctrl+1` through `Ctrl+7` shortcuts.
- `test_cisco_device_filtering`: Verifies category filtering across Routers, Switches, and All.
- `test_cisco_search_query_matching`: Tests search query matching by name_en, name_vi, id, syntax, and topic.
- `test_cisco_support_badge_states`: Ensures strict adherence to support state labels and confirms PT verified is never assigned unless `SupportState::VerifiedPacketTracer`.
- `test_cisco_parameter_form_and_examples`: Verifies "Fill safe examples", parameter map population, and executable plan transition.
- `rendering_checks::all_tabs_render_with_actual_synthetic_analysis_and_detail_selection`: Renders all 7 tabs through egui's layout pipeline to verify zero panics and valid render output.
- `localization::tests::embedded_keys_and_placeholders_match`: Verifies 100% key and placeholder parity across language catalogs.
