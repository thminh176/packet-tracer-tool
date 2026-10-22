# Security policy

The latest 0.1.x version is the supported release line. Treat all activity files and decoded XML as untrusted input. The app is offline and never executes Cisco commands; review generated configuration before pasting into a target device CLI.

Please report suspected vulnerabilities with GitHub's **Report a vulnerability** feature under this repository's Security tab when available. If unavailable, open a minimal issue asking for a private reporting channel, without exploit details, private files or secrets. Do not attach coursework or decoded answer data. Maintainers will triage reports and coordinate a fix; no guaranteed response SLA is offered.

Known limits include supported-schema gaps, inferred assessment selection semantics and ambiguous snapshot roles. These are documented in docs/COMPATIBILITY.md. The app does not bypass activity passwords, modify grades or replace Cisco's native assessment engine.

Never run generated IOS scripts in PowerShell, Command Prompt or a host shell. Copy one device's supported commands into that device's CLI in Packet Tracer.

## Dependency maintenance snapshot (2026-10-22)

The frozen v0.1.0 graph passes `cargo audit` with zero known vulnerabilities at advisory database revision `59ef7abf`. Informational unmaintained notices remain for `ttf-parser` (used by the Windows font stack) and the non-Windows D-Bus packages `zbus_names`, `zvariant`, and `zvariant_derive`. They are maintenance risks, not a clean-bill-of-health guarantee. We accept these notices for the tested egui 0.33 MVP and track a GUI-stack upgrade for a later feature branch. Only the default embedded fonts and installed Windows system font are used; arbitrary user font loading is not exposed. New vulnerability advisories must still fail the release security check; no blanket advisory suppression is configured. `cargo deny check licenses` is the separate license gate; a full `cargo deny check` additionally flags maintenance advisories.
