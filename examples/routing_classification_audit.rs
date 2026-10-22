//! Aggregate-only compatibility probe; never prints activity answers or device names.
use pka_lab_inspector::{commands, inspector};
fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Supply a permitted copy"))?;
    let analysis = inspector::load(std::path::Path::new(&path))?;
    let mut ospf_rows = 0;
    let mut overlapping_static = 0;
    let mut command_ready = 0;
    for c in &analysis.checkpoints {
        if commands::is_ospf_network_property(&c.property, &c.path) {
            ospf_rows += 1;
            overlapping_static +=
                usize::from(commands::is_static_route_property(&c.property, &c.path));
            command_ready += usize::from(commands::generate(c).supported);
        }
    }
    println!(
        "{}",
        serde_json::json!({"total_checkpoints": analysis.checkpoints.len(), "ospf_network_checkpoints":ospf_rows, "incorrectly_overlapping_static":overlapping_static, "ospf_command_ready":command_ready})
    );
    Ok(())
}
