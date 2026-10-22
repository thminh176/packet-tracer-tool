# v0.1.0 source and publication review

Review date: 2026-10-22. Candidate: `2865b5e6f16ba8dd490814badc2c5f3992f6a1cf`. This updates the earlier baseline assessment using independent candidate checks. It does not certify Cisco compatibility, native grading equivalence or a finished release artifact.

**Verdict: no new serious parser or command safety blocker found; baseline check failures have follow-up source fixes, but publication still needs final-candidate regressions and artifact acceptance.** Parser budgets, bounded reads, command deduplication and the quick-xml security upgrade are present and synthetic regressions pass. Follow-up commits `9bb1cd4f44e71e9a4f3f8b641069f87848f497cf` and `4eaa1bb65382db3ab6bc09ed4ece82d9cfc51f84` were inspected through Git without applying them to this checkout; their test/artifact results are not asserted by the baseline runs below.

## Findings and current dispositions

| ID | Candidate status | Evidence and release action |
|---|---|---|
| R1 | Resolved for reported path amplification | `src/inspector.rs` preflights 512-byte labels, 4,096-byte paths/values, an 8 MiB aggregate path budget and 50,000 terminal nodes before extraction. Oversized-label and repeated-deep-path regressions pass. These are application limits, not a process memory sandbox. |
| R2 | Follow-up source remediated; artifact gate open | `scripts/build-release.ps1` refuses an existing versioned staging directory, closing stale-directory reuse. Whole local `docs`, `locales` and `third-party` trees are still copied recursively, including possible untracked material. Follow-up `9bb1cd4` replaces broad copying with `scripts/package-files.txt`, tracked-file checks and hash-verified supplemental notice paths. This closes the reported broad-copy source defect. Inspect the final ZIP and prove seeded unlisted-file exclusion; tracked files can still contain local edits, so use a clean reviewed release checkout. |
| R3 | Current checksum/document fixes verified; history decision open | Executable/ZIP checksums use relative filenames. Current tracked blobs contain no matched coordination identifiers or personal absolute paths. Two historical coordination identifiers remain in `docs/MILESTONES.md`; counts/path only are reported below. The coordinator explicitly accepts this nonsecret historical metadata without a history rewrite; keep its presence disclosed. Final ZIP privacy remains open with R2. |
| R4 | Reported read/matching issues remediated | Reads use `take(MAX_BYTES + 1)` and reject excessive length. Device/checkpoint pairs are capped at five million and snapshot-node/checkpoint operations at fifty million. XML has 64 MiB, one-million-element and depth-128 limits. Special-file wall time and allocator termination are not contained; conservative trusted local input remains the scope. |
| R5 | Duplicate-command defect resolved; nonblocking semantic limits | Exact `Plan` equality deduplicates paired IP/mask commands; rendering groups identical interface contexts and saves last. Dedup/grouping, conflict, injection and ordering tests pass. Overlapping VTY ranges, equivalent interface spellings and interacting properties are not fully solved. Linear plan searches can impose quadratic work. Require device-specific review; all-device exports contain separate blocks, not automatic device selection. |
| R6 | Follow-up source remediated | Markdown escapes HTML, pipes and line breaks but leaves links/images/backslashes active. Follow-up `9bb1cd4` additionally escapes link/image delimiters, backslashes and other Markdown syntax, with regression assertions. Baseline renderers could fetch external images or display misleading links. CSV quoting/formula-prefix defenses have tests but do not cover every importer. Expected/current values can contain lab credentials; review before sharing. |
| R7 | Follow-up source remediated | Individual weights must be finite/nonnegative, but `Analysis::summary` does not detect aggregate floating-point overflow. Multiple extreme finite weights can yield infinite totals. Follow-up `9bb1cd4` rejects nonfinite aggregate assessment weights during parse and adds a regression. Manually constructed public `Analysis` values still bypass parsing; these are not native grading scores. |
| R8 | Low / nonblocking worker/state privacy limitation retained | `clear_progress` invalidates results and clears loading without stopping the existing worker; repeated clear/open actions can leave concurrent work. Persisted checklists/preferences retain activity identities and notes. Cancellation is result invalidation, not guaranteed resource cancellation. |
| R9 | Vulnerability blocker resolved; policy gate remains | Application/vendor declarations resolve to quick-xml 0.41.0. Audit finds zero vulnerabilities. `cargo deny --locked check` exits 1 for Windows-reachable unmaintained `ttf-parser` 0.25.1 (`RUSTSEC-2026-0192`); bans, licenses and sources pass. Follow-up `4eaa1bb` records explicit maintenance-risk acceptance in `SECURITY.md` and makes CI run audit plus separate deny license/source gates. This disposes the maintenance warning for frozen MVP scope; full deny still fails and must not be described as passing. Unmaintained status alone is not a demonstrated exploitable vulnerability. |
| R10 | Follow-up source fixed; rerun required | `cargo clippy --locked --all-targets -- -D warnings` exits 1 on `clippy::collapsible_if` at `src/commands.rs:696`, the nested deduplication conditions. Follow-up `9bb1cd4` collapses these conditions as suggested. Rerun the required check on the integrated candidate; the baseline result remains failed. This is a build-policy failure, not a command safety flaw. |

## Positive source controls

- Application paths read activity/progress files without invoking the vendored encoder on user sources. No device-command execution, network client, password bypass or activity-score rewriting was found in reviewed application paths.
- XML rejects mismatched tags, duplicate attributes, unknown entities and DTDs. The vendored inflater caps declared output at 64 MiB, reads at most declared size plus one and requires exact length. Synthetic corruption/native round-trip tests pass.
- Unknown/custom/variable/ambiguous assessments remain Unverified. Snapshot roles are not inferred by position; comparison requires supported explicit values and unique identities.
- Narrow command grammars reject control-character injection and unsupported contexts. Commands remain suggestions requiring human review.
- Exclusive sibling temporary exports and rename protect loaded sources and Packet Tracer extensions, with hardlink-preservation and failed-write cleanup coverage. Hostile concurrent filesystem replacement is outside this guarantee.

## Reachable history and tracked-content privacy

No `main` or remote-tracking ref exists here. Local `master` and the candidate branch both point to the reviewed commit, so the mainline audit uses **13 commits reachable from `master`, containing 108 unique file blobs**. The current index contains **91 tracked paths / 67 unique blobs**. Other branches, unreachable objects, reflogs, ignored files and remote repositories are outside these counts.

Targeted heuristics checked provider-token patterns, private-key headers, quoted credential assignments, personal absolute paths, coordination identifiers and coursework/privacy hints. Scan output contained counts/paths only, never matching values:

| Category | Reachable mainline | Current tracked blobs |
|---|---|---|
| Provider-token patterns | 0 | 0 |
| Private-key headers | 0 | 0 |
| Quoted credential assignments | 0 | 0 |
| Personal absolute paths | 0 | 0 |
| Coordination identifiers | 2 matches; `docs/MILESTONES.md` | 0 |
| Coursework/privacy keyword hints | 5 matches; `.gitignore`, `docs/REVIEW.md`, `docs/TEST_RESULTS.md` | 3 matches; same paths |
| PKA/PKT, archive, executable, DLL, PDF, DOCX or PPTX paths | 0 | 0 |

Keyword hints occur in ignore rules and review/test documentation, not discovered private courseware payloads. Generic synthetic test configuration and the codec's public format constant are not discovered user secrets. This is a heuristic review, not exhaustive secret-scanner or forensic assurance. No real private PKA/PKT files, ignored fixtures or decoded private contents were opened; tests used synthetic fixtures. User exports/persisted state can still contain private material, and Git ignore rules do not govern ZIP contents.

## Dependencies, workflows and artifact gates

cargo-audit 0.22.2 reports zero vulnerabilities across 397 dependencies against database `59ef7abf8ce56ea80fe7eba97ba3835449009fa4` (1,348 advisories; reported update 2026-10-22). Four unmaintained warnings remain: `ttf-parser` 0.25.1, `zbus_names` 4.3.5, `zvariant` 5.15.1 and `zvariant_derive` 5.15.1. cargo-deny 0.20.2 fails its advisory policy for the font parser through Windows dependencies `owned_ttf_parser` / `ab_glyph` / `epaint`; it is not solely a non-Windows warning. License metadata policy passes but does not verify required bundled notice bytes.

Actions are now pinned. Exact-revision `action.yml` fetches from official GitHub repositories returned HTTP 200 for checkout (`11d5960a326750d5838078e36cf38b85af677262`), rust-toolchain (`b725eda525362e7cf34af9a7243cac1e3f01a7ec`), upload-artifact (`ea165f8d65b6e75b540449e92b4886f43607fa02`) and download-artifact (`d3f86a106a0bac45b974a628896c90dbdf5c8093`). This proves resolvable definitions, not hosted workflow execution or the comments' version labels. GitHub commit API requests returned HTTP 403; raw exact-revision files supplied validation. Stable Rust and `windows-latest` still move.

The baseline names an x64 archive without forcing the target and hardcodes the version; inspected follow-up `9bb1cd4` forces `x86_64-pc-windows-msvc`, derives the version from Cargo metadata and rejects a mismatched GitHub tag. Coordinator remediation should verify target/version/tag agreement, an explicit publication manifest, supplemental notice hashes, all embedded font notices, final ZIP contents and checksums. See [LICENSE_AUDIT.md](LICENSE_AUDIT.md) for separate notice/content acceptance. No final ZIP, clean-machine launch, code signing, native clipboard or save-dialog acceptance was established by this worker. No GUI control, GitHub mutation or publication occurred.

## Verification and frozen scope

Commands used the requested checkout cargo executable, version 1.99.1 with rustc 1.99.1. `cargo test --locked --all-targets` passed **71 tests, zero failures, one opt-in private-file test ignored**: 6 library, 24 UI, 12 command and 29 inspector passes. Resource-budget and command deduplication regressions pass. `cargo fmt --all -- --check` passed. Standalone vendored codec tests passed **18 tests**; they resolve their own ignored lockfile and do not define the shipped inventory. Clippy failed as recorded in R10; do not carry forward the old baseline's clean clippy claim to this candidate.

Freeze v0.1.0 as an offline, read-only inspector for the modern authenticated container family and supported explicit XML semantics. Selected-tree interpretation is inferred; unlabeled snapshots remain Unverified. Synthetic grammar coverage does not certify real schema extraction or native grading. Keep editing, bypass, password recovery, script execution and universal grading outside this release.
