# Packet Tracer Tool v0.2.2 — Cisco Protocol Coverage

Independent offline Windows x64 Rust / egui lab companion. Not affiliated with or endorsed by Cisco.

## Changes

- 66 bounded command families: 12 common, 23 switching and 31 routing; 26 declared configured-state comparison subsets. These counts describe bounded workflows, not universal IOS support.
- Add IPv4/IPv6 NTP server and source workflows, LLDP global/interface/timers, explicit RIP version/passive/summary and classic EIGRP passive/configured-ID controls.
- Include DHCP relay, serial DCE clock, serial PPP, CDP and OSPF router-ID/passive/reference-bandwidth workflows.
- Fix IPv6 Unicast Routing and explicit dot1Q VLAN checkpoint interpretation. Never infer a VLAN from a subinterface suffix or a process/AS from numeric tree labels.
- Resolve generic assessment labels only within an unambiguous device/protocol scope. Compare configured routing controls only with uniquely owned current configuration evidence. Generic or active Router ID stays Unverified from running-config alone.
- Block contradictory grouped commands, preserve independent servers/passive ports and clear stale Library parameters when changing command families.
- Preserve English/Vietnamese interface, offline inspection, individual command copying and report exports.

## Tested scope and limitations

244 application tests pass; one opt-in private-fixture test remains ignored. All 18 vendored codec tests pass. Formatting, strict Clippy and dependency/security gates pass. Native Windows smoke tested the original public four-checkpoint synthetic fixture, Library search, explicit IPv6 NTP version gating and clipboard contents. See docs/TEST_RESULTS.md for detailed evidence.

No new command family is execution-certified on actual Packet Tracer devices. Synthetic tests and Cisco documentation are not simulator certification. NTP configuration does not prove synchronization; LLDP generation does not grade neighbors. Saved configuration cannot prove protocol convergence, forwarding or official activity scores. Missing or ambiguous evidence remains Unverified. Some diagnostic prose retains English fallback. Unknown device/image support carries warnings or blocks generation.

No private coursework, answer keys, Cisco binaries or logos are distributed. Earlier release tags remain intact.

## Windows installation

Download packet-tracer-tool-v0.2.2-windows-x64.zip and its .sha256 from this release. Verify the SHA-256, extract the complete folder, and launch pka-lab-inspector.exe. Keep the included locales, licenses and notices. The EXE is unsigned and may require the Microsoft Visual C++ x64 runtime. Generated commands are suggestions and are never executed automatically.
