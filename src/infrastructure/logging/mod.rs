//! Logging Infrastructure
//!
//! Tracing setup and configuration.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use crate::infrastructure::config::Settings;

/// Initialize the tracing/logging infrastructure
pub fn init(settings: &Settings) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&settings.log.level));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // Use JSON format in production, pretty format in development
    if settings.log.format == "json" {
        let registry = tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer.json());
        #[cfg(test)]
        {
            let _ = registry.try_init();
        }
        #[cfg(not(test))]
        {
            registry.init();
        }
    } else {
        let registry = tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer);
        #[cfg(test)]
        {
            let _ = registry.try_init();
        }
        #[cfg(not(test))]
        {
            registry.init();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_init_json_and_pretty() {
        let mut settings = Settings::default();
        settings.log.format = "json".to_string();
        init(&settings).expect("json init");

        settings.log.format = "pretty".to_string();
        init(&settings).expect("pretty init");
    }
}
