//! Local comparison utility: no expected values or configuration secrets are printed.
use pka_lab_inspector::{commands, inspector};
use std::collections::BTreeMap;
fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Supply a permitted test copy"))?;
    let a = inspector::load(std::path::Path::new(&path))?;
    if std::env::args().nth(2).as_deref() == Some("--checkpoint-ids") {
        let rows: Vec<_> = a.checkpoints.iter().map(|c| {
            let command = commands::generate(c);
            serde_json::json!({"id":c.id,"device":c.device,"interface":c.interface,"property":c.property,"cli":command.supported && !command.gui,"gui":command.supported && command.gui})
        }).collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    let mut devices = BTreeMap::<String, [usize; 4]>::new();
    for c in &a.checkpoints {
        let n = devices.entry(c.device.clone()).or_default();
        n[0] += 1;
        let command = commands::generate(c);
        if command.supported {
            n[if command.gui { 2 } else { 1 }] += 1;
        } else {
            n[3] += 1;
        }
    }
    println!("{}", serde_json::to_string_pretty(&devices)?);
    Ok(())
}
