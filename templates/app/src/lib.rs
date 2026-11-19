//! {{description}}

use anyhow::Result;

/// Runs the application.
pub fn run() -> Result<()> {
    tracing::info!("{{project-name}} started");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn run_succeeds() {
        assert!(run().is_ok());
    }
}
