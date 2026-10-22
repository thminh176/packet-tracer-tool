# Cisco command research for v0.2

Research date: 2026-10-22. Independently authored implementation notes; no Cisco manual is mirrored. See dedicated Switch/Router research for each implemented registry entry. Each runtime CommandSpec includes syntax, mode, fields, prerequisites, negation description when known, verification commands, source URL and software family.

## Shared foundations and carryover repairs

- Classic IOS 15M&T [OSPF configuration](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/iproute_ospf/configuration/15-mt/iro-15-mt-book/iro-cfg.html): OSPF statements are scoped to an explicit local process. Examples using process 1 are examples, not activity requirements. Missing process/area stays incomplete.
- Classic IOS 15M&T [basic routing](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/iproute_pi/configuration/15-mt/iri-15-mt-book/iri-iprouting.html): static routes require a destination/mask and forwarding target; administrative distance affects route preference. An unspecified explicit CLI distance has the documented default 1; this does not authorize reconstructing a missing target or network mask.
- Catalyst 2960 IOS 15.2(2)E [VLAN trunks](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_01101.html): access/trunk mode and native/allowed VLAN choices have distinct meanings. Do not derive mode from an unrelated numeric assessment value.
- Catalyst 3560 IOS 15.0(2)SE [interface characteristics](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_2_se/configuration/guide/scg3560/swint.html) and [unicast routing](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swiprout.html): routed physical ports differ from SVIs and Layer 2 ports. A supported routed-port workflow must explicitly account for Layer 3 mode and routing prerequisites.

## Evidence rules

Cisco's hardware documentation establishes documented syntax for that release, not execution support in a Packet Tracer simulation. 1941/2911 classic IOS and ISR4321 IOS XE are distinct profiles. A broad IOS document is not proof of the exact simulated feature set. No real device/Packet Tracer execution result is claimed by these notes. Unknown features and missing parameters remain visible, not assumed complete.

The new saved-config parser preserves global/interface/router/VLAN/ACL/DHCP/line context and ACL order. It conservatively rejects opaque multiline constructs for semantic verification. Read-only show commands are suggestions, never run automatically. Removal forms are reference metadata; erase/delete/reload/reset or bulk removal repairs are not generated.
