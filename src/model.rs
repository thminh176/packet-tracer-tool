use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Status {
    Passed,
    Failed,
    #[default]
    Unverified,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub device: String,
    pub device_type: String,
    pub interface: String,
    pub category: String,
    pub property: String,
    pub expected: String,
    pub current: Option<String>,
    pub weight: f64,
    pub path: String,
    pub status: Status,
    pub explanation: String,
    pub evidence: Vec<String>,
    #[serde(default)]
    pub command_classification: crate::cisco::Classification,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Analysis {
    pub checkpoints: Vec<Checkpoint>,
    pub diagnostics: Vec<String>,
    pub xml: String,
    pub timings: Vec<(String, f64)>,
    #[serde(default)]
    pub command_coverage: crate::cisco::mapping::CheckpointCoverage,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Command {
    pub supported: bool,
    pub text: String,
    pub explanation: String,
    pub gui: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Summary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub unverified: usize,
    pub verified_score: f64,
    pub total_score: f64,
    pub coverage: f64,
}
impl Analysis {
    pub fn summary(&self) -> Summary {
        let mut s = Summary {
            total: self.checkpoints.len(),
            ..Default::default()
        };
        for c in &self.checkpoints {
            s.total_score += c.weight;
            match c.status {
                Status::Passed => {
                    s.passed += 1;
                    s.verified_score += c.weight;
                }
                Status::Failed => s.failed += 1,
                Status::Unverified => s.unverified += 1,
            }
        }
        if s.total > 0 {
            s.coverage = (s.passed + s.failed) as f64 / s.total as f64;
        }
        s
    }
}
