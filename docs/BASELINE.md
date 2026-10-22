# Release baseline

Baseline commit: `8d64768`. Audit date: 2026-10-22. Working tree was clean and had no configured Git remote. GitHub CLI authenticated successfully; no repository was created during baseline capture.

`cargo test --locked`: 68 passed, 0 failed, one optional real-file test ignored. Existing Windows release executable was previously launched and real/synthetic evidence is recorded in TEST_RESULTS.md. The release is an offline native Rust eframe application with read-only decoder, conservative checkpoint extraction/validation, commands, CSV/Markdown exports and extensible English/Vietnamese catalogs.

Known regressions at baseline: IP and subnet-mask checkpoints produce duplicate identical commands in device scripts; unsupported items repeat generic English comments; CLI destination is insufficiently clear. These are the first stabilization changes. Unknown snapshot roles and assessment selection semantics remain diagnosed limitations. Full native Cisco grading, universal activity compatibility and live integration are outside v0.1.0 scope.

Release task graph: command stabilization + UI polish + independent license/ecosystem audit + read-only security review ? integration and source/history audit ? tests, license/vulnerability checks, real GUI/export smoke ? public repository and CI ? annotated v0.1.0 tag ? successful Windows workflow ? release ZIP/checksum verification and publication. Only then may future feature branches be started.
