use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

pub struct Tracer;

impl Tracer {
    pub fn info(&self, msg: String) {
        info!("{msg}");
    }

    pub fn error(&self, msg: String) {
        error!("{msg}");
    }

    pub fn warn(&self, msg: String) {
        warn!("{msg}");
    }

    pub fn debug(&self, msg: String) {
        debug!("{msg}");
    }
}

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
