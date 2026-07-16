use anyhow::{bail, Result};

use crate::cmds::android::stack::AndroidStackConfig;
use crate::diagnostics::OutputMode;

#[derive(Debug, Clone)]
pub struct RuntimeContext {
    pub profile: String,
    pub output_mode: OutputMode,
    pub android: AndroidStackConfig,
}

impl RuntimeContext {
    pub fn resolve(profile: impl Into<String>, cli_mode: Option<OutputMode>) -> Result<Self> {
        let config = crate::core::config::Config::load().unwrap_or_default();
        let environment_mode = std::env::var("CONTEXTDROID_OUTPUT_MODE")
            .ok()
            .map(|value| parse_output_mode(&value))
            .transpose()?;
        Ok(Self {
            profile: profile.into(),
            output_mode: cli_mode.or(environment_mode).unwrap_or(config.output.mode),
            android: AndroidStackConfig {
                application_ids: config.android.application_ids,
                source_prefixes: config.android.source_prefixes,
                generated_prefixes: config.android.generated_prefixes,
            },
        })
    }
}

fn parse_output_mode(value: &str) -> Result<OutputMode> {
    match value {
        "lossless" => Ok(OutputMode::Lossless),
        "balanced" => Ok(OutputMode::Balanced),
        "aggressive" => Ok(OutputMode::Aggressive),
        _ => bail!("CONTEXTDROID_OUTPUT_MODE must be lossless, balanced, or aggressive"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn explicit_output_mode_precedes_environment() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("CONTEXTDROID_OUTPUT_MODE", "aggressive");
        let context =
            RuntimeContext::resolve("contextdroid-safe", Some(OutputMode::Lossless)).unwrap();
        std::env::remove_var("CONTEXTDROID_OUTPUT_MODE");
        assert_eq!(context.output_mode, OutputMode::Lossless);
    }

    #[test]
    fn invalid_environment_mode_fails_closed() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("CONTEXTDROID_OUTPUT_MODE", "surprising");
        let result = RuntimeContext::resolve("contextdroid-safe", None);
        std::env::remove_var("CONTEXTDROID_OUTPUT_MODE");
        assert!(result.is_err());
    }
}
