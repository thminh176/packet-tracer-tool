# Milestones and ownership

Orca run: run_743a8b6e0f0f.

1. Shared model and module contract (coordinator) ? decoder/extraction (Codex, inspector-core) + GUI (Antigravity, inspector-ui) + commands (Codex, inspector-commands).
2. Structured assessment and scores depend on decoder/extraction; summary model belongs to coordinator.
3. Conservative progress validation and evidence belong to inspector-core.
4. Context-aware commands belong to inspector-commands; integration depends on real schema extraction.
5. Desktop filtering, manual state, clipboard and reports depend on all three modules. Reports belong to coordinator.
6. Coordinator reviews and integrates isolated worktree commits, resolves compilation failures, executes format/lint/tests, probes real files, builds and packages Windows release. Follow-up corrections are dispatched only after existing ownership settles.

No workers share editable files. Real files are read-only local test inputs and excluded from Git. Release artifacts stay local; GitHub Actions builds downloadable artifacts only when the repository workflow runs.
