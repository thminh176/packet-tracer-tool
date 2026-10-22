# Verification & Command Explanation Copy Review

**Target:** `locales/en.json`, `locales/vi.json`, `docs/VERIFICATION_COPY_REVIEW.md`  
**Date:** 2026-10-22  
**Component:** Localization Catalogs & UX Copy for Assessment Explanations  
**Status:** Completed  

---

## 1. Executive Summary & Problem Context

In user testing and feedback reports, users frequently conflated the generic **Unverified** checkpoint explanation with an inability of the application to generate expected CLI configuration commands. Specifically:

1. **The Confusion:** When an activity is inspected without an active or uniquely identified current-state snapshot (such as opening an uncompleted `.pka` without a paired student save, or when runtime evidence is unobserved), checkpoints are marked as `Unverified`. The prior copy stated *"No verified current snapshot evidence"*, which led users to assume that the tool failed, encountered an error, or lacked the ability to suggest CLI commands.
2. **The Reality:** Verification status and command generation are decoupled:
   - **Verification** evaluates whether the user's current device configuration matches expected grading criteria based on runtime/progress snapshot evidence.
   - **Command Generation** synthesizes recommended CLI configurations directly from requirement criteria (e.g. AccessVLAN, NativeVLAN, AllowedVLANs, VLANName), allowing students and instructors to see required configuration steps even before runtime evidence is collected.
3. **The Risk of Overpromising:** While clarifying that commands can still be generated from requirements, copy must not promise that all commands are supported or that generated commands are guaranteed to execute unconditionally without operator verification.

This review documents copy improvements in `locales/en.json` and `locales/vi.json` that clearly delineate missing current evidence from command generation capabilities while reinforcing safety warnings, VLAN/port mode prerequisites, and reference syntax boundaries.

---

## 2. Catalog Key Changes & Localization Mapping

All modifications maintain exact 1-to-1 parity between English (`en`) and Vietnamese (`vi`) catalogs, with matching placeholders and zero alterations to technical syntax.

| Key | English (`en.json`) | Vietnamese (`vi.json`) | Category / Role |
| :--- | :--- | :--- | :--- |
| `detail.reference_syntax` | `Reference syntax, not a complete command` | `Cú pháp tham khảo, chưa phải lệnh hoàn chỉnh` | **New Key**: Labels command syntax templates in catalogs to distinguish them from full contextualized device scripts. |
| `explanation.requirement_command` | `Requirement-based command suggestion; verify VLAN state and port mode where applicable. Current configuration has not been verified.` | `Gợi ý lệnh dựa trên yêu cầu; hãy kiểm tra trạng thái VLAN và chế độ cổng khi cần. Cấu hình hiện tại chưa được xác minh.` | **New Key**: Explains that command preview is derived from requirement criteria (such as VLAN parameters) without runtime verification, cautioning about prerequisites. |
| `explanation.no_current` | `No current-state evidence to verify; requirement-based command suggestions may still be generated where supported.` | `Chưa có dữ liệu cấu hình hiện tại để đối chiếu; lệnh gợi ý theo yêu cầu vẫn có thể được tạo nếu thuộc tính được hỗ trợ.` | **New Key**: Clarifies that unverified status denotes lack of current-state evidence rather than CLI generation failure. |
| `explanation.no_snapshot` | `No verified current snapshot evidence` | `Chưa có bằng chứng xác minh cấu hình hiện tại` | **Preserved Literal**: Retained exact string matching `src/inspector.rs:335` to preserve `Localizer.message` translation lookup compatibility without core modifications. |
| `explanation.review_command` | `Review explicit values and device compatibility before applying. Suggestion is generated from requirements without runtime verification; no commands have been executed.` | `Kiểm tra các giá trị cụ thể và tính tương thích của thiết bị trước khi áp dụng. Lệnh được tạo từ yêu cầu mà chưa đối chiếu trạng thái hiện tại; ứng dụng chưa thực thi lệnh nào.` | **Updated**: Command preview advice explicitly clarifies requirement derivation and safe manual execution boundaries. |

---

## 3. Detailed UX Rationale & Copy Principles

### 3.1 Disentangling "Unverified" from "Unsupported CLI"
- **Prior Problem:** A checkpoint reporting *"No verified current snapshot evidence"* offered no hint regarding whether remediation commands could be generated or viewed. Users assumed "Unverified" meant "Not functional".
- **Improved Solution (`explanation.no_current`):** By stating *"No current-state evidence to verify; requirement-based command suggestions may still be generated where supported"*, the user immediately understands that:
  - The assessment engine lacks snapshot observation data to grade the checkpoint.
  - The command synthesis engine can nonetheless generate recommended CLI commands from the assessment requirement specifications (for supported properties).
  - The phrase *"where supported"* / *"nếu thuộc tính được hỗ trợ"* explicitly avoids promising universal command support.
- **Source Literal Compatibility (`explanation.no_snapshot`):** Because `Localizer::message` resolves translations by matching English source literals (e.g. `src/inspector.rs:335`), `explanation.no_snapshot` preserves the exact string *"No verified current snapshot evidence"* so runtime diagnostics translate reliably without requiring immediate core source changes.

### 3.2 Reference Syntax vs. Actionable Command Script
- **Prior Problem:** Users viewing command catalog entries occasionally mistook reference command templates (with generalized placeholders or partial context) for ready-to-run terminal scripts.
- **Improved Solution (`detail.reference_syntax`):** The explicit label *"Reference syntax, not a complete command"* (*"Cú pháp tham khảo, chưa phải lệnh hoàn chỉnh"*) establishes clear user expectations before users copy or attempt to execute snippets.

### 3.3 Requirement-Based Suggestions & VLAN/Prerequisite Boundaries
- **Prior Problem:** Users applying generated commands assumed the tool had inspected device interfaces, confirmed feature prerequisites (e.g. VLAN existence before access port assignment, routing process enablement), and validated hardware compatibility.
- **Improved Solution (`explanation.requirement_command`, `explanation.review_command`):** The copy clearly states that commands are generated from explicit requirements rather than observed state, reminding students and operators to verify VLAN state and port mode where applicable before applying scripts into Packet Tracer.

---

## 4. Technical Validation & Safeguards

1. **Key and Placeholder Parity:**
   - Both `locales/en.json` and `locales/vi.json` contain exactly 296 keys.
   - All message placeholders (`{device}`, `{example}`, `{value}`, etc.) are verified identical across both language packs.
2. **UTF-8 and Diacritics Integrity:**
   - Vietnamese diacritics and typography conform to standard Vietnamese networking terminology (e.g., *"Cú pháp tham khảo"*, *"trạng thái VLAN và chế độ cổng"*, *"đối chiếu"*).
3. **Scope Enforcement:**
   - Changes are strictly confined to `locales/en.json`, `locales/vi.json`, and `docs/VERIFICATION_COPY_REVIEW.md`.
   - No modifications were made to `src/ui.rs`, `src/commands.rs`, `src/cisco/`, contracts, or test suites.
   - UI wiring and implementation remain owned by the coordinator.
