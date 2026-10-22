//! Private local diagnostic: prints only an explicitly selected checkpoint.
use pka_lab_inspector::{commands, inspector};
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 4,
        "Usage: inspect_checkpoint <test-copy> <device> <interface> <property>"
    );
    let a = inspector::load(std::path::Path::new(&args[0]))?;
    let matches: Vec<_> = a
        .checkpoints
        .iter()
        .filter(|c| c.device == args[1] && c.interface == args[2] && c.property == args[3])
        .collect();
    anyhow::ensure!(
        matches.len() == 1,
        "Expected one uniquely selected checkpoint, found {}",
        matches.len()
    );
    let c = matches[0];
    let command = commands::generate(c);
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"property":c.property,"device":c.device,"interface":c.interface,"expected":c.expected,"status":c.status,"command":command})
        )?
    );
    Ok(())
}
