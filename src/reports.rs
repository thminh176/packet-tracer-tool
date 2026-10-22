use crate::model::Analysis;

pub fn csv(analysis: &Analysis) -> anyhow::Result<String> {
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record([
        "ID",
        "Device",
        "Type",
        "Interface",
        "Category",
        "Property",
        "Expected",
        "Current",
        "Points",
        "Status",
        "Path",
        "Explanation",
        "Evidence",
    ])?;
    for c in &analysis.checkpoints {
        let row = vec![
            c.id.clone(),
            c.device.clone(),
            c.device_type.clone(),
            c.interface.clone(),
            c.category.clone(),
            c.property.clone(),
            c.expected.clone(),
            c.current.clone().unwrap_or_default(),
            c.weight.to_string(),
            format!("{:?}", c.status),
            c.path.clone(),
            c.explanation.clone(),
            c.evidence.join("; "),
        ];
        // Prevent spreadsheet formula execution when opening reports in Excel.
        w.write_record(row.iter().map(|v| spreadsheet_safe(v)))?;
    }
    Ok(String::from_utf8(w.into_inner()?)?)
}
fn spreadsheet_safe(v: &str) -> String {
    if v.trim_start().starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{v}")
    } else {
        v.to_owned()
    }
}
fn md(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\\', "\\\\")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
        .replace('(', "&#40;")
        .replace(')', "&#41;")
        .replace('*', "&#42;")
        .replace('_', "&#95;")
        .replace('`', "&#96;")
        .replace('!', "&#33;")
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
}
pub fn markdown(a: &Analysis) -> String {
    let s = a.summary();
    let mut out = format!(
        "# PKA Lab Inspector report\n\n{} checkpoints: {} passed, {} failed, {} unverified.\n\nVerified score: {} / {}. Verification coverage: {:.1}%.\n\n| Device | Interface | Property | Expected | Current | Status | Points | Explanation |\n|---|---|---|---|---|---|---|---|\n",
        s.total,
        s.passed,
        s.failed,
        s.unverified,
        s.verified_score,
        s.total_score,
        s.coverage * 100.0
    );
    for c in &a.checkpoints {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:?} | {} | {} |\n",
            md(&c.device),
            md(&c.interface),
            md(&c.property),
            md(&c.expected),
            md(c.current.as_deref().unwrap_or("Unknown")),
            c.status,
            c.weight,
            md(&c.explanation)
        ));
    }
    out.push_str("\n## Diagnostics\n\n");
    for d in &a.diagnostics {
        out.push_str(&format!("- {}\n", md(d)));
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Checkpoint, Status};
    #[test]
    fn export_quotes_and_escapes() {
        let a = Analysis {
            checkpoints: vec![Checkpoint {
                device: "=1+1".into(),
                expected: "a,b\"c".into(),
                property: "<x>|y".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let c = csv(&a).unwrap();
        assert!(c.contains("'=1+1"));
        assert!(c.contains("\"a,b\"\"c\""));
        assert!(markdown(&a).contains("&lt;x&gt;\\|y"));
        assert!(!md("![x](https://example.invalid/a) [link](url)").contains("](https"));
        assert!(md("[link](url)").contains("&#91;link&#93;"));
    }
    #[test]
    fn score_keeps_unknown_separate() {
        let a = Analysis {
            checkpoints: vec![
                Checkpoint {
                    weight: 25.,
                    status: Status::Passed,
                    ..Default::default()
                },
                Checkpoint {
                    weight: 25.,
                    status: Status::Failed,
                    ..Default::default()
                },
                Checkpoint {
                    weight: 50.,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let s = a.summary();
        assert_eq!((s.passed, s.failed, s.unverified), (1, 1, 1));
        assert_eq!(s.verified_score, 25.);
        assert_eq!(s.total_score, 100.);
        assert!((s.coverage - 2. / 3.).abs() < 1e-9);
    }
}
