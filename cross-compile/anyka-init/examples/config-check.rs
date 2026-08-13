//! Parse an `anyka.toml` with the real supervisor parser and report the verdict.
//!
//! Exists because the failure mode it guards is unrecoverable: `main.rs:16-22`
//! parks on a config it cannot load, and `park()` never exits, so neither the
//! update trial nor `config.sh`'s 240-second deadman ever fires. On a camera
//! reachable only through the jumphost that is a site visit, so a device config
//! gets validated here before it is pushed, not after.
//!
//! An example rather than a `[[bin]]`: `cargo build` skips examples, so this
//! stays out of the ARM cross-build that `build_sd_contents.sh` runs.
//!
//! Usage: cargo run --example config-check --target x86_64-unknown-linux-gnu -- <path>

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: config-check <anyka.toml>");
        return std::process::ExitCode::FAILURE;
    };
    match anyka_init::config::Config::load(&path) {
        Ok(cfg) => {
            println!("OK   {path}  (schema={})", cfg.schema);
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            println!("FAIL {path}\n     {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
