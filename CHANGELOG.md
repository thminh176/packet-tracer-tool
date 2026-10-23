# Changelog

## 0.2.2-dev — Cisco service coverage (unreleased)

- Add explicit DHCP relay, DCE serial clock, synchronous PPP, global/interface CDP, and OSPFv2 router ID/passive/reference-bandwidth workflows; 56 bounded families in total.
- Require explicit process IDs, hardware/cable/peer assertions and correct configuration modes. Additional checkpoint aliases are synthetically tested; automatic validation for these eight new families remains unsupported.
- Preserve multiple DHCP relay servers; transfer incomplete supported requirements into the bilingual Library without invented context.
- Add an original research matrix covering 14 expansion topics with 23 Cisco source URLs, separating classic IOS, IOS XE and Packet Tracer execution evidence.

## 0.2.1 — IPv6 assessment and dot1Q context fixes

- Map the explicit IPv6 Unicast Routing Boolean to global IOS configuration; reject unknown numeric enums and incorrect device/interface scope.
- Add distinct interface IPv6 enable and SLAAC families, with enable-only suggestions and scoped explicit-configuration comparison.
- Complete EthernetEncapProcess from a unique selected VLAN requirement in the same interface scope; otherwise offer explicit parameter completion in the bilingual Command Library. Never infer VLAN IDs from subinterface suffixes.
- Detect conflicting encapsulation requests in device scripts and compare only uniquely identified, explicit current dot1Q configurations.
- Registry now contains 48 bounded families. No additional real Packet Tracer execution compatibility is claimed.

## 0.2.0 — Cisco Command Coverage

- Add a modular typed registry of 46 bounded command families, explicit device profiles, configuration contexts, prerequisite validation, Cisco source links and read-only verification suggestions.
- Add the bilingual Cisco Command Library (Ctrl+7), searchable by topic/device/profile, parameter forms and gated command copying.
- Remove implicit OSPF process/area and generic classful inference; validate complete hyphen structures, routing targets and unique saved-device ownership.
- Parse IOS modes and preserve ACL order; add bounded switching validators and separate command readiness from verification/score.
- Resolve explicit IPv6 siblings and dot1Q/VLAN branches without mining interface suffixes or ambiguous answer snapshots. Conditional scalar VLAN suggestions preserve missing current-state evidence.
- Keep all Packet Tracer execution support untested unless independently demonstrated. See coverage, compatibility and test evidence; unsupported families remain explicit.

## 0.1.1 — Network & Routing Fixes

- **IPv4 static route hyphenated encoding fix**: Parse and normalize Packet Tracer's internal hyphenated static route representation (`destination-prefix-nexthop_or_interface-unknown-distance`, such as `172.16.128.0-17-172.18.192.1-0-1` and `0.0.0.0-0-Serial0/1/0-0-1`). Correctly generates Cisco IOS `ip route` configuration with subnet masks, next-hop IP or exit interfaces, and administrative distance.
- **OSPF wildcard and area handling**: Parse and normalize OSPF network statements across multiple token encodings, invert dotted subnet masks to wildcard masks, handle area notation (normalizing `0.0.0.0` to `0`), and generate structured `router ospf <pid>` context and `network <net> <wildcard> area <area>` commands.
- **CIDR prefix normalization**: Introduce `NormalizedNetwork` engine to canonicalize CIDR prefix (`/prefix` and `-prefix`), dotted subnet mask, and wildcard mask inputs into canonical network addresses, clamping non-zero host bits with confidence levels and human-readable diagnostic explanations.
- **Improved Auto Check diagnostics**: Enhance assessment verification against device running-config with multi-line matching for static routes and OSPF networks. Produce specific diagnostic messages when configurations are absent (e.g. "No IPv4 static routes configured on device") or when assessment checkpoints lack required parameters.
- **IPv6 static route support**: Parse hyphenated IPv6 static route strings (`prefix-prefixlen-nexthop_or_interface-unknown-distance` and default routes `::-0-...`) and generate corresponding `ipv6 route` commands.
- **Comprehensive network test suite**: Added `tests/network_values.rs` with 12 automated unit and integration tests covering CIDR equivalence, wildcard calculations, hyphenated static routes, default routes, exit interfaces, floating distance, and OSPF statements.

## 0.1.0 — Initial release

- Offline native Rust Windows inspector for supported modern PKA/PKT containers and decoded XML.
- Recursive assessment candidates, device/interface grouping, evidence and Passed/Failed/Unverified states.
- Conservative IOS suggestions and endpoint GUI instructions; supported explicit families only.
- Fixed duplicate IP/mask commands and grouped device configuration contexts; omitted checks summarized separately.
- CSV/Markdown/CLI exports with source-preserving atomic writes.
- Extensible English/Vietnamese JSON catalogs, persistent preferences, background parsing and virtualized views.
- Bounded native decoder, synthetic fixture and regression tests, Windows CI and tagged release packaging.

Known limitations: inferred selection metadata, unlabelled snapshot roles, incomplete complex validators, unknown/sentinel values and older container variants. No live integration, native grading parity or universal PKA support is claimed.
