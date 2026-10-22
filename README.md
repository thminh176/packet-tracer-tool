# PKA Lab Inspector

Native Rust desktop inspector for Cisco Packet Tracer activities and saved networks. Opens files read-only, retains assessment paths and weights, and distinguishes verified passes, verified failures, and unsupported or ambiguous checks. Works offline after installation; no Python or external decipher runtime.

## Build on Windows

Install stable Rust for x86_64-pc-windows-msvc and Visual Studio C++ Build Tools with the Windows SDK.

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
.\target\release\pka-lab-inspector.exe
```

## Workflow

Open an activity with the native file picker. Review the overview and diagnostics before interpreting the checkpoint table. Load a separate saved progress file to compare it with the activity. Select a device or status in the sidebar, search for a property, and select a checkpoint to inspect its evidence, raw assessment path, and command support explanation.

Copy individual commands or a grouped device script. PC and server settings use Packet Tracer GUI instructions where appropriate. Review the generated commands in context before applying them. Unsupported values remain explicit. Export CSV or Markdown reports and CLI scripts through the interface. Manual checklist state does not alter automatic scores.

## Compatibility and limits

See [test results](docs/TEST_RESULTS.md) for exactly which real and synthetic files were exercised. Support is schema-dependent. A successfully decoded container does not establish reliable validation of every assessment type. Unknown schemas and ambiguous network snapshot roles remain Unverified. The specific 32-checkpoint / 100-point / 7-device baseline must be tested with its actual file; no expected counts are hardcoded.

The decoder uses MIT-licensed pktfile 0.2.2, whose published contract is Packet Tracer PKT decoding. PKA compatibility is measured against local real files rather than assumed from the extension. No source was copied or translated from PKA Decipher or other restrictively licensed projects. No score manipulation, activity rewriting, or password bypass is implemented.

## Development

See [architecture](docs/ARCHITECTURE.md), [task graph](docs/MILESTONES.md), and [third-party notices](THIRD_PARTY_NOTICES.md). Real activity files and private answer data are excluded from version control. No remote push or release publication is part of the local build.

Read-only command-line probe:

```powershell
cargo run --release --example inspect -- "C:\path\activity.pka" "C:\path\saved.pkt"
```

## Ngôn ngữ / Languages

Ứng dụng hỗ trợ Tiếng Việt và English. Chọn ngôn ngữ trên thanh công cụ; lựa chọn được lưu. Để thêm ngôn ngữ, sao chép `locales/en.json`, dịch phần `messages`, đổi `locale` và `name`, rồi đặt tệp trong thư mục `locales` cạnh EXE. Có thể bấm tải lại ngôn ngữ; không cần sửa mã Rust hoặc biên dịch lại. Xem [hướng dẫn ngôn ngữ](docs/LOCALIZATION.md).

The portable local package is produced by `scripts/build-release.ps1`. It includes the EXE, external language catalogs, documentation, SHA-256 checksum and available third-party license texts. The GitHub Actions workflow checks and packages Windows artifacts without publishing a release.
