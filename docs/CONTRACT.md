# Shared implementation contract

Library crate pka_lab_inspector. Models in src/model.rs are coordinator-owned.
- inspector::load(path: &std::path::Path) -> anyhow::Result<Analysis>
- inspector::parse(xml: &str) -> anyhow::Result<Analysis>
- inspector::compare(activity: &mut Analysis, progress: &Analysis): validate with evidence; unknown remains Unverified.
- commands::generate(checkpoint: &Checkpoint) -> Command
- commands::device_script(checkpoints: &[Checkpoint], device: Option<&str>) -> String
- reports::csv(analysis: &Analysis) -> anyhow::Result<String>
- reports::markdown(analysis: &Analysis) -> String
GUI owns src/main.rs and src/ui.rs. Inspector owns src/inspector.rs and tests/inspector*. Commands owns src/commands.rs and tests/commands*. Coordinator owns all other files.
No PKA/PKT fixtures or private answer data in Git. No Python runtime, bypass, source copying, score editing, or positional snapshot assumptions.
