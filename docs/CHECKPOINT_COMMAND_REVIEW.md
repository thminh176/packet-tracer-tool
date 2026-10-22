# Conditional Access VLAN checkpoint command review

Reviewed 2026-10-22. Scope: public checkpoint command suggestions and their relationship to the strict Cisco command library. Only synthetic inputs were used; no Packet Tracer GUI, private activity data, network device execution, or push was performed.

## Finding and intended behavior

A complete `Access VLAN = 20` requirement on a known Switch physical interface identifies a configuration assignment even when the current switchport mode and VLAN activity are unknown. The selected checkpoint should therefore offer the exact line `switchport access vlan 20`, scoped to its interface. It must explain that operational effect depends on access mode and the VLAN being available/active. It must not silently add `switchport mode access`, create a VLAN, claim the port is operational, or mark the checkpoint verified.

This is a conditional requirement-based suggestion, not evidence of a satisfied prerequisite. Missing state may permit this narrow suggestion; explicit contradictory or malformed typed named evidence must still reject it. Unknown/router device identity, conflicting model/type evidence, unsafe or nonphysical interfaces, unsupported assessment checks, invalid numeric values, and reserved VLANs 1002–1005 remain blocked. The reserved-VLAN restriction preserves existing repository policy; it is not inferred solely from the command's numeric syntax range.

## Official Cisco evidence

The [Catalyst 2960-X IOS 15.2(5)E Layer 2 command reference](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst2960x/software/15-2_5_e/command_reference/b_1525e_consolidated_2960x_cr/b_1525e_consolidated_2960x_cr_chapter_0100.html), in its `switchport access vlan` section, documents interface configuration mode and VLAN IDs 1–4094. It states: “The port must be in access mode before the switchport access vlan command can take effect.” The `switchport mode` section separately explains that access and trunk settings persist while activation follows the selected mode. This supports offering the assignment with an explicit condition, without changing mode or asserting successful execution.

The [Catalyst 3560 Release 15.0(1)SE VLAN configuration guide](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst3560/software/release/15-0_1_se/configuration/guide/scg3560/swvlan.html) treats selecting access mode and assigning the access VLAN as separate configuration steps. These references establish IOS semantics; they do not establish that the suggestion was executed successfully in Packet Tracer or that any particular checkpoint currently passes.

## Source review

- `src/commands.rs`: `plan` resolves explicit requirements, checks identity and safe input, and routes recognized properties through typed generation; `generate` and `device_script` expose the suggestion publicly. The existing legacy Access VLAN formatter already describes the assignment line, but normal typed routing blocks incomplete context before reaching it.
- `src/cisco/mapping.rs`: `request` maps `Access VLAN` to `switch.access_vlan` and copies uniquely named typed metadata without inventing state. `parameter_conflict` rejects repeated differing values or metadata contradicting the explicit requirement. `classify` and `coverage` must report command readiness independently of verified status.
- `src/cisco/switch/mod.rs`: the typed `switch.access_vlan` spec requires interface, interface mode, VLAN, and active-VLAN context.
- `src/cisco/switch/handlers.rs`: typed validation enforces physical interfaces, numeric/choice constraints, access mode, and the reserved-VLAN restriction. Keep this workflow strict; do not weaken its catalog prerequisites to implement checkpoint suggestions.
- `src/cisco/validator.rs` and `tests/cisco_safety.rs`: existing validation requires sufficient mode/configuration evidence before treating the access VLAN requirement as passed. Generating a suggestion must not synthesize that evidence.

## Public regression coverage

`tests/checkpoint_commands.rs` covers the following observable behavior:

- Generic Switch, Catalyst 2960, and Catalyst 3560 requirements produce the exact assignment line in the individual command and selected-device script, with no mode change or VLAN creation.
- Partial compatible metadata permits the conditional suggestion while missing mode or active-VLAN evidence remains missing.
- VLAN bounds 1 and 4094 are accepted; out-of-range, reserved, sentinel, malformed, and injection values are rejected.
- Router/unknown/endpoint identity, conflicting router models, and invalid or nonphysical interfaces are rejected.
- Explicit typed mode/activity contradictions, malformed choices, conflicting interface/VLAN metadata, and differing repeated values are rejected rather than ignored.
- Unsupported/ambiguous/inventory assessment evidence remains blocked.
- Command readiness leaves the checkpoint serialization, evidence, current value, Unverified status, and verified score unchanged.
- Direct typed library generation remains nonexecutable until its actual mode/activity prerequisites are supplied, and contradictory mode remains rejected.

Validation: source and test review only. No cargo build or test was started, as instructed; the coordinator owns shared implementation and execution of `cargo test --test checkpoint_commands` plus the existing relevant regression suites. The new tests specify acceptance behavior and are not a claim that the coordinator's implementation has passed them.
