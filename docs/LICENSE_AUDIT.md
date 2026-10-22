# License and release audit

Verified 2026-10-22. Application baseline `8d64768`; dependency inventory: 398 Cargo.lock package entries including this application, 397 third-party entries. All corresponding third-party manifests were available locally. This review covers declared licenses, retained upstream notices, public terms and the inspected packaging design; it is not legal advice or an assurance of all jurisdictions' rights. [Provenance](PROJECT_PROVENANCE.md) and [ecosystem sources](ECOSYSTEM_COMPARISON.md) supply the source chain.

## Actionable verdict

**Hold the baseline binary release until dependency notices are complete and the final ZIP is verified.** The confirmed distribution blocker is missing full license material for several Windows dependencies. Supply pinned upstream notices, include applicable copyright/NOTICE records, and inspect the finished artifact. A bare executable must not be the only distribution without legally required accompanying materials. Packaging changes under development must be audited against the finished artifact before this gate closes.

**The encrypted-container adapter has a documented rights caution, not a demonstrated distribution ban.** The inspected evidence supplies an MIT grant for pktfile and shows no Cisco proprietary code import, binary disassembly or restrictive implementation import in this repository. Cisco contractual terms concern applicable Cisco Offer/Documentation use; those terms alone do not establish a missing right to redistribute independently licensed code. No component block is justified solely by generic, unassessed jurisdictional risk. Users remain responsible for applicable agreements, authorized file access and content rights. The release owner should record this scope and avoid claiming universal legal clearance.

If specific evidence later identifies restricted derivation, an applicable breached agreement, unauthorized protected-content distribution or an unresolved material anti-circumvention issue, **block the affected encrypted adapter/content component** until resolved; do not present an educational-purpose disclaimer as a cure. An XML-only build with the adapter removed or disabled is an optional narrower scope if such an issue arises, not a prerequisite established by this audit. This documentation change implements no feature gate. Original source, XML toolkit and MIT decoder code retain distinct provenance and notice obligations; any chosen scope still needs complete notices and authorized fixtures/assets.

No direct restrictive-code contamination or mandatory GPL dependency was found in the baseline. Contributor origin declarations remain an evidence gap, not an identified infringement. Any later evidence of copied restrictive code must block the affected component until removed or separately licensed.

## License compatibility and transitive obligations

`LICENSE` grants MIT for application-owned code. Retain component-specific terms; do not label all bundled materials MIT. Lockfile inventory includes MIT/Apache alternatives, Apache-only packages, BSD, ISC, Zlib, Boost Software License 1.0, Unicode-3.0, CC0, plus font licenses. No unavoidable GPL/LGPL dependency was found: `r-efi` offers MIT/Apache alternatives to LGPL, and alternatives must be consciously selected. `AND` expressions are cumulative; old slash-separated MIT/Apache metadata should be checked against actual texts, not mechanically treated as a new license.

| License family / component | Required distribution action |
|---|---|
| MIT, including local pktfile | Include full permission, copyright and warranty notice; keep upstream holder names. Preserve local patch description. |
| Apache-2.0-only and chosen Apache alternatives | Include complete license and applicable upstream NOTICE/attribution; identify modified files where required. Apache patent/mark provisions remain distinct from MIT. |
| BSD-2/3, ISC, Zlib | Preserve applicable copyright, conditions and disclaimer; obey nonendorsement and altered-source attribution where applicable. |
| BSL-1.0 | Retain the Boost text where required; do not confuse it with Business Source License. |
| Unicode-3.0 / combined expressions | Include Unicode copyright/permission notices, including for generated Unicode data; a code license alone is insufficient. |
| CC0 / Unlicense alternatives | Preserve supplied dedication/disclaimer records; do not invent upstream copyright ownership. |
| OFL / Ubuntu / nested font licenses | Ship the font-specific full texts and copyright records; do not relicense fonts as MIT or sell restricted fonts alone. |

This inventory deliberately includes non-Windows and optional lockfile packages. It is not proof each crate is linked into the Windows executable. Check the target/features used for the release with `cargo tree --locked --offline --target x86_64-pc-windows-msvc`; include build-tool notices conservatively where generated material may be incorporated. Final linked/transitive content and source headers still need artifact-level review. No blanket compatibility certification follows from SPDX metadata.

## Concrete missing-text remediation

The baseline `scripts/build-release.ps1` recursively collects LICENSE/LICENCE/COPYING/NOTICE/UNLICENSE/AUTHORS/OFL/UFL/Hack notice files plus manifest `license_file`, preserving relative paths. This captures existing notices but does not fail when none exist; `CRATE.txt` with an SPDX name is not the full license grant. The following Windows dependency/build graph gaps were confirmed against cached archives. API trees verified these upstream fallback paths at the exact `.cargo_vcs_info.json` revisions:

| Packages lacking a conventional full license file in registry archive | Pinned source for fallback texts |
|---|---|
| eframe, egui, emath, epaint, ecolor, egui-winit, egui_glow 0.33.3 | [emilk/egui at 44cdd653](https://github.com/emilk/egui/tree/44cdd653e2317d300fb8a6c9c36b03f23991e803): root `LICENSE-MIT`, `LICENSE-APACHE` |
| accesskit 0.21.1 | [AccessKit at 0b6901a8](https://github.com/AccessKit/accesskit/tree/0b6901a8c8e6fb1cb5a2dc7c19dc7bee1cc5ad09): root `LICENSE-MIT`, `LICENSE-APACHE`, `LICENSE.chromium` |
| accesskit_winit and accesskit_windows 0.29.2; accesskit_consumer 0.31.0 | [AccessKit at 5801c4a4](https://github.com/AccessKit/accesskit/tree/5801c4a49ff9b2c9ec3f77b3c31272d0da4c5cd3): same three root files; inspect applicable Chromium-derived portions |
| profiling 1.0.18 | [profiling at 82715511](https://github.com/aclysma/profiling/tree/8271551172eb6fa4cba47369aedd93790c623df9): root `LICENSE-MIT`, `LICENSE-APACHE` |
| clipboard-win 5.4.1 | [clipboard-win at 3b27cf2b](https://github.com/DoumanAsh/clipboard-win/tree/3b27cf2bfd1adcfa6e0264eb51c1025ddaf0f342): root `LICENSE` |
| gl_generator 0.14.0 (build graph) | [gl-rs at ea503e8d](https://github.com/brendanzab/gl-rs/tree/ea503e8d5fb6d73c6030e6191ce738cd3bf3433e): root `LICENSE` |
| khronos_api 3.1.0 (build graph) | [gl-rs at f150967b](https://github.com/brendanzab/gl-rs/tree/f150967b1c44ae888e6676f93f639ebc82771bdc): root `LICENSE` |

Non-Windows lockfile packages also lack files (including objc2 family, block2, dispatch, atspi-proxies, JNI, NDK and r-efi); another platform release needs its own complete target audit. Store retrieved fallback bytes with source URLs/revisions/checksums, preserve upstream copyright headers and applicable nested notices, and fail packaging for an unresolved required text. A new fallback bundle must be reviewed for content, not merely presence.

## Bundled fonts

The enabled `default_fonts` feature embeds fonts from [epaint_default_fonts 0.33.3 at its published revision](https://github.com/emilk/egui/tree/44cdd653e2317d300fb8a6c9c36b03f23991e803/crates/epaint_default_fonts/fonts). All four following notice files exist in the registry archive and match the baseline packaging filters:

| Font payload | Required retained file / observation |
|---|---|
| Hack-Regular.ttf | `Hack-Regular.txt`: Source Foundry MIT **and Bitstream Vera license**, plus DejaVu public-domain statement. Do not shorten to MIT. |
| NotoEmoji-Regular.ttf | `OFL.txt`: SIL OFL 1.1. Preserve copyright metadata with the embedded font; inspect name-table copyright if making a text notice index. |
| Ubuntu-Light.ttf | `UFL.txt`: Ubuntu Font Licence 1.0. Preserve copyright metadata and unchanged naming; modified fonts have naming conditions. |
| emoji-icon-font.ttf | `emoji-icon-font-mit-license.txt`: John Slegers MIT notice. |

The fonts remain under their own licenses; unchanged embedding does not require relicensing application code under OFL/UFL. The source package SPDX expression is `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`, which does not enumerate every nested Hack notice. Runtime Segoe UI is supplied by Windows, not redistributed; exclude system font copies from artifacts.

## Cisco terms, branding and uncertainty

Official sources reviewed on 2026-10-22:

- [Packet Tracer Offer Description](https://www.cisco.com/c/dam/en_us/about/doing_business/legal/seula/cisco-packet-tracer-software.pdf), controlled document EDCS-19611795 v2.0, modified 2025-09-10. Section 4.1 limits use of the Cisco Offer/related Documentation to Networking Academy simulations or other written-approved purposes. This is a restriction on covered Cisco use, not an express license or ban governing every independent third-party program.
- [Cisco General Terms](https://www.cisco.com/c/en/us/about/legal/cloud-and-software/end_user_license_agreement.html), EDCS-24218913 v6.0, modified 2025-09-10. Section 2.2(d) restricts decryption/reverse engineering/modification of Cisco Offers; 2.6 discusses legally required interoperability information. Whether these terms, older installation terms, statutory exceptions or other agreements apply to a particular developer/user cannot be determined from the repository. [Cisco's terms index](https://www.cisco.com/c/en/us/about/legal/cloud-and-software/software-terms.html) connects the product document to the general framework.
- [Cisco trademark list](https://www.cisco.com/c/en/us/about/legal/trademarks.html) and [logo guidance](https://www.cisco.com/c/en/us/about/brand-center/logo-usage-guidelines.html). Descriptive compatibility references do not imply endorsement or grant logo permission. Use an independent project name and retain a nonaffiliation statement; do not ship Cisco branding artwork.

PKA Lab Inspector is an independent project, not affiliated with, sponsored by or endorsed by Cisco. Cisco, Cisco IOS and Packet Tracer names identify third-party products; relevant trademark rights remain with their owners. No Cisco binaries, logo assets or courseware should be bundled. User files, answers, screenshots, documentation and device templates may have separate rights; file possession and an educational-purpose disclaimer do not establish redistribution permission.

The upstream code license permits reuse of that code subject to its conditions. It does **not** supply Cisco authorization, user contractual permission, rights to protected activity content, or a universal anti-circumvention exemption. Conversely, the existence of restrictive Cisco terms does **not** prove that this original tool or every act of file-format interoperability violates law. Assess any specific affected facts if they arise; do not assert either universal legality or universal illegality. No concrete third-party distribution-right defect in pktfile was established by this audit.

## Release owner acceptance record

Before closing this audit, record the candidate commit, target/features, fallback notice manifest and checksum verification, finished ZIP inspection (including all font notices), source/content exclusion scan and the documented Cisco-use caution (plus disposition of any specifically identified rights issue). Keep a source-only/XML-only candidate distinct from a decoder-enabled build. At this audit checkpoint no final candidate ZIP was available in this worktree; the notice/artifact gate remains open until that evidence is attached. The current evidence does not impose a separate adapter permission gate. This audit makes no source-code changes and does not authorize publishing a release.
