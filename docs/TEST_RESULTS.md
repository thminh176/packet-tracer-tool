# Verification and known limits

Test date: 2026-10-22. Target: Windows x86_64, Rust 1.99.1, native eframe/egui 0.33.3 OpenGL backend.

## Automated checks

- `cargo fmt --all -- --check` passes.
- `cargo clippy --locked --all-targets -- -D warnings` passes.
- Application tests cover decoder corruption, binary/XML prefix collisions, XML limits/entities/DTD/duplicate fields, recursive extraction, selected versus inventory nodes, fractional and invalid weights, device and interface identity ambiguity, IPv4 masks/prefixes, canonical IPv6, explicit IOS configuration, snapshot roles, stale progress, command families and sentinels, scoped command ordering/conflicts, report escaping, preferences, localization, protected atomic exports, hardlinks, asynchronous state, virtualized Unicode XML, and drawing all six UI tabs.
- The vendored codec has 18 passing unit tests, including the added decompression-size limit/mismatch regression.
- A separate opt-in real-PKA integration test passes on a private duplicate. Real files and decoded answers are not committed or packaged.

The final local check transcript and counts are summarized below after the final run. The default real-file test remains ignored unless `PKA_INSPECTOR_LOCAL_SAMPLE` points to a legitimate local fixture.

## Real files

The MIT native codec decoded and parsed all six activities bundled with the installed Packet Tracer 9.0.1: MQTT, Sokoban, the industrial control panel, and three PLC examples. Observed selected tree counts are MQTT 8 and Sokoban 2; the four scripted industrial/PLC activities have no selected tree checkpoints. Their scripts and alternative grading systems are not executed or emulated.

A private duplicate of a user activity decoded to 15,087,355 bytes of XML and yielded 213 selected candidate leaves, 213 candidate weight, and 21 identified assessed devices. It contained 22,212 unchecked/partial/disabled terminal inventory nodes, retained in the raw XML and counted in diagnostics. On this sample, supported CLI/GUI suggestions are a subset of all leaves; unsupported cases remain explicit. The original was not modified. Following the user's instruction, further exam testing uses only the duplicate under ignored `private-fixtures/`.

Additional legitimate local activity files were tested read-only before the user's exam-file restriction. Their observed selected counts ranged from 3 to 136. A real standalone VLAN PKT also decoded and parsed successfully, with no assessment invented from its configuration alone.

All tested real activity snapshots lack explicit reliable role metadata. Their automatic outcomes therefore remain Unverified. Successful decoding is not evidence of native-grader equivalence or universal PKA compatibility. The supplied 32-checkpoint / 100-point / 7-device baseline has not been identified and has not been certified. No actual 126-checkpoint PKA has been identified or certified.

## Synthetic tests

`tests/fixtures/synthetic-126.xml` is original synthetic data with 126 selected checkpoints, 126 points and seven devices. It produces 7 Passed, 7 Failed and 112 Unverified, with 7 verified points and 11.11% verification coverage. Its explicit current-network role is synthetic and does not establish that unlabelled real activity snapshots have the same structure.

Unit tests additionally exercise explicit role order changes, a supported failed check, current/expected evidence, missing and conflicting observations, GUI-only endpoint settings, supported command grammars for VLANs, ACL, NAT/PAT, DHCP, OSPF, RIP, EIGRP, VTY/console/device security and save commands, and rejection of incomplete or ambiguous inputs. Complex family tests establish command grammar support on explicit synthetic requirements; they do not establish extraction/validation coverage for every real Packet Tracer grading schema.

## Desktop verification

A release executable launches on Windows and exposes the native accessibility tree. The synthetic file loads in the background, displays all 126 checkpoints and the correct score/status counts, and supports visible device/status filters and checkpoint selection. Switching English/Vietnamese was verified in the live application; Vietnamese glyphs were visually checked after loading the Windows Segoe UI system font. The toolbar and tabs wrap; checkpoint rows and long XML are virtualized, with separate checkpoint details and command preview.

Clipboard and file-export logic have automated state/export coverage. Final direct live clipboard evidence is recorded below if completed. The live desktop check does not prove every manual UI path or every GPU configuration.

## Measurements

Representative single warm release probe runs on this machine (not a statistical benchmark):

| Input | XML | Decode | XML parse | Extraction | Validation | Total load |
|---|---:|---:|---:|---:|---:|---:|
| Synthetic 126 | 20,624 bytes | 0.019 ms (plain XML detection) | 0.575 ms | 0.560 ms | 0.604 ms | 2.292 ms |
| Private duplicate | 15,087,355 bytes | 763.385 ms | 125.626 ms | 24.321 ms | not attempted: ambiguous role | 941.960 ms |
| Bundled MQTT PKA | 5,300,749 bytes | 461.488 ms | 26.767 ms | 2.573 ms | not attempted: ambiguous role | 498.435 ms |

The initial release process produced a window handle in approximately 5.2 seconds, including graphics/backend initialization; that is a window-readiness observation, not a precise first-painted-frame measurement. An observed running synthetic session used about 175 MiB working set and 198 MiB peak working set before the final system-font change. GPU driver and shared OS memory contribute to this process-level measurement. No allocator-level memory benchmark, statistical startup distribution, or cross-machine performance claim is made.

## Remaining limits

- `NAME checkType=0/1/2` is interpreted from its observed Qt-like tree check states. The interpretation is explicitly diagnosed as inferred; a controlled GUI-save/schema experiment was attempted but not completed, so selected counts and weight are not certified Cisco grades.
- Unlabelled embedded answer/initial/current snapshots are not assigned by position. Expected values are taken only from explicit assessment `nodeValue`; missing/sentinel values are not reconstructed from an ambiguous answer network.
- Validators cover a conservative subset: explicit IPv4, masks/prefixes, IPv6 and prefix requirements, explicit interface power/state, and hostname/display name distinctions. Dynamic defaults, operational reachability, custom/script grading, variable binding and full routing/VLAN/ACL/NAT/DHCP semantic validation remain unsupported.
- Commands require complete explicit supported context. Unknown enums, numeric sentinel encodings, incomplete multi-field rules, unsafe values and conflicting requirements are rejected. Some real properties or masks/prefixes may not have enough reliable context to generate a command.
- Native activity password handling, score editing, bypass and saving modified PKA/PKT files are absent by design. The encoder is used only to construct synthetic codec tests.
- The modern authenticated pktfile container family is supported; legacy/other formats may fail with a diagnostic. Input, decoded XML, XML depth/node count and decompression are bounded.
- UI controls are localized through extensible JSON catalogs; unknown upstream/custom diagnostic text, assessment names, values, evidence and generated Cisco commands may remain English/raw.

## Final local check results

Final application suite: **68 passed, 0 failed, 1 opt-in real-file test ignored by default** (6 library, 24 GUI/state/render, 11 command, 27 inspector tests). The opt-in test was also run separately against the private duplicate and passed. Vendored codec suite: **18 passed, 0 failed**. Strict Clippy and rustfmt checks pass. The Windows release build succeeds.

The release GUI additionally loaded the private duplicate and reported 213 Unverified / 213 total candidates with 0 verified points; the process remained responsive. Its observed working set was about 139 MiB, with 210 MiB peak. The final probe reports 33 supported CLI checkpoint suggestions and 19 supported GUI suggestions on that duplicate; this is command-generation availability, not proof that the commands satisfy every native grader.

Native clipboard output was not independently confirmed: the live automation attempt encountered a stale element after the interface changed. The clipboard path uses egui's native clipboard support, and command output/state logic is tested; this specific manual acceptance check remains to be completed by a user. Export formats and source-preserving writes have automated integration coverage; the native Save dialog was not separately certified end to end.
