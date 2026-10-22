//! Local schema probe: omits expected values, device names and the leading device path.
use pka_lab_inspector::inspector;
fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Supply a test copy"))?;
    let a = inspector::load(std::path::Path::new(&path))?;
    for c in a.checkpoints.iter().take(200) {
        println!(
            "{} | {} | {} | {} | {}",
            c.device_type,
            c.property,
            c.path
                .split(" / ")
                .skip(2)
                .map(|part| if part.parse::<std::net::IpAddr>().is_ok() {
                    "<address>"
                } else {
                    part
                })
                .collect::<Vec<_>>()
                .join(" / "),
            if c.expected.parse::<std::net::Ipv6Addr>().is_ok() {
                "IPv6"
            } else if c.expected.parse::<u32>().is_ok() {
                "integer"
            } else {
                "other"
            },
            c.evidence
                .iter()
                .map(|e| e.split('=').next().unwrap_or(""))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    Ok(())
}
