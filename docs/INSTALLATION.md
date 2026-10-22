# Installation

Download `packet-tracer-tool-v0.1.0-windows-x64.zip` and its `.sha256` file from https://github.com/thminh176/packet-tracer-tool/releases/tag/v0.1.0.

1. Verify the ZIP using `Get-FileHash .\packet-tracer-tool-v0.1.0-windows-x64.zip -Algorithm SHA256` and compare with the checksum.
2. Extract the full ZIP into a writable folder.
3. Run `pka-lab-inspector.exe`. Keep `locales` beside it for editable translations; embedded English/Vietnamese work if external assets are absent.
4. Open a supported activity. Load a saved network separately when the activity cannot establish current state. Review diagnostics before interpreting candidate counts/weight.
5. Copy one device's supported commands into **Packet Tracer ? device ? CLI**. They are Cisco IOS commands, not PowerShell commands. Configure PCs/servers through Packet Tracer GUI instructions.

The application is unsigned; Windows may show publisher reputation prompts. No installer, Python, internet connection or Packet Tracer runtime is required for offline inspection. Cisco Packet Tracer is separate software, required to simulate or edit labs.

Build from source with stable Rust, Visual Studio C++ Build Tools and Windows SDK: `cargo build --release --locked`. Run `powershell -File scripts/build-release.ps1` to check and package the release. Windows x86_64 is the verified target; other platforms are not release-tested.
