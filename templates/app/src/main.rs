//! Entry point: set up diagnostics, then hand over to the library. Keep this file thin.

use anyhow::Result;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    {{crate_name}}::run()
}
