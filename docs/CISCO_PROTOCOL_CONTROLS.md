# NTP, RIP/EIGRP and scoped assessment controls

Independent research and implementation, 2026-10-23. Current development catalog: **66 bounded families** (12 common, 23 switching, 31 routing). The number counts workflows, not every CLI keyword, address combination, or positive/negative variant. NTP IPv4 and IPv6 intentionally share a family.

## Official references and implementation

| Workflow | Mode and explicit inputs | Boundaries and evidence |
|---|---|---|
| NTP server | Global `ntp server <address> [version <2/3/4>] [prefer]` | IPv4 remote unicast or unscoped IPv6 global/ULA. IPv6 requires explicitly supplied version 4; Catalyst IPv6 support is blocked in this subset. Preference is enable-only; no source, authentication key, local clock or master role is inferred. [IOS NTP reference](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/bsm/command/bsm-cr-book/bsm-cr-n1.html). |
| NTP source | Global `ntp source <interface>` | Requires operator-confirmed address/return-path context. A Layer 2 switch uses a management SVI workflow. Per-server overrides and synchronization are separate. Same NTP reference. |
| RIP version | `router rip`, then `version <1/2>` | No network selection or summarization policy is added. Interface send/receive overrides remain outside the subset. [IOS RIP reference](https://www.cisco.com/c/en/us/td/docs/ios/iproute_rip/command/reference/irr_book/irr_rip.html). |
| RIP summarization | `router rip`, then `[no] auto-summary` | Explicit true/false policy; no undocumented numeric Boolean interpretation. Changes can affect discontiguous networks. Same RIP reference. |
| RIP passive | `router rip`, then `[no] passive-interface <interface>` | A specific interface is required; default/all-interface policy is not inferred. Outgoing updates and receiving behavior must be distinguished. [Protocol-independent reference](https://www.cisco.com/c/en/us/td/docs/ios/iproute_pi/command/reference/iri_book/iri_pi1.html). |
| Classic IPv4 EIGRP passive | `router eigrp <as_number>`, then `[no] passive-interface <interface>` | Explicit numeric AS and participating interface; named/IPv6 address families are separate. Suppression affects Hellos/adjacencies, not just outgoing RIP-style updates. [EIGRP passive reference](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/iproute_eigrp/command/ire-cr-book/ire-i1.html). |
| Classic EIGRP router ID | `router eigrp <as_number>`, then `eigrp router-id <dotted-id>` | A unique identifier, not an interface assignment. All-zero/all-ones IDs are rejected. No process removal/restart is generated. [EIGRP ID reference](https://www.cisco.com/c/en/us/td/docs/ios/iproute_eigrp/command/reference/ire_book/ire_a1.html). |
| Catalyst LLDP | Global, port transmit/receive, and timer workflows | Explicit states and exact identified profiles; see [dedicated sources and limits](CISCO_LLDP_RESEARCH.md). Numeric Boolean enums remain unsupported. |

## Assessment interpretation

Explicit aliases and generic leaves such as `Version` or `Router ID` use one exact protocol category below a uniquely identified device. Device names and numeric tree IDs do not establish protocol/process identity. Mixed protocols, OSPFv3/IPv6 ambiguity and missing context stay unsupported or incomplete. Raw labels and assessment paths remain intact. These mappings have synthetic tests, not universal PKA schema certification.

Selected process/AS evidence must still be explicit. A category ending in `OSPF / 1 / Router ID` does not supply process 1. Missing fields can transfer to the bilingual Library without loading synthetic example values into the requirement.

## Configured-state comparison

Eight additional declared comparator subsets cover configured OSPF router ID/reference bandwidth/passive state, RIP version/summarization/passive state and EIGRP ID/passive state. Comparisons require unique current snapshot/device ownership, complete compatible requirements and one explicit value in the correct routing process. Missing, duplicate, invalid, conflicting or default-only evidence remains Unverified. Interface aliases are normalized only through the explicit interface parser. Configured router ID equality does not certify an effective ID or adjacency state.

NTP synchronization, LLDP neighbor discovery and LLDP saved-state grading are not evaluated by these additions. There are **26 declared semantic subsets** across the full catalog and **zero actual Packet Tracer execution-certified families**. Cisco IOS references are not simulated-device certification.

## Device scripts

Separate RIP version and RIP network requirements share a version conflict check. Independent passive ports and NTP server addresses can coexist. Conflicting completed LLDP port states are omitted together, and globally disabling CDP/LLDP cannot be combined with interface enablement. Canonical interface identities deduplicate equivalent blocks without changing individual suggestions. These safeguards do not discover arbitrary existing device policy or execute commands.

## Follow-up priorities

Remaining useful work includes broader ACL/NAT/PAT semantics, protocol interface timers/cost, additional evidence-backed PKA mappings and simulator execution tests. VTP mutation, redistribution, authentication changes and unsupported images require their own context and research; a command name alone does not activate support.
