use std::str::FromStr;

use tracing::info;
use tracing_subscriber::{filter::Directive, prelude::*, EnvFilter};

use crate::{PKG_NAME, PKG_VERSION};

pub fn init() {
    let level = if cfg!(debug_assertions) { "debug" } else { "warn" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level))
        .add_directive(Directive::from_str("calloop=error").unwrap())
        .add_directive(Directive::from_str(&format!("smithay={level}")).unwrap())
        .add_directive(Directive::from_str(&format!("{PKG_NAME}={level}")).unwrap());
    let fmt_layer = tracing_subscriber::fmt::layer().compact();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter)
        .init();
    if cfg!(debug_assertions) {
        log_panics::init()
    } else {
        log_panics::Config::new()
            .backtrace_mode(log_panics::BacktraceMode::Off)
            .install_panic_hook()
    }

    info!("{PKG_NAME} {PKG_VERSION}");
}
