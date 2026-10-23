# Packet Tracer Tool 0.2.2-dev - Cisco services expansion (unreleased)

Adds eight bounded workflows: DHCP relay, serial DCE clock, serial PPP, global and interface CDP, OSPFv2 router ID, passive interface and reference bandwidth. The catalog now has 56 families. Required process, interface, hardware and peer context remain explicit; unknown numeric schema flags are not generalized.

Generation and synthetic mapping tests are separate from saved-configuration validation. These eight families do not add grading validators or real Packet Tracer execution certification. This branch is a development build, not a published stable release.

Research, sources and next priorities: CISCO_EXPANSION_RESEARCH.md. Test evidence: TEST_RESULTS.md.

---
# Packet Tracer Tool v0.2.1 - IPv6 and dot1Q checkpoint fixes

- Explicit IPv6 Unicast Routing = 1 now generates ipv6 unicast-routing in global configuration mode. Disabling forwarding carries an impact warning.
- Distinct interface ipv6 enable and ipv6 address autoconfig suggestions require explicit enablement and correct interface context. No hidden address, default route or no shutdown is added.
- EthernetEncapProcess uses a unique selected dot1Q VLAN in the same port scope, or transfers to the bilingual Command Library for explicit completion. Subinterface suffixes are never VLAN evidence.
- Conflicting dot1Q requirements are omitted from device scripts. Verification requires a unique current snapshot/device and explicit scoped configuration.
- 48 bounded families; no new live Packet Tracer execution certification. See TEST_RESULTS.md and V0.2.1_COMMAND_RESEARCH.md.

This describes a local candidate and does not establish GitHub publication.

---
# Packet Tracer Tool v0.2.0 — Cisco Command Coverage

Independent offline Windows x64 Rust / egui companion for Packet Tracer labs. Not affiliated with or endorsed by Cisco.

## Included

- Typed catalog of 46 bounded command families: 8 shared fundamentals/security, 20 switching and 18 router families. Each has parameters, modes, prerequisites, Cisco source links and read-only verification suggestions.
- Device profiles for 2960/3560, 1941/2911 and ISR4321, plus clearly warned generic/unknown profiles. Documented syntax and actual Packet Tracer execution evidence are separate.
- Native English/Vietnamese Command Library (Ctrl+7): search, device/topic/profile filters, explicit parameter forms, synthetic learning examples and gated copy actions.
- No implicit OSPF process 1, no classful inference for generic network values, strict bounded hyphen structures and complete forwarding targets. Equivalent OSPF decimal/dotted areas compare numerically.
- Route absence requires a unique current snapshot and uniquely owned device configuration. ECMP/floating route statements remain distinct. Contradictory device identities and incompatible gateway workflows suppress suggestions.
- Mode-aware saved-configuration parsing, explicit scalar switch validators and separate checkpoint command classification / command coverage / automatic verification metrics.
- Existing read-only PKA/PKT/XML inspection, evidence, manual checklist separation, bilingual preferences and protected reports remain available.
- Conditional Access/Native/Allowed VLAN assignments and VLAN naming use explicit requirement context; dot1Q uses the explicit VLAN ID, never the subinterface suffix. Current-state verification and strict library prerequisites remain separate.
- OSPF `RouteN` leaves no longer enter static-route generation or comparison. The table identifies the protocol explicitly; incomplete OSPF requirements can transfer their network/area into the Library for explicit process selection.

## Evidence and limitations

Automated tests and Windows GUI evidence are recorded in docs/TEST_RESULTS.md; generated catalog counts and timings are in docs/CISCO_COMMAND_COVERAGE.md. **No new command family has been execution-certified against an actual Packet Tracer device.** Official Cisco documentation and synthetic tests are not simulator certification. The application is not Cisco's grader.

The 14 declared validator subsets cover particular explicit fields, not complete protocol behavior or every parameter/dependency of those families. No convergence, reachability, forwarding or live activity score is proven. Unknown snapshot roles, defaults, unsupported/opaque configurations and incomplete inputs remain Unverified.

New typed routing/service subsets do not cover OSPFv3, named EIGRP, extended/named/IPv6 ACL semantics, dynamic NAT/PAT validation, PPP/CHAP/PAP, VTP authoring, DHCP snooping, DAI, LLDP, storm control, NTP or arbitrary removal workflows. Some earlier explicit CLI grammars remain available but have no new complete semantic validation claim. Library examples are synthetic and must never be treated as lab answers. Some diagnostics/prerequisite prose and report headings remain English.

## Install

Download packet-tracer-tool-v0.2.0-windows-x64.zip and its .sha256, verify the checksum, extract the complete folder and launch pka-lab-inspector.exe. The EXE is unsigned and may require the official Microsoft Visual C++ x64 runtime. Keep included licenses/notices. Paste IOS suggestions only into the intended Packet Tracer device CLI; the application never executes them.

v0.1.1 remains available as a fallback. No older release tags are replaced. No private activities, answer networks, Cisco binaries or logos are included.
