# Command correctness gate for the local v0.2 candidate

This candidate stays local until command and layout review is complete. Existing published releases are unchanged. A syntactically generated command is not proof of simulator acceptance, connectivity, or a grading result.

## Evidence order

1. Explicit activity requirements and unambiguous device/interface identities supply values. Examples never supply missing answers.
2. Match the command to official Cisco documentation for the appropriate device/software family. Classic IOS, IOS XE and IOS XR syntax are not interchangeable.
3. Check mode, required parameters and existing prerequisites through both the library API and assessment generation API. Check unsupported and contradictory inputs as well as successful cases.
4. Exercise batch ordering, repeated interface contexts, conflicting scalar requirements and ACL order.
5. Use a new synthetic Packet Tracer lab to verify actual command acceptance, resulting saved configuration and read-only show output. Record device and simulator versions before marking any family execution-tested.

Automated offline tests cover steps 1–4 in bounded subsets. Step 5 is a separate gate; no current family has that certification. A successful Rust test or Cisco documentation link must not change that label.

## Local PDF references

The user supplied four networking handbooks in Downloads: one 34-page IOS/IOS XE configuration guide, two six-page quick references and a 21-page practical device handbook. Only these networking documents were extracted locally for review. They are secondary references; their licensing and authorship have not been established. Neither the PDFs nor their extracted text belong in the public repository or release archive. Page numbers in scoped audit reports refer to PDF page positions.

The quick-reference handbook's page 2 distinguishes local users, line authentication and SSH prerequisites. Pages 3–4 distinguish L2 management gateways, routed ports, VLAN/trunk workflows and routing contexts. Examples in a handbook are examples, not activity requirements or evidence of Packet Tracer device support.

## Shared authentication correction

Generated plaintext credentials now include an explicit `0` discriminator after `secret`, so numeric or option-shaped secrets are not interpreted as encryption types or command options. This does not select a hashing algorithm or promise modern password-storage security. Existing hashes must not be passed through the plaintext workflow.

Primary references:

- [Cisco IOS Security Command Reference, enable secret](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/security/d1/sec-d1-cr-book/sec-cr-e1.html).
- [Cisco IOS Security Command Reference, username](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/security/s1/sec-s1-cr-book/sec-cr-t2.html).
- [Cisco IOS 15M&T SSH Version 2 configuration](https://www.cisco.com/c/en/us/td/docs/ios-xml/ios/sec_usr_ssh/configuration/15-mt/sec-usr-ssh-15-mt-book/sec-secure-shell-v2.html).
- [Cisco SSH router configuration and prerequisites](https://www.cisco.com/c/en/us/support/docs/security-vpn/secure-shell-ssh/4145-ssh.html).

The VTY workflow requires explicit existing SSH, local-user and AAA state. It does not silently regenerate RSA keys, change AAA or assume a reachable management address.

Scoped router, switch, independent safety and Library layout reports record the concrete changes and test limits for this review wave.
