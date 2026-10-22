//! Read-only local compatibility and timing probe; no decoded answer data is exported.
use pka_lab_inspector::inspector;
use std::{path::Path, time::Instant};
fn main() -> anyhow::Result<()> {
 let args:Vec<String>=std::env::args().skip(1).collect();
 if args.is_empty() { anyhow::bail!("Usage: cargo run --release --example inspect -- <file.pka|file.pkt|file.xml> [saved-progress]"); }
 let begin=Instant::now();
 let mut a=inspector::load(Path::new(&args[0]))?;
 if let Some(progress)=args.get(1) { let p=inspector::load(Path::new(progress))?; inspector::compare(&mut a,&p); }
 println!("{}",serde_json::to_string_pretty(&a.summary())?);
 println!("Elapsed: {:.3} ms; decoded XML: {} bytes",begin.elapsed().as_secs_f64()*1000.,a.xml.len());
 for (stage,ms) in &a.timings { println!("{stage}: {ms:.3} ms"); }
 for d in &a.diagnostics { println!("Diagnostic: {d}"); }
 Ok(())
}
