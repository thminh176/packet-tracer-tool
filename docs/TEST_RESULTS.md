# Verification and known limits

## v0.2.1 local patch, 2026-10-23

`cargo test --locked`: **209 passed, 0 failed, 1 private opt-in test ignored**; the vendor codec separately passes **18 tests**. Formatting, strict all-target Clippy, the Windows x64 MSVC release build and dependency license/source checks pass. The dependency audit reports zero vulnerabilities, with informational unmaintained advisories retained.

The rebuilt native Windows GUI opened the original four-checkpoint IPv6/dot1Q fixture and displayed four explicit matches. Its individual IPv6 forwarding copy action was read back from the Windows clipboard and matched the global IOS script exactly. This is a synthetic application smoke test, not an actual Packet Tracer device test.

Regression coverage includes scoped IPv6 Boolean mapping and explicit comparison, distinct interface enable/SLAAC commands, optional XML fields versus IOS evidence, dot1Q sibling selection, conflicting encapsulation requests and incomplete-parameter transfer into the Library. The original public `tests/fixtures/ipv6-dot1q.xml` deliberately uses subinterface suffix .77 and VLAN 42.

The same 32 permitted private copies contain 15 IPv6 forwarding requirements: command-ready counts improved from 0 to 15. Six EthernetEncapProcess objects have no selected VLAN sibling and remain incomplete; ten of fourteen explicit dot1Q VLAN leaves are command-ready, unchanged. These are generation results, not passing grades or Packet Tracer execution certification. Private files and decoded values are excluded from Git and packages.

The registry now contains 48 bounded families and 18 declared semantic subsets. No family has new actual Packet Tracer execution evidence. Historical counts below describe earlier builds.

## v0.2.0 final local candidate, 2026-10-23

`cargo test --locked`: **199 passed, 0 failed, 1 explicitly opt-in real-file test ignored**. The vendored codec suite separately passes **18 tests** (217 total passing). Formatting and strict all-target Clippy pass. Conditional VLAN tests reject contradictory mode, active-VLAN and peer evidence, preserve Unverified status, and keep the typed library prerequisite workflow strict. Dot1Q takes its explicit VLAN requirement rather than a subinterface suffix.

The x86_64 MSVC release builds successfully. A fresh Windows GUI launched with the original synthetic 126-checkpoint fixture, displayed 7 Passed / 7 Failed / 112 Unverified and 7/126 verified points, opened the full-height Command Library and switched Vietnamese/English. Copying the Library's synthetic `basic.hostname` script was read back from the native clipboard and matched exactly. [Screenshot](images/cisco-library-windows.png) shows only synthetic data. Automated tests cover filters, preferences, reports, source-preserving exports and both library layout sizes; the native Save dialog is not independently certified end to end.

`cargo audit` reports **zero vulnerabilities**, with informational unmaintained-dependency advisories retained for review. `cargo deny --locked check licenses sources` passes. Local transcripts and private diagnostic output remain ignored. Historical test counts below describe earlier candidates.

No Cisco command family has actual Packet Tracer device execution certification. The 32-file read-only copy review establishes decoding/extraction only; see [real-file evidence](REAL_FILE_REVIEW.md). No official grade or completed activity is claimed.

The final routing correction was compared with v0.1.1 on the same permitted copy: ten OSPF leaves incorrectly overlapped static detection before, zero afterward. The rebuilt Windows GUI displayed OSPF badges and transferred an incomplete selected requirement to the Library, preserving the missing process ID and blocking executable-script copying. See [routing regression evidence](ROUTING_CLASSIFICATION_FIX.md). Private UI captures are excluded from distribution.

## Local v0.2 review update

The current local candidate passes **180 application tests**, with zero failures and one private opt-in test ignored, plus the codec's 18 tests. Formatting and strict all-target Clippy pass. This includes 17 independent public-API acceptance cases, 14 router cases, 15 switch cases, address-context regressions and two Library layout tests. See [real-file review](REAL_FILE_REVIEW.md) for the newly authorized 32-file collection and the identified 32/100/7 structural reference. These findings supersede older statements below that the reference had not been found.

The Library collapse was corrected and tested headlessly at 960×600 and 1280×820 in English and Vietnamese. The Windows release executable launched, and the live synthetic Library showed a full-height list and parameter/detail panel. Device-command copying from the synthetic fixture was independently read back from the native Windows clipboard. No new family has Packet Tracer CLI execution certification. No v0.2 changes have been pushed or published.

The sections below retain historical v0.1.x test evidence and limits; their counts describe those earlier builds.

Test date: 2026-10-22. Target: Windows x86_64, Rust 1.99.1, native eframe/egui 0.33.3 OpenGL backend.

## Automated checks

- Application tests cover:
  - Decoder corruption, binary/XML prefix collisions, XML limits/entities/DTD/duplicate fields.
  - Recursive extraction, selected versus inventory nodes, fractional and invalid weights.
  - Device and interface identity ambiguity, IPv4 masks/prefixes, canonical IPv6.
  - **Network normalization and routing engine (`tests/network_values.rs`, 12 tests)**: CIDR prefix normalization (`/prefix` and `-prefix`), subnet-to-wildcard conversions, host bit clamping with confidence levels, complete IPv4 static route parsing (`network-prefix-nexthop-distance`), default routes (`0.0.0.0/0`), exit interfaces, floating administrative distance, missing next-hop diagnostics, OSPF 3-token and 4-token parsing, area normalization (`0.0.0.0` vs `0`), process ID association, sentinel rejection, and Cisco IOS command generation.
  - Explicit IOS configuration, snapshot roles, stale progress, command families and sentinels.
  - Scoped command ordering/conflicts, report escaping, preferences, localization.
  - Protected atomic exports, hardlinks, asynchronous state, virtualized Unicode XML, and drawing all UI tabs.
- The vendored codec has 18 passing unit tests, including the added decompression-size limit/mismatch regression.
- A separate opt-in real-PKA integration test passes on a private duplicate. Real files and decoded answers are not committed or packaged.

The final local check transcript and counts are summarized below. The default real-file test remains ignored unless `PKA_INSPECTOR_LOCAL_SAMPLE` points to a legitimate local fixture.

## Real files

The MIT native codec decoded and parsed all six activities bundled with the installed Packet Tracer 9.0.1: MQTT, Sokoban, the industrial control panel, and three PLC examples. Observed selected tree counts are MQTT 8 and Sokoban 2; the four scripted industrial/PLC activities have no selected tree checkpoints. Their scripts and alternative grading systems are not executed or emulated.

A private duplicate of a user activity decoded to 15,087,355 bytes of XML and yielded 213 selected candidate leaves, 213 candidate weight, and 21 identified assessed devices. It contained 22,212 unchecked/partial/disabled terminal inventory nodes, retained in the raw XML and counted in diagnostics. On this sample, supported CLI/GUI suggestions are a subset of all leaves; unsupported cases remain explicit. The original was not modified. Following the user's instruction, further exam testing uses only the duplicate under ignored `private-fixtures/`.

Additional legitimate local activity files were tested read-only before the user's exam-file restriction. Their observed selected counts ranged from 3 to 136. A real standalone VLAN PKT also decoded and parsed successfully, with no assessment invented from its configuration alone.

All tested real activity snapshots lack explicit reliable role metadata. Their automatic outcomes therefore remain Unverified. Successful decoding is not evidence of native-grader equivalence or universal PKA compatibility. The supplied 32-checkpoint / 100-point / 7-device baseline has not been identified and has not been certified. No actual 126-checkpoint PKA has been identified or certified.

## Synthetic tests

`tests/fixtures/synthetic-126.xml` is original synthetic data with 126 selected checkpoints, 126 points and seven devices. It produces 7 Passed, 7 Failed and 112 Unverified, with 7 verified points and 11.11% verification coverage. Its explicit current-network role is synthetic and does not establish that unlabelled real activity snapshots have the same structure.

Unit tests additionally exercise explicit role order changes, a supported failed check, current/expected evidence, missing and conflicting observations, GUI-only endpoint settings, supported command grammars for IPv4/IPv6 static routes, OSPF networks, VLANs, ACL, NAT/PAT, DHCP, RIP, EIGRP, VTY/console/device security and save commands, and rejection of incomplete or ambiguous inputs. Complex family tests establish command grammar support on explicit synthetic requirements; they do not establish extraction/validation coverage for every real Packet Tracer grading schema.

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
- Validators cover a conservative subset: explicit IPv4, masks/prefixes, IPv6 and prefix requirements, IPv4 and IPv6 static routes, OSPF network statements, explicit interface power/state, and hostname/display name distinctions. Dynamic routing convergence, metric computation, operational reachability, custom/script grading, variable binding and full VLAN/ACL/NAT/DHCP semantic validation remain unsupported.
- Commands require complete explicit supported context. Unknown enums, numeric sentinel encodings, incomplete multi-field rules, unsafe values and conflicting requirements are rejected. Some real properties or masks/prefixes may not have enough reliable context to generate a command.
- Native activity password handling, score editing, bypass and saving modified PKA/PKT files are absent by design. The encoder is used only to construct synthetic codec tests.
- The modern authenticated pktfile container family is supported; legacy/other formats may fail with a diagnostic. Input, decoded XML, XML depth/node count and decompression are bounded.
- UI controls are localized through extensible JSON catalogs; unknown upstream/custom diagnostic text, assessment names, values, evidence and generated Cisco commands may remain English/raw.

## Final local check results

Final application suite: **97 passed, 0 failed, 1 opt-in real-file test ignored by default** (15 library, 28 GUI/state/render, 12 command, 30 inspector, 12 network_values). The opt-in test was also run separately against the private duplicate and passed. Vendored codec suite: **18 passed, 0 failed**. Total passed across test suites: **115 tests**. Strict release packaging builds succeed.

The release GUI additionally loaded the private duplicate and reported 213 Unverified / 213 total candidates with 0 verified points; the process remained responsive. Its observed working set was about 139 MiB, with 210 MiB peak. The final probe reports 33 supported CLI checkpoint suggestions and 19 supported GUI suggestions on that duplicate; this is command-generation availability, not proof that the commands satisfy every native grader.

Native clipboard output was not independently confirmed: the live automation attempt encountered a stale element after the interface changed. The clipboard path uses egui's native clipboard support, and command output/state logic is tested; this specific manual acceptance check remains to be completed by a user. Export formats and source-preserving writes have automated integration coverage; the native Save dialog was not separately certified end to end.
