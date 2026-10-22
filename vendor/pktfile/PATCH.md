# Local pktfile 0.2.2 patch

Source: crates.io pktfile 0.2.2, https://github.com/TantiE100/pktctl, MIT (see LICENSE).
The original MIT source is retained here to make this release reproducible.
Application-owned changes: inflate rejects short prefixes, caps declared XML at 64 MiB, limits zlib reads to the declared size plus one byte and rejects mismatches rather than truncating decoded bytes. Unused example/test manifest entries were removed. The quick-xml dependency is updated from 0.38 to 0.41 to resolve RUSTSEC-2026-0194 and RUSTSEC-2026-0195. No cryptographic changes were made. This patch is solely a resource and corruption-handling improvement.
