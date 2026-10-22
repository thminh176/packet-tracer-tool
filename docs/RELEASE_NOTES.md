# Packet Tracer Tool v0.1.1 — Network & Routing Fixes

An independent, bilingual desktop companion for learning, inspecting, validating and troubleshooting Packet Tracer labs. Native offline Windows x86_64 Rust application; no Python required.

## Overview

Version 0.1.1 introduces a dedicated network normalization and routing engine, fixing hyphenated static route encoding issues, adding OSPF wildcard and area handling, standardizing CIDR prefix representations, and significantly improving Auto Check diagnostics against device `running-config`.

## Key Improvements

### 1. IPv4 Static Route Hyphenated Encoding Fix
- **Hyphenated Encoding Support**: Correctly parses Packet Tracer's internal hyphen-delimited format: `destination-prefix-nexthop_or_interface-unknown-distance` (e.g. `172.16.128.0-17-172.18.192.1-0-1`).
- **Complete Command Emission**: Automatically maps CIDR prefix lengths to dotted-decimal subnet masks, identifies whether the target is an IP next-hop or an exit interface (e.g. `Serial0/1/0`, `GigabitEthernet0/0/0`), and emits standard Cisco IOS `ip route <network> <mask> <next-hop|interface> [distance]` commands.
- **Default Route Handling**: Full support for default routes encoded as `0.0.0.0-0-<target>-0-1` (`ip route 0.0.0.0 0.0.0.0 <target>`).
- **Floating Administrative Distance**: Accurately preserves and emits non-default administrative distances (e.g. `10.0.0.0 255.0.0.0 192.168.1.1 5`).
- **Sibling Evidence Context**: Automatically resolves missing next-hop or exit-interface fields when paired with unique sibling assessment evidence (`commands:route-next-hop=`).
- **IPv6 Static Routes**: Adds parsing and generation for IPv6 static routes (`prefix-prefixlen-nexthop_or_interface-unknown-distance` and default routes `::-0-...`).

### 2. OSPF Wildcard & Area Handling
- **Multi-Token OSPF Parsing**: Supports both 3-token (`<network> <wildcard> <area>`) and 4-token (`<process_id> <network> <wildcard> <area>`) statements, as well as hyphenated CIDR notations.
- **Wildcard Conversion**: Automatically derives and validates contiguous wildcard masks from CIDR prefixes and dotted subnet masks.
- **Area Normalization**: Correctly normalizes dotted-decimal area notation (e.g. `0.0.0.0`) to canonical integer area notation (`0`).
- **Context Generation**: Emits proper Cisco IOS context hierarchies with `router ospf <process_id>` and `network <network> <wildcard> area <area>`.

### 3. CIDR Prefix Normalization Engine
- **Canonical Network Representation**: Introduces `NormalizedNetwork` to standardize IPv4 network representations across `/prefix`, `-prefix`, dotted subnet mask, and wildcard mask inputs.
- **Host-Bit Clamping**: Zeros host bits on subnet boundaries, providing explicit confidence classifications (`High`, `Medium`, `Low`) and transparent diagnostic explanations.

### 4. Improved Auto Check Diagnostics
- **Running-Config Evaluation**: Implements multi-line running-config matching for static routes and OSPF networks, moving beyond single-observation comparisons.
- **Clear Failure Explanations**: When expected routes or OSPF statements are missing from a device's running-config, Auto Check reports explicit, context-rich explanations (e.g. `"No IPv4 static routes configured on device"`, `"No OSPF network statements configured on device"`).
- **Missing Checkpoint Parameter Detection**: If an assessment checkpoint lacks essential routing parameters (such as next-hop or exit-interface), Auto Check explains precisely which fields are required.

## Packaging and CI Configuration Review

- **Workflow Review (`.github/workflows/ci.yml`, `.github/workflows/release.yml`)**:
  - CI enforces `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, and dependency auditing (`cargo-audit`, `cargo-deny`).
  - `release.yml` triggers on version tags (`v*`) and builds draft GitHub releases with artifact checksum verification.
- **Packaging Scripts (`scripts/build-release.ps1`, `scripts/verify-package.ps1`)**:
  - Packaging generates `dist/packet-tracer-tool-v<version>-windows-x64.zip` and `.sha256` digest files.
  - Package integrity is verified against strict criteria: exactly one executable, valid PE x86_64 header, license notices, and absence of forbidden activity/key files.
  - Prior to publishing a tagged release, `Cargo.toml` must align with the release tag version, and any clippy/formatting warnings in CI must be resolved.
- **Release Safety**: No git tags have been pushed and no GitHub releases have been published, conforming to release governance protocols.

## Test Verification

- **Automated Test Results**: **97 passed, 0 failed, 1 opt-in test ignored** in the main application suite, including 12 dedicated tests in `tests/network_values.rs` and 4 new UI rendering and network detail tests.
- **Codec Suite**: **18 passed, 0 failed** in the vendored codec suite. Total: **115 passing automated tests**.
- **Performance & Safety**: Sub-millisecond parsing and validation on synthetic fixtures, memory bounded, read-only and offline guarantees preserved.

## Installation

Download `packet-tracer-tool-v0.1.1-windows-x64.zip` and its `.sha256` checksum file from [Releases](https://github.com/thminh176/packet-tracer-tool/releases). Verify the SHA256 digest, extract the archive, and launch `pka-lab-inspector.exe`. The application runs 100% offline and requires no Python or Cisco software.

Generated commands are designed for Cisco IOS CLI in Packet Tracer (target device CLI tab), not PowerShell or host command prompts.
