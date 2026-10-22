# Compatibility

Version 0.1.0 is an independent read-only lab inspector, not Cisco's grading engine. It supports the modern authenticated container decoded by MIT pktfile 0.2.2 with a local bounded-decompression patch. Unsupported/legacy containers fail with a diagnostic.

Real evidence: all six PKA samples bundled with an installed Packet Tracer 9.0.1 decoded/parsed; a private user-owned duplicate also loaded in the GUI. No activity files, courseware, decoded XML or private expected configuration is distributed. Counts are candidate tree selections based on an inferred Qt-like `checkType` mapping, not certified native assessment grades. The known 32-checkpoint/100-point/7-device baseline and a real 126-checkpoint activity have not been identified or certified. Original synthetic 126 checks cover structure/scoring and remain labeled synthetic.

Expected requirements come from explicit assessment values. Current state requires a uniquely identified explicit current snapshot or standalone saved PKT. Unlabelled embedded snapshots remain Unverified. Missing or conflicting data, custom/script grading and variables remain Unverified, not Failed. Manual review never changes automatic outcomes.

Validators cover explicit supported IPv4, subnet masks/prefixes, IPv6/prefixes, selected interface states and hostname fields. Full routing, VLAN, ACL, NAT, DHCP, custom grading, dynamic defaults and reachability validation are not implemented. Command grammars support additional families only when complete explicit context exists; this does not imply extraction or validation coverage for every activity schema. Unsupported properties receive no guessed command.

English/Vietnamese GUI localization is available with fallback. Unknown diagnostics, assessment names/evidence, technical identifiers and IOS exports may remain English or raw. Reports currently use English structural headings. Device scripts are for Cisco IOS CLI only; GUI-only/unsupported requirements are excluded from executable lines. Review omitted items individually.

See TEST_RESULTS.md for exact real/synthetic evidence, measurements and manual checks not completed.
