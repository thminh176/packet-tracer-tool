# Cisco device compatibility

Profiles identify explicit model strings only: Catalyst 2960/2960-24TT, Catalyst 3560/3560-24PS, Cisco 1941, Cisco 2911, ISR4321. Device names such as R1 or SW1 never establish a hardware model. A generic Router/Switch type remains a generic profile. Conflicting model metadata remains unknown.

The support states are VerifiedPacketTracer, DocumentedNotTested, Unsupported and Unknown. No profile is currently marked execution-verified in Packet Tracer. A Cisco configuration reference supports documented syntax for its named hardware/software; it does not prove the simulator implements that feature.

| Profile | Baseline software context | Safety constraints |
|---|---|---|
| 2960 class | Catalyst IOS 15.2(2)E sources | Treat as Layer 2 profile; no router-protocol, routed physical port or IPv6 routing repair. Management SVI is distinct from a physical switch port. Real platform/image exceptions do not establish simulated support. |
| 3560 class | Catalyst IOS 15.0(1/2)SE sources | Separate Layer 2 physical port, routed physical port and SVI. Route protocols remain feature-image dependent; actual PT testing absent. |
| 1941 / 2911 | Classic IOS 15M&T syntax research | Modular serial hardware is not assumed from model name. Reject Layer 2 switchport operations. Feature/image/licensing and interface availability remain unknown. |
| ISR4321 | IOS XE; not interchangeable with classic IOS | Do not promote generic classic IOS research to confirmed IOS XE/PT support. Generic suggestions carry explicit unknown compatibility warning. |
| Unknown / generic | No exact hardware evidence | Safe syntax suggestions may be offered with compatibility warning; never shown as simulated execution verified. |

Sources: [2960 trunks](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/b_1522e_2960_2960c_2960s_2960sf_2960p_cg_chapter_01101.html), [3560 interfaces](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_2_se/configuration/guide/scg3560/swint.html), and per-family registry references. See dedicated Router/Switch research and PT execution matrix for finer limits.
