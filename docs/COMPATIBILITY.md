# Compatibility

Version 0.1.1 is an independent read-only lab inspector, not Cisco's grading engine. It supports the modern authenticated container decoded by MIT pktfile 0.2.2 with a local bounded-decompression patch. Unsupported/legacy containers fail with a diagnostic.

Real evidence: all six PKA samples bundled with an installed Packet Tracer 9.0.1 decoded/parsed; a private user-owned duplicate also loaded in the GUI. No activity files, courseware, decoded XML or private expected configuration is distributed. Counts are candidate tree selections based on an inferred Qt-like `checkType` mapping, not certified native assessment grades. The known 32-checkpoint/100-point/7-device baseline and a real 126-checkpoint activity have not been identified or certified. Original synthetic 126 checks cover structure/scoring and remain labeled synthetic.

Expected requirements come from explicit assessment values. Current state requires a uniquely identified explicit current snapshot or standalone saved PKT. Unlabelled embedded snapshots remain Unverified. Missing or conflicting data, custom/script grading and variables remain Unverified, not Failed. Manual review never changes automatic outcomes.

## Validators and Command Generation in v0.1.1

Validators and command generators cover:
- **IPv4 Address & Subnet Masks**: Explicit IPv4, dotted subnet masks, prefix lengths (0..=32), and hyphenated prefix notation (`IP-prefix`). Host bits are zeroed with confidence grading (`High`, `Medium`, `Low`).
- **IPv4 Static Routes**: Packet Tracer's internal hyphenated multi-part format (`destination-prefix-nexthop_or_interface-unknown-distance`, such as `172.16.128.0-17-172.18.192.1-0-1` and default route `0.0.0.0-0-Serial0/1/0-0-1`) is parsed and normalized into canonical Cisco IOS `ip route <network> <mask> <next-hop/interface> [distance]` commands. Sibling assessment context (`commands:route-next-hop=`) completes partially specified routes.
- **IPv6 Static Routes**: Hyphenated IPv6 static route format (`prefix-prefixlen-nexthop_or_interface-unknown-distance` and default route `::-0-...`) is normalized into `ipv6 route <prefix>/<len> <next-hop/interface> [distance]`.
- **OSPF Network Statements**: 3-token and 4-token OSPF network statements (`[process_id] <network> <wildcard> <area>`) are parsed and normalized. Subnet masks are inverted to wildcard masks, area representations (e.g. `0.0.0.0` vs `0`) are normalized, and sibling context (`commands:ospf-process=`, `commands:ospf-area=`) is incorporated into `router ospf <pid>` contexts.
- **Auto Check Diagnostics**: When assessing against device `running-config`, multi-line matching is performed for static routes and OSPF networks. If no configuration exists on the device, explicit failure diagnostics are reported (e.g. "No IPv4 static routes configured on device", "No OSPF network statements configured on device"). If assessment checkpoints lack necessary parameters, clear diagnostic messages identify the missing fields.
- **Interface State & Hostnames**: Selected interface power/shutdown states, loopbacks, and hostname distinctions.

Full dynamic routing convergence, metric computation, VLAN, ACL, NAT, DHCP, custom grading, dynamic defaults and reachability validation are not implemented. Command grammars support additional families only when complete explicit context exists; this does not imply extraction or validation coverage for every activity schema. Unsupported properties receive no guessed command.

English/Vietnamese GUI localization is available with fallback. Unknown diagnostics, assessment names/evidence, technical identifiers and IOS exports may remain English or raw. Reports currently use English structural headings. Device scripts are for Cisco IOS CLI only; GUI-only/unsupported requirements are excluded from executable lines. Review omitted items individually.

See TEST_RESULTS.md for exact real/synthetic evidence, measurements and manual checks not completed.
