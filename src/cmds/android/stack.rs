use crate::diagnostics::{ClassifiedFrame, FrameOwnership, SourceLocation};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AndroidStackConfig {
    pub application_ids: Vec<String>,
    pub source_prefixes: Vec<String>,
    pub generated_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollapsedFrames {
    pub preserved: Vec<ClassifiedFrame>,
    pub collapsed: BTreeMap<FrameOwnership, usize>,
}

pub fn classify_frame(frame: &str, config: &AndroidStackConfig) -> FrameOwnership {
    let symbol = frame.trim().strip_prefix("at ").unwrap_or(frame.trim());
    if frame.trim_start().starts_with('#') || symbol.contains(".so") || symbol.contains("tombstone")
    {
        return FrameOwnership::Native;
    }
    if config
        .generated_prefixes
        .iter()
        .any(|prefix| symbol.starts_with(prefix))
    {
        return FrameOwnership::Generated;
    }
    if config
        .application_ids
        .iter()
        .chain(config.source_prefixes.iter())
        .any(|prefix| symbol.starts_with(prefix))
    {
        return FrameOwnership::Application;
    }
    if symbol.starts_with("kotlinx.coroutines.") || symbol.starts_with("kotlin.coroutines.") {
        return FrameOwnership::KotlinCoroutine;
    }
    if symbol.starts_with("android.")
        || symbol.starts_with("androidx.")
        || symbol.starts_with("java.")
        || symbol.starts_with("dalvik.")
        || symbol.starts_with("com.android.internal.")
    {
        return FrameOwnership::AndroidFramework;
    }
    if symbol.starts_with("org.gradle.")
        || symbol.starts_with("com.android.build.gradle.")
        || symbol.starts_with("org.jetbrains.kotlin.gradle.")
    {
        return FrameOwnership::GradlePlugin;
    }
    if frame.trim_start().starts_with("at ") {
        FrameOwnership::ThirdParty
    } else {
        FrameOwnership::Unknown
    }
}

pub fn collapse_frames<'a>(
    frames: impl IntoIterator<Item = &'a str>,
    config: &AndroidStackConfig,
) -> CollapsedFrames {
    let mut result = CollapsedFrames::default();
    for text in frames {
        let ownership = classify_frame(text, config);
        let frame = ClassifiedFrame {
            text: text.trim().to_string(),
            ownership,
            location: frame_location(text),
        };
        if matches!(
            ownership,
            FrameOwnership::Application
                | FrameOwnership::Generated
                | FrameOwnership::KotlinCoroutine
                | FrameOwnership::Native
                | FrameOwnership::Unknown
        ) {
            result.preserved.push(frame);
        } else {
            *result.collapsed.entry(ownership).or_default() += 1;
        }
    }
    result
}

fn frame_location(frame: &str) -> Option<SourceLocation> {
    lazy_static! {
        static ref LOCATION: Regex = Regex::new(r"\(([^():]+\.(?:kt|java)):(\d+)\)").unwrap();
    }
    let captures = LOCATION.captures(frame)?;
    Some(SourceLocation {
        file: captures[1].to_string(),
        line: captures[2].parse().ok()?,
        column: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::FrameOwnership;

    fn config() -> AndroidStackConfig {
        AndroidStackConfig {
            application_ids: vec!["com.example.app".into()],
            source_prefixes: vec!["com.example".into()],
            generated_prefixes: vec!["com.example.databinding".into(), "hilt_".into()],
        }
    }

    #[test]
    fn test_classify_android_stack_frame_ownership() {
        let cases = [
            ("at com.example.app.MainActivity.onCreate(MainActivity.kt:42)", FrameOwnership::Application),
            ("at com.example.databinding.ActivityMainBinding.inflate(ActivityMainBinding.java:8)", FrameOwnership::Generated),
            ("at android.app.Activity.performCreate(Activity.java:9000)", FrameOwnership::AndroidFramework),
            ("at kotlinx.coroutines.DispatchedTask.run(DispatchedTask.kt:104)", FrameOwnership::KotlinCoroutine),
            ("at org.gradle.api.internal.tasks.execution.ExecuteActionsTaskExecuter.execute(ExecuteActionsTaskExecuter.java:1)", FrameOwnership::GradlePlugin),
            ("#00 pc 000000000001 libexample.so", FrameOwnership::Native),
            ("at okhttp3.RealCall.execute(RealCall.kt:20)", FrameOwnership::ThirdParty),
        ];

        for (frame, expected) in cases {
            assert_eq!(classify_frame(frame, &config()), expected, "frame: {frame}");
        }
    }

    #[test]
    fn test_collapse_preserves_application_coroutine_and_native_frames() {
        let frames = [
            "at com.example.app.MainActivity.onCreate(MainActivity.kt:42)",
            "at android.app.Activity.performCreate(Activity.java:9000)",
            "at android.os.Handler.dispatchMessage(Handler.java:100)",
            "at kotlinx.coroutines.DispatchedTask.run(DispatchedTask.kt:104)",
            "#00 pc 000000000001 libexample.so",
        ];

        let collapsed = collapse_frames(frames, &config());

        assert_eq!(collapsed.preserved.len(), 3);
        assert!(collapsed
            .preserved
            .iter()
            .any(|frame| frame.ownership == FrameOwnership::Application));
        assert_eq!(collapsed.collapsed[&FrameOwnership::AndroidFramework], 2);
    }
}
