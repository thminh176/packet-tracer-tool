# Contributing

Thanks for helping build an independent bilingual Packet Tracer lab companion.

Open an issue describing the problem and expected behavior before large changes. Never upload private activities, courseware, answer keys, passwords or Packet Tracer binaries. Use original synthetic XML fixtures, or provide aggregate diagnostics with permission. Security issues belong in the private reporting process described in SECURITY.md.

Use stable Rust on Windows with MSVC Build Tools and Windows SDK. Run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked` and `cargo build --release --locked`. Keep changes scoped and add regression tests for behavioral fixes. Unsupported checks must remain Unverified; commands must reject ambiguous/sentinel requirements.

Source contributions must be original or clearly attributed under compatible licenses. Do not port restrictively licensed source. Contributions are submitted under the project's MIT license unless explicitly agreed otherwise. Preserve upstream notices for third-party material. Translation contributions belong in locales JSON and must retain keys and placeholders.

Use feature branches and pull requests. Describe behavior, validation and limitations, update CHANGELOG.md, and disclose AI assistance when relevant to provenance review. No live write feature may ship without explicit user confirmation and offline use must remain independent.
