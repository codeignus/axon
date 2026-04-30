use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

pub struct Tracer;

/// FFI: Initializes the global tracing subscriber.
/// Reads `RUST_LOG` env var for filter configuration; defaults to `info` level.
/// Returns a `Tracer` handle for subsequent log calls.
#[axon_export]
fn init_tracing() -> Tracer {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();
    Tracer
}

impl Tracer {
    /// FFI: Logs a message at INFO level.
    pub fn info(&self, msg: String) {
        info!("{msg}");
    }

    /// FFI: Logs a message at ERROR level.
    /// Named `err` because `error` is an Axon keyword.
    pub fn err(&self, msg: String) {
        error!("{msg}");
    }

    /// FFI: Logs a message at WARN level.
    pub fn warn(&self, msg: String) {
        warn!("{msg}");
    }

    /// FFI: Logs a message at DEBUG level.
    pub fn debug(&self, msg: String) {
        debug!("{msg}");
    }
}
