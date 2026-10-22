# Local real-file review, 2026-10-22

The user authorized testing their local networking collection. All testing used separate ignored copies with anonymous sample IDs. SHA-256 was checked when copying and all 32 source hashes were unchanged after inspection. Source names, local paths, activities, decoded XML, reports and configuration answers are excluded from Git and distribution.

## Decoder and extraction evidence

| Observation | Result |
|---|---:|
| Files / distinct source hashes | 32 / 32 |
| PKA files | 25 |
| PKT files | 7 |
| Successful decode and analysis | 32 |
| Selected candidate checkpoints | 1,052 |
| Automatically Unverified | 1,052 |
| CLI-ready checkpoint suggestions before context fix | 193 |
| CLI-ready checkpoint suggestions after context fix | 308 |
| Endpoint GUI suggestions before / after | 67 / 140 |

The seven standalone PKT samples contain no selected assessment tree. Zero checkpoints on those samples is expected, not a fabricated grading result. All real activity snapshot roles remain ambiguous, so none of these 1,052 checks gains an automatic pass or fail. The selected counts and points still use the documented inferred selection metadata; these are not certified native grades.

One real sample matches the supplied structural baseline: **32 checkpoints, 100 candidate points and seven assessed devices**. Before the correction, no suggestions were generated. Afterward it produces nine CLI and 18 GUI suggestions, retaining five unsupported numeric requirements (three interface state checks, one IPv6 routing flag and one startup configuration check). The file was decoded and enumerated; no command was execution-tested against its devices and no successful completion is claimed. No real 126-checkpoint activity was certified.

## Context correction

Some IPv6 assessment branches label their child leaves simply `IP Address` and `Prefix Length`. The inspector now uses the explicit IPv6 address branch to interpret those labels, retaining original labels and paths. It pairs only uniquely selected explicit siblings in the same device/interface/parent group. Missing, duplicate or contradictory siblings remain unsupported. The prefix checkpoint gets its own command action, and duplicate address/prefix plans collapse in a device script.

`Link Local` maps to the bounded link-local workflow; IPv6 endpoint gateways use Packet Tracer GUI instructions. Numeric state and save encodings remain unsupported. Multiple explicit global IPv6 addresses may coexist; conflicting prefixes for the same address are omitted from batch scripts. See [Cisco IPv6 command reference](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/ipv6/command/ipv6-cr-book/ipv6-i1.html).

## Reproduction and privacy

`cargo run --release --locked --example compatibility_batch -- <directory-of-permitted-copies>` emits per-sample counts and reasons. Keep output local: custom property labels and diagnostics may be private. The local source-to-sample manifest and JSON evidence stay in ignored folders. `generate_validation_lab` creates an original synthetic two-router command set using the same checkpoint API; it is a test lab, not an activity answer or simulator certification.

Current limitations include unsupported complex schemas, incomplete prerequisites and unknown numeric enums. Documentation-backed command generation, actual CLI acceptance, network behavior and assessment verification are distinct evidence categories. This local review does not certify universal file compatibility or all 46 command families on real devices.
