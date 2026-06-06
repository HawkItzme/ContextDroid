//! Generic filter layers applied to raw output before a command's own filter.
//! A command picks its `Layers`; the pipeline applies them in either captured
//! (`run`) or streaming (`stream`) mode, with the command's filter running last.

use crate::core::stream::StreamFilter;
use crate::core::utils::strip_ansi;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DecorativeLevel {
    Light,
    #[default]
    Reasonable,
    High,
}

impl DecorativeLevel {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" | "low" => Some(Self::Light),
            "reasonable" | "normal" | "default" | "medium" | "med" => Some(Self::Reasonable),
            "high" | "aggressive" => Some(Self::High),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Levels {
    pub decorative: DecorativeLevel,
}

/// Resolved once and cached: many call sites read levels, but config is read
/// from disk at most once to protect the <10ms startup budget.
pub fn levels() -> &'static Levels {
    use std::sync::OnceLock;
    static LEVELS: OnceLock<Levels> = OnceLock::new();
    LEVELS.get_or_init(resolve_levels)
}

fn resolve_levels() -> Levels {
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

/// Per-command, code-level choice of which generic layers run before the
/// command's own filter. Not user-configurable; the custom filter always runs.
#[derive(Clone, Copy, Debug)]
pub struct Layers {
    pub decorative: bool,
}

impl Default for Layers {
    fn default() -> Self {
        Self { decorative: true }
    }
}

pub struct Pipeline {
    layers: Layers,
}

impl Pipeline {
    pub fn for_layers(layers: Layers) -> Self {
        Self { layers }
    }

    pub fn run(&self, raw: &str, custom: impl Fn(&str) -> String) -> String {
        let mut data = raw.to_string();
        if self.layers.decorative {
            data = decorative(&data, levels().decorative);
        }
        custom(&data)
    }

    pub fn stream<'a>(&self, inner: Box<dyn StreamFilter + 'a>) -> Box<dyn StreamFilter + 'a> {
        if self.layers.decorative {
            Box::new(Decorating {
                inner,
                level: levels().decorative,
                prev_blank: false,
            })
        } else {
            inner
        }
    }
}

struct Decorating<'a> {
    inner: Box<dyn StreamFilter + 'a>,
    level: DecorativeLevel,
    prev_blank: bool,
}

impl StreamFilter for Decorating<'_> {
    fn feed_line(&mut self, line: &str) -> Option<String> {
        let clean = decorative_line(line, self.level, &mut self.prev_blank)?;
        self.inner.feed_line(&clean)
    }

    fn flush(&mut self) -> String {
        self.inner.flush()
    }

    fn on_exit(&mut self, exit_code: i32, raw: &str) -> Option<String> {
        self.inner.on_exit(exit_code, raw)
    }
}

pub fn decorative(input: &str, level: DecorativeLevel) -> String {
    if level == DecorativeLevel::Light {
        return strip_ansi(input);
    }

    let mut prev_blank = false;
    let mut out: Vec<String> = input
        .lines()
        .filter_map(|line| decorative_line(line, level, &mut prev_blank))
        .collect();

    while out.first().is_some_and(String::is_empty) {
        out.remove(0);
    }
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out.join("\n")
}

// None = line dropped (redundant blank, or decoration at High).
fn decorative_line(line: &str, level: DecorativeLevel, prev_blank: &mut bool) -> Option<String> {
    let stripped = strip_ansi(line);
    if level == DecorativeLevel::Light {
        return Some(stripped);
    }
    let trimmed = stripped.trim_end();
    if trimmed.is_empty() {
        if *prev_blank {
            return None;
        }
        *prev_blank = true;
        return Some(String::new());
    }
    if level == DecorativeLevel::High && is_decoration_line(trimmed) {
        return None;
    }
    *prev_blank = false;
    Some(trimmed.to_string())
}

fn is_decoration_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|c| is_decoration_char(c) || c.is_whitespace())
}

// Box Drawing + Block Elements; ASCII rules (---, ===) are left alone since they
// often carry meaning.
fn is_decoration_char(c: char) -> bool {
    matches!(c, '\u{2500}'..='\u{259F}') || matches!(c, '•' | '·')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_strips_ansi_only() {
        let out = decorative("\x1b[32mok\x1b[0m\n\n\ntrailing   ", DecorativeLevel::Light);
        assert_eq!(out, "ok\n\n\ntrailing   ");
    }

    #[test]
    fn reasonable_collapses_blanks_and_trims() {
        let out = decorative("a\n\n\n\nb   \n", DecorativeLevel::Reasonable);
        assert_eq!(out, "a\n\nb");
    }

    #[test]
    fn reasonable_strips_ansi() {
        assert_eq!(
            decorative("\x1b[1mbold\x1b[0m", DecorativeLevel::Reasonable),
            "bold"
        );
    }

    #[test]
    fn high_drops_box_drawing_lines() {
        let out = decorative("header\n──────────\nbody\n│ kept │", DecorativeLevel::High);
        assert_eq!(out, "header\nbody\n│ kept │");
    }

    #[test]
    fn high_preserves_ascii_rules() {
        let out = decorative("title\n-----\n===\nbody", DecorativeLevel::High);
        assert_eq!(out, "title\n-----\n===\nbody");
    }

    #[test]
    fn parse_accepts_known_values() {
        assert_eq!(
            DecorativeLevel::parse("light"),
            Some(DecorativeLevel::Light)
        );
        assert_eq!(
            DecorativeLevel::parse("REASONABLE"),
            Some(DecorativeLevel::Reasonable)
        );
        assert_eq!(
            DecorativeLevel::parse(" High "),
            Some(DecorativeLevel::High)
        );
        assert_eq!(DecorativeLevel::parse("bogus"), None);
    }

    #[test]
    fn run_applies_layers_then_custom() {
        let out = Pipeline::for_layers(Layers::default()).run("\x1b[32mx\x1b[0m\ny", |s| {
            format!("[{}]", s.replace('\n', "|"))
        });
        assert_eq!(out, "[x|y]");
    }

    #[test]
    fn run_without_layers_passes_raw_to_custom() {
        let raw = "\x1b[32mx\x1b[0m";
        let out = Pipeline::for_layers(Layers { decorative: false }).run(raw, |s| s.to_string());
        assert_eq!(out, raw);
    }

    struct Echo;
    impl StreamFilter for Echo {
        fn feed_line(&mut self, line: &str) -> Option<String> {
            Some(line.to_string())
        }
        fn flush(&mut self) -> String {
            String::new()
        }
    }

    #[test]
    fn stream_decorates_lines_before_inner() {
        let mut f = Pipeline::for_layers(Layers::default()).stream(Box::new(Echo));
        let out = f.feed_line("\x1b[32mok\x1b[0m").unwrap();
        assert!(!out.contains('\x1b') && out.contains("ok"));
    }

    #[test]
    fn stream_without_layers_is_passthrough() {
        let mut f = Pipeline::for_layers(Layers { decorative: false }).stream(Box::new(Echo));
        assert_eq!(
            f.feed_line("\x1b[32mok\x1b[0m"),
            Some("\x1b[32mok\x1b[0m".to_string())
        );
    }

    #[test]
    fn empty_input_is_empty() {
        assert_eq!(decorative("", DecorativeLevel::High), "");
    }
}
