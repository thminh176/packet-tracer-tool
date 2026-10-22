# Route labels and OSPF context, 2026-10-23

`Route0` is an assessment label, not proof of a static route. The v0.1.1 detector tested this label before its OSPF branch, so one checkpoint could be both an OSPF network and an IPv4 static route. Value-shaped UI fallbacks also allowed unrelated address values to acquire a routing badge.

The corrected detectors exclude recognized dynamic protocol branches from static routing, identify OSPF network leaves by protocol context, and distinguish indexed `RouteN` from `Router ID`. Protocol words inside a device name do not identify a routing branch. The table adds an explicit protocol badge while preserving the original property and assessment path. UI labels no longer classify an unrelated address merely because its value can be parsed by a route parser.

The inspector now resolves explicit process/area/sibling evidence consistently before semantic routing comparison. It still requires a unique current snapshot and uniquely owned device configuration. Missing process IDs remain Unverified and cannot silently become process 1 or a number taken from an ancestor label.

For incomplete OSPF requirements, the details panel can transfer the explicit network/area and target device to the Command Library. The user supplies the missing process ID there. No synthetic example is loaded by this action, no grade is changed and no command executes. Conflicting or unsupported assessment evidence blocks this shortcut.

## Evidence

An aggregate-only probe ran against the same permitted private copy using v0.1.1 and the corrected candidate: both decoded 213 selected checkpoints and recognized ten OSPF network leaves. The old detector also classified all ten as static; the new detector classifies zero of them as static. None has sufficient verified process evidence for automatic complete command generation. No activity names, addresses, decoded XML or answer configurations are published by the probe.

Synthetic regressions cover OSPF classification and generation with explicit process evidence, dynamic protocol exclusion, device-name isolation, IPv6 static routes, process-scoped saved-config comparison, missing-process behavior, UI badges and library parameter transfer. The complete candidate passes 199 application tests plus 18 codec tests. These are inspection/generation tests, not Packet Tracer CLI execution certification.

The rebuilt Windows GUI also opened the permitted copy, filtered OSPF, showed all ten network rows with the OSPF badge and opened the selected requirement in `router.ospf.network` in the Library. The transferred form retained an empty process field, explicitly reported that missing parameter and withheld an executable script. This check did not insert a guessed process, run device commands or alter the original activity. Its screenshots and tree captures remain private.

## Syntax reference

The [Cisco IOS 15M&T OSPF configuration guide](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/iproute_ospf/configuration/15-mt/iro-15-mt-book/iro-cfg.html) documents global `router ospf <process-id>` followed by router-mode `network <network> <wildcard> area <area>`. The separate static routing workflow uses `ip route`. These are concise independently written descriptions; the guide is linked rather than copied.
