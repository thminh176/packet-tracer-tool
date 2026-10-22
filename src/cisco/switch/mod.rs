pub fn specs() -> Vec<super::CommandSpec> {
    vec![]
}
pub fn generate(_: &super::CommandRequest) -> Result<Vec<super::ModeCommand>, String> {
    Err("Unknown family".into())
}
