# Public command generation safety review

Review date: 2026-10-22. Baseline implementation: `3eeca7ab3c3d9c5148033070edc9f9b743a74fcf`.

This review adds synthetic acceptance regressions at `commands::generate`,
`commands::device_script`, and `cisco::generate`. It reads shared command,
mapping, basic IOS, and profile implementation without changing implementation.
Router and switch internals have separate reviewers. No Packet Tracer session,
device command execution, private course content, PDFs, UI change, push, or
release was used or performed. Documentation establishes IOS syntax and scope;
it does not establish execution support in any Packet Tracer version.

## Reproduction

Run `cargo test -j 2 --test cisco_acceptance -- --nocapture`.
The suite contains 16 tests. Assertions describe the required safe behavior;
failures must be fixed in shared implementation by the coordinator rather than
hidden with ignored tests or changed expectations.

On the baseline implementation, the final scoped run completed with **9 passed,
7 failed, 0 ignored**. All seven failing cases below reproduce generated unsafe
or contradictory output; this report does not claim a green local acceptance
gate. The initial run also exposed an overly strict test comparing a redundant
`exit` before `end`; that assertion was corrected to compare effective scope and
commands, and the corrected positive case passes.

## Findings requiring shared fixes

| Regression | Minimal synthetic input | Baseline defect and consequence |
| --- | --- | --- |
| `hostname_obeys_documented_terminal_character_rule_at_both_apis` | Typed hostname `Branch-` | `basic.hostname` checks the first character and interior alphabet but not the final character, accepting a name outside the documented grammar. |
| `checkpoint_vty_cannot_bypass_typed_authentication_prerequisites` | Checkpoint `VTY SSH = 0 4`, no evidence | Legacy fallback emits `login local` and `transport input ssh` without the existing user, SSH, and AAA context required by `basic.vty_ssh`. The public entry points enforce different safety policies for the same management operation. |
| `cidr_address_cannot_silently_override_explicit_sibling_mask` | `192.0.2.1/24` plus `commands:ipv4-mask=255.255.0.0` | CIDR/dash conversion returns before sibling-mask reconciliation, silently choosing /24 over explicit /16 evidence. |
| `complete_route_cannot_silently_override_explicit_next_hop` | Route `10.0.0.0 255.0.0.0 192.0.2.1` plus `commands:route-next-hop=192.0.2.2` | Next-hop evidence is considered only for incomplete routes, so a complete requirement suppresses contradictory evidence. |
| `named_parameter_evidence_must_agree_with_requirement` | Hostname `Branch-1` plus `commands:parameter:hostname=Branch-2` | Mapping skips named evidence for fields already populated from the requirement, losing contradictions. |
| `interface_aliases_cannot_hide_conflicting_batch_addresses` | Different primary addresses on `GigabitEthernet0/0` and `Gi0/0` | Conflict keys use the rendered interface spelling; aliases for the same interface evade conflict detection and both primary-address commands are emitted. |
| `description_aliases_cannot_hide_conflicting_batch_values` | `Description = WAN` and `Interface Description = LAN` on the same interface | Property spellings map to one typed family but retain different batch conflict keys, emitting contradictory scalar settings. |

The contradiction rules above are application safety requirements inferred from
the explicit-evidence contract, not claims that IOS cannot accept sequential
replacement commands. Where multiple contradictory requirements exist, this tool
must omit the disputed configuration and explain the conflict. Distinct static
routes remain legitimate ECMP/floating routes; this review does not demand
rejecting those combinations.

## Positive and boundary coverage

The other acceptance cases require complete console/VTY parameters and line
scope, reject banner delimiters and control characters, isolate batch conflicts
from unaffected settings and devices, preserve the unsupported 2960 routing
gate, reject unknown families/parameters and target injection, compare complete
IPv4 output between public APIs, allow unique named VLAN evidence with explicit
VTP context and reject conflicting names, accept duplicate identical evidence without
duplicate configuration, and keep explicit save operations in EXEC scope after
configuration. Ready plans must retain the warning that Packet Tracer execution
has not been tested; no test promotes a profile to `VerifiedPacketTracer`.

## Primary Cisco evidence

- [IOS hostname command reference](https://www.cisco.com/E-Learning/bulk/public/tac/cim/cib/using_cisco_ios_software/cmdrefs/hostname.htm): hostname is a global configuration command; the documented grammar begins with a letter, ends with a letter or digit, permits interior letters/digits/hyphens, and limits length to 63 characters.
- [IOS fundamentals, A through B](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/fundamentals/command/cf_command_ref/A_through_B.html): `banner motd` uses a matching delimiter around the message in global configuration mode; the delimiter must not appear in the message.
- [Verify Login Behaviour with and without AAA](https://www.cisco.com/c/en/us/support/docs/ios-nx-os-software/authentication-authorization-accounting-aaa/200173-Verify-AAA-behaviour-when-login-local.html): local login and AAA state change authentication behavior; the local user database is an explicit prerequisite in Cisco's example. This supports retaining authentication context rather than automatically changing AAA state.
- [Configuring IPv4 Addresses, IOS 15.0M](https://www.cisco.com/en/US/docs/ios-xml/ios/ipaddr_ipv4/configuration/15-0m/config-ipv4-addr.html): interface selection precedes `ip address address mask`; enabling the interface is a separate operation. An address-only requirement does not authorize an additional administrative-state change.
- [Managing Configuration Files, IOS 15M&T](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/config-mgmt/configuration/15mt/config-mgmt-15-mt-book/cm-config-files.html): saving running configuration is an EXEC operation. The suite checks command scope and ordering, not successful completion of any interactive device prompt.

## Remaining verification

Coordinator must apply shared fixes and run the acceptance suite with the
existing command, router, switch, catalog, safety, and inspector suites against
the integrated tree. This worker's tests establish only local generated-output
behavior; actual Packet Tracer execution and platform/software compatibility
remain unverified.
