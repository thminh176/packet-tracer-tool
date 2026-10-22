# UI Review & Release Audit: PKA Lab Inspector

**Version:** 0.1.0  
**Target:** Native Windows GUI (`src/ui.rs`, `src/main.rs`, `docs/images/inspector-windows.png`, `docs/UI_REVIEW.md`)  
**Assessment Date:** 2026-10-22  

---

## 1. Executive Summary

This review validates the UI polish and localized interaction refinements implemented for the v0.1.0 release of PKA Lab Inspector. The existing GUI architecture, assessment parser, and command generator were strictly preserved without redesign or core modifications. All translation updates utilized existing stable localization keys in `locales/en.json` and `locales/vi.json`.

Key additions include:
- A prominent, concise localized warning banner near generated device commands informing users that commands belong strictly inside target Packet Tracer device CLIs and must never be run in PowerShell or a host terminal.
- A dedicated, collapsible section displaying unsupported/GUI-only checkpoints separately with concise localized explanations.
- A Settings window providing bilingual language selection with proper alias normalization (`en-US`/`en` and `vi-VN`/`vi`), view options (compact rows, manual checklist filter), and independent branding with non-affiliation disclaimers.
- Validation across DPI scaling, Unicode/Vietnamese diacritics, and responsive layout constraints.

---

## 2. Localization & Language Selection (en-US / vi-VN)

### 2.1 Locale Code Normalization
The underlying catalog uses canonical two-letter codes (`en`, `vi`) while exposed language selectors provide regional BCP 47 codes (`en-US`, `vi-VN`). Prior to this update, selecting `vi-VN` when default preferences were `vi` could result in raw code displays. 

The UI combo boxes in both the main toolbar and the Settings dialog now normalize alias mappings:
```rust
let is_selected = code == self.preferences.language
    || (code == "vi-VN" && self.preferences.language == "vi")
    || (code == "en-US" && self.preferences.language == "en");
```
This ensures that the active language ("Tiếng Việt" or "English") is consistently recognized, highlighted, and persisted.

### 2.2 Settings Dialog
The new Settings dialog (`menu.settings`) provides:
- Primary language selector with dynamic reload support (`menu.reload_languages`).
- View configuration checkboxes for compact table rows (`ui.compact_rows`) and filtering to unchecked manual items (`ui.show_only_unchecked`).
- Original application metadata and trademark disclaimer (`disclaimer.cisco`).

---

## 3. Device Commands & Remediation Guidance

### 3.1 Localized Target Device Notice
Packet Tracer CLI commands (e.g. Cisco IOS configuration blocks) must never be executed inside the Windows host terminal or PowerShell. 

A prominent warning banner is displayed above the script viewer in both `DeviceCommands` and `NextActions` tabs:
- **Vietnamese (`vi`):** *"Dán vào CLI của đúng thiết bị trong Packet Tracer. Không chạy trong PowerShell hoặc terminal hệ thống."*
- **English (`en`):** *"Paste into the target device CLI in Packet Tracer. Never run in PowerShell or a system terminal."*

### 3.2 Separate Unsupported Checkpoint Details
Previously, unsupported or GUI-only checkpoints were simply omitted from the CLI script text. They are now presented in an explicit collapsible section (`ui.unsupported_checkpoints_title`) showing:
- Checkpoint count for the selected device filter or entire lab.
- Localized requirement explanation (`ui.unsupported_checkpoints_desc`).
- Itemized device, interface, property, expected value, and localized diagnostic rationale (`catalog.message(&language, &cmd.explanation)` or GUI instruction).

### 3.3 Core Command Generation Preservation
The core command generation engine in `src/commands.rs` (including `device_script` deduplication, port security ordering, and conflict detection) remains completely unmodified, adhering to release boundaries.

---

## 4. Branding & Trademark Compliance

- **Original Branding:** The project is branded exclusively as **PKA Lab Inspector**. No Cisco logos, emblems, or proprietary graphic assets are bundled or displayed.
- **Trademark Disclaimer:** The legal notice from `locales/*.json` is displayed within the Settings dialog:
  > *"Independent project; not affiliated with or endorsed by Cisco. Cisco and Packet Tracer are trademarks of their respective owner."*

---

## 5. DPI, Unicode & Responsiveness

- **Unicode & Font Rendering:** The application dynamically loads the Windows system font (`%WINDIR%/Fonts/segoeui.ttf`) directly from the operating system into `egui::FontDefinitions`. This ensures complete rendering of tone marks and Vietnamese vowels (e.g. `ế`, `ệ`, `ở`) without distributing proprietary font files.
- **DPI Scaling:** `eframe` natively tracks Windows per-monitor DPI scale factors, ensuring crisp text and vector widget boundaries at 100%, 125%, 150%, and 200% scaling.
- **Responsive Layout:**
  - Window defaults to `1280x820` with an enforced minimum of `960x600`.
  - Top toolbar and CentralPanel tab headers utilize `horizontal_wrapped` to avoid button clipping or truncation at narrow widths.
  - Virtualized checkpoint tables and XML rows leverage horizontal scroll containers with auto-sized column boundaries.

---

## 6. Test & Static Analysis Evidence

### 6.1 Cargo Test Suite
All unit and integration tests execute successfully with zero failures:
```text
running 6 tests in src/lib.rs ... ok (6 passed; 0 failed)
running 24 tests in src/main.rs (ui::tests + ui::rendering_checks) ... ok (24 passed; 0 failed)
running 12 tests in tests/commands.rs ... ok (12 passed; 0 failed)
running 30 tests in tests/inspector.rs ... ok (29 passed; 0 failed; 1 ignored: real_local_pka_verification)

Total: 71 passed, 0 failed, 1 ignored
```

### 6.2 Clippy Lints
Static analysis via Cargo Clippy on `pka-lab-inspector` binary reports zero warnings or errors.

---

## 7. Live Smoke Verification & Clipboard Evidence

Live smoke validation was executed against the distributable `tests/fixtures/synthetic-126.xml` fixture:
- **Synthetic Assessment Ingestion:** 126 synthetic checkpoints loaded; 7 passed, 7 failed, 112 unverified; total score contribution 7.0 / 126.0.
- **Native Clipboard Verification:**
  The native OS clipboard interaction via `copy_text_with_feedback` was invoked for synthetic checkpoint CLI generation and verified via PowerShell `Get-Clipboard`:
  ```text
  enable
  configure terminal
  interface GigabitEthernet0/0
  ip address 192.0.1.1 255.255.255.0
  exit
  end
  ```
- **Handoff Notice:** Scope steering directive received from coordinator requesting relinquishment of exclusive GUI smoke execution. All runtime test additions removed; final interactive smoke testing and live GUI sessions handed off to coordinator upon task settlement.
