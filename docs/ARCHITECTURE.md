# Architecture

The application is a single Rust executable. `model` defines serializable checkpoints, validation status, evidence, commands, and score summaries. `inspector` performs read-only decoding, XML parsing, recursive assessment extraction, snapshot identification, and conservative comparison. `commands` maps supported explicit requirements to context-aware IOS or Packet Tracer GUI instructions. `reports` escapes CSV and Markdown output. The eframe GUI owns navigation, background load jobs, preferences, and manual checklist state.

Dependency direction: GUI ? inspector / commands / reports ? shared model. XML is retained for inspection. Parsing happens away from the UI thread; completed analysis is delivered through a channel. Automatic validation and manual checklist completion are separate stores. Only Passed contributes to verified score; Failed and Unverified remain distinct. Coverage counts verified passing and failing checks divided by all extracted checks.

Unknown assessment leaves must remain visible. Snapshot roles require structural or explicit evidence rather than sibling order. Validators must retain explanations and evidence. Command support is narrower than extraction: insufficient context, unknown enum meanings, sentinel values and unsafe text produce an unsupported result instead of guessed commands. Scripts group commands by target device and preserve configuration contexts.

## Original implementation and boundaries

The XML extraction, validation, commands, reports, and UI are original Rust code. Container decoding uses the MIT-licensed pktfile dependency. No encoder is exposed by the application. Source files are never overwritten. Export paths must remain separate from input paths. PKA variants outside the dependency codec are reported as unsupported; encrypted activity passwords are not handled.

## Performance

Analysis records available decode, parsing, extraction and comparison timings. The release probe measures end-to-end duration and XML payload size without exporting decoded answers. See test results for measured values, memory/startup methodology, and remaining measurements.

## v0.2 architecture

cisco/mod.rs defines typed CommandSpec, ParameterSpec, CommandRequest, ConfigMode, CommandPlan, support states and the canonical catalog. basic.rs contains shared fundamentals; switch and router modules own bounded family handlers and source metadata. DeviceProfile identifies explicit model evidence; missing models remain generic/unknown, contradictory models stop suggestions.

The catalog is initialized once and borrowed by hot generation/mapping paths. Public registry clones are convenient for GUI exploration. Measurement before caching found 26.469 ms per 1000 warm synthetic basic plans; the final generated report records current measurements. Example requests are constructed by an explicitly named synthetic-learning helper and never fill assessment requirements.

mapping connects explicit properties/path/sibling evidence to typed requests while retaining raw requirements. Command readiness and configuration validation are distinct fields on each checkpoint; aggregate command coverage is computed during background parsing, not on each UI frame. Legacy supported grammars remain guarded adapters while new families move into the registry incrementally.

parser preserves global/interface/router/VLAN/ACL/DHCP/line hierarchy and order, rejects opaque/duplicate mode identities and bounds configuration work. validator handles a deliberately small explicit scalar subset. inspector supplies unique snapshot/device proof before using validators or proving absence; uncertain defaults and contexts stay Unverified. Command generation never executes commands. Reports expose command readiness separately from automatic status and score.

## Protocol controls and discovery expansion (0.2.2-dev.2)

NTP handling lives in src/cisco/basic/time.rs; classic RIP/EIGRP options in router/protocol_controls.rs; Catalyst LLDP in switch/lldp.rs. Mixed IPv4/IPv6 inputs use the typed IpAddress parameter kind. Conditional requirements (IPv6 NTP version 4) remain visible as missing parameters rather than inserted defaults.

commands/assessment.rs resolves generic leaf names only below a unique device/protocol category, preserving raw XML labels and rejecting mixed/IPv6-ambiguous contexts. Process and AS parameters remain explicit. validator/controls.rs compares one scoped configured scalar through the same typed validation contract, without inferring defaults, neighbor state or effective router ID.

commands/conflicts.rs handles global-versus-port discovery contradictions and canonical interface identities. Device scripts preserve independent passive ports and NTP server addresses; completed contradictory blocks are omitted. The Library clears its draft only when changing families so old hostname or protocol parameters cannot contaminate a new request.