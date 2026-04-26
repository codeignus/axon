use std::env;

#[axon_export]
fn cli_command() -> String {
    env::args().nth(1).unwrap_or_default()
}

#[axon_export]
fn cli_target() -> String {
    env::args().nth(2).unwrap_or_default()
}
