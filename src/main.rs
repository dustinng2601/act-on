//! act-on entry point.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match act_on::cli::run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("act-on: {e:#}");
            ExitCode::from(2)
        }
    }
}
