//! User-facing aggressivity per layer, resolved once from env/config.

use super::decorative::DecorativeLevel;

#[derive(Clone, Copy, Debug, Default)]
pub struct Levels {
    pub decorative: DecorativeLevel,
}

/// Resolved once and cached: many call sites read levels, but config is read
/// from disk at most once to protect the <10ms startup budget.
pub fn current() -> &'static Levels {
    use std::sync::OnceLock;
    static LEVELS: OnceLock<Levels> = OnceLock::new();
    LEVELS.get_or_init(resolve)
}

fn resolve() -> Levels {
    let decorative = std::env::var("RTK_DECORATIVE")
        .ok()
        .and_then(|v| DecorativeLevel::parse(&v))
        .or_else(|| {
            crate::core::config::Config::load()
                .ok()
                .and_then(|c| DecorativeLevel::parse(&c.levels.decorative))
        })
        .unwrap_or_default();
    Levels { decorative }
}
