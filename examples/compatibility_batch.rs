//! Read-only batch probe. Use copies in an ignored folder; output contains no answer values.
use pka_lab_inspector::{commands, inspector};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

#[derive(Serialize)]
struct ResultRow {
    sample: String,
    decoded: bool,
    checkpoints: usize,
    points: f64,
    devices: usize,
    cli: usize,
    gui: usize,
    unsupported: usize,
    unverified: usize,
    xml_bytes: usize,
    milliseconds: f64,
    unsupported_properties: BTreeMap<String, usize>,
    unsupported_reasons: BTreeMap<String, usize>,
    value_shapes: BTreeMap<String, usize>,
}
fn main() -> anyhow::Result<()> {
    let directory = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("Supply a directory containing test copies"))?,
    );
    let mut files: Vec<_> = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("pka") || e.eq_ignore_ascii_case("pkt"))
        })
        .collect();
    files.sort();
    let mut rows = Vec::new();
    for path in files {
        let start = Instant::now();
        let mut row = ResultRow {
            sample: path.file_name().unwrap().to_string_lossy().into(),
            decoded: false,
            checkpoints: 0,
            points: 0.,
            devices: 0,
            cli: 0,
            gui: 0,
            unsupported: 0,
            unverified: 0,
            xml_bytes: 0,
            milliseconds: 0.,
            unsupported_properties: BTreeMap::new(),
            unsupported_reasons: BTreeMap::new(),
            value_shapes: BTreeMap::new(),
        };
        if let Ok(a) = inspector::load(&path) {
            row.decoded = true;
            let summary = a.summary();
            row.checkpoints = summary.total;
            row.points = summary.total_score;
            row.unverified = summary.unverified;
            row.xml_bytes = a.xml.len();
            row.devices = a
                .checkpoints
                .iter()
                .map(|c| &c.device)
                .filter(|d| !d.is_empty())
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            for c in &a.checkpoints {
                let shape = if c.expected.trim().is_empty() {
                    "empty"
                } else if matches!(c.expected.trim(), "-1" | "65535" | "4294967295") {
                    "sentinel"
                } else if c.expected.parse::<std::net::IpAddr>().is_ok() {
                    "IP address"
                } else if c.expected.parse::<i64>().is_ok() {
                    "integer"
                } else {
                    "text or structured"
                };
                *row.value_shapes.entry(shape.into()).or_default() += 1;
                let command = commands::generate(c);
                match (command.supported, command.gui) {
                    (true, false) => row.cli += 1,
                    (true, true) => row.gui += 1,
                    _ => {
                        row.unsupported += 1;
                        *row.unsupported_reasons
                            .entry(command.explanation)
                            .or_default() += 1;
                        *row.unsupported_properties
                            .entry(c.property.clone())
                            .or_default() += 1;
                    }
                }
            }
        }
        row.milliseconds = start.elapsed().as_secs_f64() * 1000.;
        rows.push(row);
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
