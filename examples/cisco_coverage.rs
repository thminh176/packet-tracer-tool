//! Reproducible synthetic coverage and warm operation timings; no private inputs.
use pka_lab_inspector::cisco::{self, CommandRequest, DeviceProfile};
use std::time::Instant;
fn main() {
    let specs = cisco::registry();
    println!(
        "# Generated Cisco command coverage\n\nSource: runtime registry. Packet Tracer execution-tested families: **0**.\n\n| Family | Parameters | Executable example | Semantic validator declared | Cisco source |\n|---|---:|---|---|---|"
    );
    let mut ready = 0;
    for s in &specs {
        let profile = if s.id.starts_with("switch.") {
            if s.capability == cisco::Capability::Multilayer {
                DeviceProfile::Catalyst3560
            } else {
                DeviceProfile::Catalyst2960
            }
        } else {
            DeviceProfile::Router1941
        };
        let req = cisco::example_request(s, profile);
        let p = cisco::generate(&req);
        if p.executable() {
            ready += 1;
        }
        println!(
            "| {} | {} | {} | {} | [reference]({}) |",
            s.id,
            s.parameters.len(),
            p.executable(),
            s.semantic_validation,
            s.source
        );
    }
    let start = Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(cisco::parser::parse(
            "hostname R1\ninterface GigabitEthernet0/0\n ip address 192.0.2.1 255.255.255.0\n!\nrouter ospf 20\n network 192.0.2.0 0.0.0.255 area 0\n!\n",
        ));
    }
    println!(
        "\nDocumented/recognized registry entries: {}; executable required-field examples: {ready}. Examples are synthetic, not grading or simulation certification.\n\nWarm synthetic config parsing: {:.3} ms / 1000 parses. Single-machine measurement, not a cross-machine guarantee.",
        specs.len(),
        start.elapsed().as_secs_f64() * 1000.
    );
    if let Some(s) = specs.first() {
        let req = CommandRequest {
            family: s.id.into(),
            device: "Synthetic-Device".into(),
            profile: DeviceProfile::Unknown,
            parameters: s
                .parameters
                .iter()
                .filter(|p| p.required)
                .map(|p| (p.name.to_string(), p.example.to_string()))
                .collect(),
        };
        let start = Instant::now();
        for _ in 0..1000 {
            std::hint::black_box(cisco::generate(&req));
        }
        println!(
            "Warm synthetic plan generation ({}): {:.3} ms / 1000 plans.",
            s.id,
            start.elapsed().as_secs_f64() * 1000.
        );
    }
}
