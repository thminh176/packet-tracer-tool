# Project provenance

Audit date: 2026-10-22. Dependency/source candidate: `2865b5e`, including implementation hardening `0e9ea53`; packaging follow-up `9bb1cd4` was inspected read-only in the coordinator checkout and does not change this dependency graph. This is an evidence record, not a certification of sole authorship or legal clearance. See [release gates](LICENSE_AUDIT.md).

## Scope and history inspected

Reviewed tracked source, manifests, lockfile, notices, synthetic fixture, packaging/workflow files, reachable Git history and source diffs. No private activity files, private answer networks, external personal folders or credentials were opened. Registry dependencies and public upstream repository records were consulted. The history starts with an empty initial commit `a554fc9`; `a9157e1` introduces the Rust application contract, `6000093` adds reports/notices, `b20742b` vendors the bounded decoder patch, and subsequent scoped commits add commands, localization, GUI and inspection. Reachable alternate branch commits were also searched for restrictive-project names and entry points.

Application modules are Rust UI, XML evidence analysis, command generation, localization and report exports. No imported Python crypto/editor package, translated PKA Decipher module, simulator executable, Cisco logo or activity/courseware asset was identified in the tracked baseline tree. The fixture `tests/fixtures/synthetic-126.xml` is labeled original synthetic data; that label is supported by its construction-oriented tests, but it does not prove external authorship independently. Container encoding appears in tests and the upstream library; the baseline application adapter only invokes decoding and exposes no activity writer.

The baseline README states no code was copied or translated from PKA Decipher. History/source inspection found no contrary evidence: searches for restrictive-project names, Python entry points and patch-library identifiers produced no application-code hits. This is a bounded negative finding, **not proof of a clean-room process**. No private development records or restrictive implementation were examined. Similar XML field names or public file-format behavior alone do not demonstrate copying; no cross-project plagiarism claim is made. Obtain contributor provenance declarations before making an unqualified public claim of independent implementation.

## Vendored pktfile chain

| Evidence | Recorded value |
|---|---|
| Registry release | [pktfile 0.2.2 record](https://crates.io/api/v1/crates/pktfile/0.2.2), published 2026-09-28; MIT; not yanked on audit date |
| Published crate archive checksum | SHA-256 `56bd8d411df87a77539006eaeeedd4bcfea636a6a04fb288aa5ce56d96a7fcc2` from registry release metadata |
| Published VCS mapping | Registry `.cargo_vcs_info.json`: `b73344420af4a1be0489a435968808b12e104195`, `crates/pktfile` |
| Public source | [pinned upstream crate](https://github.com/TantiE100/pktctl/tree/b73344420af4a1be0489a435968808b12e104195/crates/pktfile) |
| Upstream notice | [MIT license](https://github.com/TantiE100/pktctl/blob/b73344420af4a1be0489a435968808b12e104195/LICENSE), copyright 2026 Santi E.; Git blob `87b1a82dcda9eac68d165b05ea9c4298bb6475ec` |
| Local resolution | Root manifest `[patch.crates-io]` resolves `pktfile = "=0.2.2"` to `vendor/pktfile` |
| Local attribution | `vendor/pktfile/LICENSE`, `README.md`, `PATCH.md`; dependency inventory in `THIRD_PARTY_NOTICES.md` |

Compared every retained vendor file with the cached registry 0.2.2 package, normalizing line endings: LICENSE, README and `canvas.rs`, `eax.rs`, `elements.rs`, `physical.rs`, `wireless.rs` match. Differences are `Cargo.toml`, `src/lib.rs` and the new `PATCH.md`. The manifest removes unused example/test targets and changes quick-xml from 0.38 to 0.41; the current lockfile resolves quick-xml **0.41.0** for both the application and local pktfile. This parser upgrade addresses RUSTSEC-2026-0194 and RUSTSEC-2026-0195; it does not change the exact pktfile 0.2.2 identity. `inflate` rejects short prefixes, caps XML at 64 MiB, bounds reads to declared length plus one and rejects length mismatches, with a regression test. No local cryptographic changes were found. The local patch description agrees with the inspected diff. The published crate checksum above identifies the upstream archive; it is not a checksum for the modified local tree.

The upstream [crate README](https://github.com/TantiE100/pktctl/blob/b73344420af4a1be0489a435968808b12e104195/crates/pktfile/README.md) credits [Unpacket](https://github.com/Punkcake21/Unpacket/tree/54cb6d6a9b46c6b4973356258cb8df6ae58b2ec7) for format documentation. Unpacket's reviewed [license](https://github.com/Punkcake21/Unpacket/blob/54cb6d6a9b46c6b4973356258cb8df6ae58b2ec7/LICENSE) is MIT, copyright 2026 Punkcake21. This is an upstream acknowledgment, not evidence that our application directly imported its code. Preserve that credit; additional source reuse requires its actual notice.

## Fonts and other assets

The baseline enables eframe `default_fonts`, embedding the epaint font payloads. The exact `epaint_default_fonts` 0.33.3 package maps to egui commit `44cdd653e2317d300fb8a6c9c36b03f23991e803`, [font source directory](https://github.com/emilk/egui/tree/44cdd653e2317d300fb8a6c9c36b03f23991e803/crates/epaint_default_fonts/fonts). The archive contains Hack, Noto Emoji, Ubuntu Light and emoji-icon-font; see the audit for their distinct notice obligations. Fonts are not all covered by the application's MIT license. Preserve the complete Hack notice, including Bitstream Vera terms, even though the package SPDX expression omits that nested name.

The UI additionally reads Segoe UI from the installed Windows font directory at runtime. It does not include that system font in tracked files or the packaging copy list. This is not a grant to redistribute Microsoft font files. Retain the OS-only loading model and check the final archive for accidental font inclusion.

No Cisco executable, library, branding image or courseware file was present in the tracked baseline. Decoded XML, reports and user-selected activities remain user content and may carry third-party rights even where this code is MIT. Do not commit real activity/answer data or redistribute extracted imagery and course text without separate rights.

## Remaining evidence to obtain

Before shipping, record the release commit and final archive hash, close the notice/artifact gate and document the Cisco-use caution in [LICENSE_AUDIT.md](LICENSE_AUDIT.md), and retain checksum-backed upstream notices in the release source. The current inventory is 397 lockfile packages including the application, with one quick-xml version; the earlier 398-package baseline is obsolete. The supplemental notice manifest has 33 records, whose content was verified after newline normalization in this worker checkout. Packaging follow-up `9bb1cd4` disables Git newline conversion for those files and fails on raw digest mismatch; final ZIP verification remains a release-owner action.

Contributor origin declarations would strengthen the bounded source/history review, especially before making an unqualified independence claim; their absence alone is not an established infringement or an additional permission gate. No actual restrictive-code import, Cisco source import or specific distribution-right defect was identified. Re-run the inventory when dependencies/features/platforms change. Do not describe this review as legal advice, a similarity scan against all public software, or a guarantee that all historical inputs were original.
