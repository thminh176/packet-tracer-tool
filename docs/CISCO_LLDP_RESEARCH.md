# Bounded Catalyst LLDP research

Reviewed official Cisco.com documentation on 2026-10-23 before implementation:

- [Catalyst 2960 family, IOS 15.2(2)E: Configuring LLDP, LLDP-MED, and Wired Location Service](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960/software/release/15-2_2_e/configuration/guide/b_1522e_2960_2960c_2960s_2960sf_2960p_cg/m_1522e_int_lldp_2960s_cg.html), especially Restrictions, Default LLDP Configuration, Enabling LLDP, and Configuring LLDP Characteristics.
- [Catalyst 3560, IOS 15.0(1)SE: Configuring LLDP, LLDP-MED, and Wired Location Service](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swlldp.html), especially Configuration Guidelines, Enabling LLDP, and Configuring LLDP Characteristics.

These are independently implemented command facts, with links for review; no manual or restrictive implementation is copied into this repository. No private fixtures or examination files were accessed.

## Documented command facts

Both guides distinguish global activation (`lldp run` / `no lldp run`) from interface transmission (`lldp transmit` / `no lldp transmit`) and reception (`lldp receive` / `no lldp receive`). They describe a globally disabled default and an enable workflow that configures both interface directions separately. The 3560 guidelines say tunnel ports automatically disable LLDP. These facts support keeping each state explicit rather than treating one Boolean as all three states. See the Enabling LLDP and Configuration Guidelines sections linked above.

Both guides' Configuring LLDP Characteristics sections give the same global timer limits, all in seconds:

| Command | Accepted interval | Documented default | Meaning |
| --- | --- | --- | --- |
| `lldp holdtime <holdtime>` | 0–65535 | 120 | Advertised retention duration at a receiving neighbor |
| `lldp reinit <reinit>` | 2–5 | 2 | Interface LLDP initialization delay |
| `lldp timer <timer>` | 5–65534 | 30 | Advertisement update interval |

The guides permit configuring these characteristics separately and in any order. Their no forms restore defaults. The implementation accepts all three explicit timer values and emits holdtime, reinit, then timer; it neither assumes defaults nor adds an undocumented relationship between these ranges. It does not implement default-reset commands.

## Engine contract and deliberate limits

| Family | Required parameters | Generated mode and behavior |
| --- | --- | --- |
| `switch.lldp.global` | `enabled` | Global: explicitly enable or disable the LLDP process |
| `switch.lldp.interface` | `interface`, `interface_mode`, `global_enabled`, `transmit`, `receive` | Interface: independently set transmit, then receive |
| `switch.lldp.timers` | `holdtime`, `reinit`, `timer` | Global: configure the three bounded timers |

`enabled`, `global_enabled`, `transmit`, and `receive` accept only the literal strings `true` and `false`. Numeric assessment enum meanings are not proven and are not accepted by this generator. Labels and field descriptions are English/Vietnamese; parameter identities and IOS command syntax are language independent.

The supported profiles are identified `Catalyst2960` and `Catalyst3560`, tied to the documented releases above. Routers, generic switch profiles and unknown profiles are rejected; model selection does not prove the installed image supports LLDP. The interface subset requires caller-confirmed static `access` or `trunk` mode and the shared physical Ethernet port validator. Subinterfaces, loopbacks, SVIs, port channels, tunnels, serial interfaces, interface ranges and routed ports are excluded from this workflow. This conservative subset is an engine boundary, not a claim that Cisco excludes every other interface type. Physical interface spelling does not prove a port exists on a particular device.

Enabling transmit or receive requires explicit `global_enabled=true` evidence supplied by the caller. Missing evidence fails closed, and `global_enabled=false` permits only disabling both directions. Interface commands never silently emit `lldp run`; global commands never invent per-interface state. Timer configuration is permitted independently of process state and never activates LLDP. This module validates supplied context and cannot inspect running equipment or coordinate independently submitted requests.

Unknown keys, empty required fields, unbounded text, control characters, padded values and invalid choices/ranges fail closed through the switch entry point as well as the top-level plan generator. Explicit per-port settings remain separate from global state in the command modes. Assessment mapping and observation validation live outside this module and must preserve these same distinctions. These catalog entries set `semantic_validation=false` because typed generation checks do not establish a saved-configuration comparator.

LLDP-MED/TLV selection, network policy, location services, discovery output parsing, neighbor identity and live operational reachability are outside this change. The 2960 Restrictions section also cautions against enabling LLDP on ports using CDP for power negotiation; callers must verify that deployment condition before enabling, since this engine has no power-negotiation observation. Suggested checks are `show lldp`, `show lldp interface <interface>` and the appropriate running configuration; they are verification guidance, not collected device evidence.

## Evidence limits and validation

Support remains `DocumentedNotTested`, never `VerifiedPacketTracer`. No real Packet Tracer execution, switch execution, packet capture or compatibility verification was performed. IOS documentation does not establish LLDP support in every Packet Tracer release, simulated model or software image.

Synthetic typed-request tests in `tests/cisco_lldp.rs` cover valid and disabled states, independent directions, global prerequisite, missing fields, all device profiles, physical interface restrictions, timer endpoints and invalid values, mode transitions, unknown fields, injection and rejection of numeric Boolean enums. The existing switch catalog test is updated from 20 to 23 families and distinguishes LLDP's unavailable saved-configuration comparison. Formatting, focused test and Clippy command results are reported in the worker completion receipt.
