//! Generic filter layers applied to raw output before a command's own filter.
//! A command picks its `Layers`; the pipeline applies the enabled layers in
//! either captured (`run`) or streaming (`stream`) mode, command filter last.

mod decorative;
mod levels;

use crate::core::stream::StreamFilter;

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
            data = decorative::apply(&data, levels::current().decorative);
        }
        custom(&data)
    }

    pub fn stream<'a>(&self, inner: Box<dyn StreamFilter + 'a>) -> Box<dyn StreamFilter + 'a> {
        if self.layers.decorative {
            decorative::wrap_stream(inner, levels::current().decorative)
        } else {
            inner
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
