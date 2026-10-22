# Packet Tracer command execution evidence

Version v0.2.0. No command family in this release has an actual Packet Tracer execution certification. Local Windows app startup, clipboard/export smoke, XML/container decoding and synthetic configuration tests are different evidence categories. This matrix must not be read as an instruction to mark all profiles verified.

| Device profile | Documented context | Implemented relevant subset | Actual Packet Tracer execution |
|---|---|---|---|
| Catalyst 2960 class | Catalyst IOS 15.2(2)E | VLAN/access/trunk, STP, access security, bounded L2 EtherChannel, management SVI | Not tested |
| Catalyst 3560 class | Catalyst IOS 15.0(1/2)SE | Switching subset plus explicit image/context-gated routed port and IPv4 routing workflows | Not tested |
| Cisco 1941 / 2911 | Classic IOS references, principally 15M&T | Explicit interface/addressing, routes, selected routing protocol and ACL/NAT/DHCP syntax | Not tested |
| ISR4321 | IOS XE differs from classic IOS | Generic suggestions with unknown compatibility; no serial module assumed | Not tested |
| Generic/unknown IOS | Exact hardware/software unknown | Context-validated generic suggestions with warning; profile-restricted workflows remain blocked | Not tested |

Future execution evidence must record Packet Tracer version, exact device/model and interface inventory, initial configuration, commands accepted/rejected, saved running configuration and observed verification output on a test-only lab. Never run these tests on private exams or reuse a person's unsaved session. Only then may a specific family/profile become VerifiedPacketTracer. Family counts and actual generation/validator subsets are automatically reported in CISCO_COMMAND_COVERAGE.md.
