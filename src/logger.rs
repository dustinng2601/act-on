//! Logger / tracing setup.

use std::io::IsTerminal;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialise the global tracing subscriber.
///
/// `RUST_LOG` is respected automatically. When `json` is true, structured
/// JSON logs are emitted; otherwise human-readable ANSI logs.
pub fn init(json: bool) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("act_on=info,warn"));

    let registry = tracing_subscriber::registry().with(filter);

    let stdout_layer = if std::io::stdout().is_terminal() && !json {
        Some(fmt::layer().with_ansi(true).with_target(false))
    } else {
        Some(fmt::layer().with_ansi(false).with_target(false))
    };

    if let Some(layer) = stdout_layer {
        registry.with(layer).init();
    }
}
