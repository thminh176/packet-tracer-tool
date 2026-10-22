# Security policy

The latest 0.1.x version is the supported release line. Treat all activity files and decoded XML as untrusted input. The app is offline and never executes Cisco commands; review generated configuration before pasting into a target device CLI.

Please report suspected vulnerabilities with GitHub's **Report a vulnerability** feature under this repository's Security tab when available. If unavailable, open a minimal issue asking for a private reporting channel, without exploit details, private files or secrets. Do not attach coursework or decoded answer data. Maintainers will triage reports and coordinate a fix; no guaranteed response SLA is offered.

Known limits include supported-schema gaps, inferred assessment selection semantics and ambiguous snapshot roles. These are documented in docs/COMPATIBILITY.md. The app does not bypass activity passwords, modify grades or replace Cisco's native assessment engine.

Never run generated IOS scripts in PowerShell, Command Prompt or a host shell. Copy one device's supported commands into that device's CLI in Packet Tracer.
