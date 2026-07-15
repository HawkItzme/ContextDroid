use crate::diagnostics::{
    assess_confidence, Cause, DiagnosticEvent, DiagnosticKind, DiagnosticRun, OmissionReport,
    ParseConfidence, ParserIdentity, Severity, SourceLocation,
};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

lazy_static! {
    static ref KOTLIN_LOCATION: Regex =
        Regex::new(r"^e:\s+(?:file://)?(.+):(\d+):(\d+)\s+(.+)$").unwrap();
    static ref JAVA_LOCATION: Regex = Regex::new(r"^(.+\.java):(\d+):\s+error:\s+(.+)$").unwrap();
    static ref LINT_LOCATION: Regex =
        Regex::new(r"^(.+?):(\d+)(?::(\d+))?:\s+(?:Error|Warning):\s+(.+?)(?:\s+\[[^]]+\])?$")
            .unwrap();
    static ref FAILED_TASK: Regex = Regex::new(r"^> Task (:\S+) FAILED$").unwrap();
    static ref EXECUTION_TASK: Regex =
        Regex::new(r#"^Execution failed for task ['"](:[^'"]+)['"]\.$"#).unwrap();
    static ref VARIANT: Regex = Regex::new(r"(?i)(debug|release)").unwrap();
}

pub fn parse(command: &str, raw: &str, exit_code: i32, run_id: &str) -> DiagnosticRun {
    let task = raw.lines().find_map(extract_failed_task);
    let causes: Vec<Cause> = raw
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Caused by: "))
        .map(|message| Cause {
            error_type: message.split_once(':').map(|(kind, _)| kind.to_string()),
            message: message.to_string(),
        })
        .collect();
    let successful_tasks = raw
        .lines()
        .filter(|line| line.starts_with("> Task ") && !line.ends_with(" FAILED"))
        .count();
    let stack = crate::cmds::android::stack::collapse_frames(
        raw.lines().filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("at ") || trimmed.starts_with('#')
        }),
        &Default::default(),
    );

    let mut events = Vec::new();
    for (raw_line, line) in raw.lines().enumerate() {
        let Some(kind) = classify_line(line) else {
            continue;
        };
        let (location, message) = parse_location_and_message(line);
        events.push(DiagnosticEvent {
            kind,
            severity: if line.to_ascii_lowercase().contains("warning") {
                Severity::Warning
            } else {
                Severity::Error
            },
            message,
            error_type: None,
            task: task.clone(),
            module: task.as_deref().and_then(module_from_task),
            variant: task.as_deref().and_then(variant_from_task),
            location,
            causes: causes.clone(),
            frames: Vec::new(),
            details: BTreeMap::new(),
            raw_line: Some(raw_line),
        });
    }

    if events.is_empty() && (!causes.is_empty() || task.is_some()) {
        let root = raw
            .lines()
            .map(str::trim)
            .find(|line| {
                line.starts_with("Execution failed for task")
                    || line.starts_with("> ")
                    || line.ends_with("Exception")
            })
            .unwrap_or("Gradle task failed");
        events.push(DiagnosticEvent {
            kind: DiagnosticKind::Gradle,
            severity: Severity::Error,
            message: root.to_string(),
            error_type: None,
            task: task.clone(),
            module: task.as_deref().and_then(module_from_task),
            variant: task.as_deref().and_then(variant_from_task),
            location: None,
            causes,
            frames: Vec::new(),
            details: BTreeMap::new(),
            raw_line: raw.lines().position(|line| line.trim() == root),
        });
    }

    let confidence =
        if exit_code == 0 && raw.lines().any(|line| line.starts_with("BUILD SUCCESSFUL")) {
            ParseConfidence::High
        } else {
            assess_confidence(exit_code != 0, &events, false)
        };
    let mut omissions = OmissionReport::default();
    if successful_tasks > 0 {
        omissions
            .collapsed
            .insert("successful Gradle tasks".into(), successful_tasks);
    }
    let error_count = events
        .iter()
        .filter(|event| event.severity == Severity::Error)
        .count();
    if error_count > 0 {
        omissions
            .preserved
            .insert("root errors".into(), error_count);
    }
    if !events.is_empty() {
        events[0].frames = stack.preserved;
        for (ownership, count) in stack.collapsed {
            omissions
                .collapsed
                .insert(format!("{:?} frames", ownership), count);
        }
        let cause_count = events.first().map_or(0, |event| event.causes.len());
        if cause_count > 0 {
            omissions
                .preserved
                .insert("caused-by exceptions".into(), cause_count);
        }
        let locations = events
            .iter()
            .filter(|event| event.location.is_some())
            .count();
        if locations > 0 {
            omissions
                .preserved
                .insert("source locations".into(), locations);
        }
    }

    DiagnosticRun {
        run_id: run_id.to_string(),
        command: command.to_string(),
        parser: ParserIdentity {
            name: "android-gradle".into(),
            version: 1,
        },
        confidence,
        events,
        omissions,
    }
}

pub fn classify_line(line: &str) -> Option<DiagnosticKind> {
    let lower = line.to_ascii_lowercase();
    if lower.contains("[ksp]") || lower.contains("symbol processing") {
        Some(DiagnosticKind::Ksp)
    } else if lower.contains("kapt") || lower.contains("annotation processing failed") {
        Some(DiagnosticKind::Kapt)
    } else if lower.contains("compose compiler") || lower.contains("@composable invocations") {
        Some(DiagnosticKind::ComposeCompiler)
    } else if lower.contains("manifest merger failed") {
        Some(DiagnosticKind::ManifestMerger)
    } else if lower.contains("mergeresources") || lower.contains("duplicate resources") {
        Some(DiagnosticKind::ResourceMerge)
    } else if lower.contains("aapt") || lower.contains("resource linking failed") {
        Some(DiagnosticKind::Aapt2)
    } else if lower.contains("could not resolve") || lower.contains("could not find") {
        Some(DiagnosticKind::DependencyResolution)
    } else if lower.contains("duplicate class") {
        Some(DiagnosticKind::DuplicateClass)
    } else if lower.starts_with("d8:") || lower.contains(" dex file") {
        Some(DiagnosticKind::D8)
    } else if lower.starts_with("r8:") || lower.contains("r8 compilation") {
        Some(DiagnosticKind::R8)
    } else if lower.contains("instrumentation_failed")
        || lower.contains("instrumentation result: shortmsg")
    {
        Some(DiagnosticKind::InstrumentationTest)
    } else if lower.contains(" > ") && lower.ends_with(" failed") {
        Some(DiagnosticKind::UnitTest)
    } else if LINT_LOCATION.is_match(line) && line.contains('[') {
        Some(DiagnosticKind::Lint)
    } else if lower.contains(".java:") && lower.contains("error:") {
        Some(DiagnosticKind::JavaCompiler)
    } else if (lower.starts_with("e: ") && (lower.contains(".kt:") || lower.contains(".kt")))
        || lower.contains("kotlin error")
    {
        Some(DiagnosticKind::KotlinCompiler)
    } else {
        None
    }
}

fn extract_failed_task(line: &str) -> Option<String> {
    FAILED_TASK
        .captures(line.trim())
        .or_else(|| EXECUTION_TASK.captures(line.trim()))
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
}

fn module_from_task(task: &str) -> Option<String> {
    let mut parts: Vec<&str> = task.trim_matches(':').split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    parts.pop();
    Some(format!(":{}", parts.join(":")))
}

fn variant_from_task(task: &str) -> Option<String> {
    VARIANT
        .captures(task)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_ascii_lowercase())
}

fn parse_location_and_message(line: &str) -> (Option<SourceLocation>, String) {
    if let Some(captures) = KOTLIN_LOCATION.captures(line) {
        return (
            Some(SourceLocation {
                file: captures[1].to_string(),
                line: captures[2].parse().unwrap_or_default(),
                column: captures[3].parse().ok(),
            }),
            captures[4].to_string(),
        );
    }
    if let Some(captures) = JAVA_LOCATION.captures(line) {
        return (
            Some(SourceLocation {
                file: captures[1].to_string(),
                line: captures[2].parse().unwrap_or_default(),
                column: None,
            }),
            captures[3].to_string(),
        );
    }
    if let Some(captures) = LINT_LOCATION.captures(line) {
        return (
            Some(SourceLocation {
                file: captures[1].to_string(),
                line: captures[2].parse().unwrap_or_default(),
                column: captures
                    .get(3)
                    .and_then(|value| value.as_str().parse().ok()),
            }),
            captures[4].to_string(),
        );
    }
    let message = line
        .trim()
        .strip_prefix("e: [ksp] ")
        .or_else(|| line.trim().strip_prefix("kapt: error: "))
        .unwrap_or(line.trim())
        .to_string();
    (None, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{DiagnosticKind, ParseConfidence, Severity};

    #[test]
    fn test_parse_kotlin_failure_preserves_task_location_message_and_causes() {
        let raw = r#"> Task :app:compileDebugKotlin FAILED
e: file:///workspace/app/src/main/java/com/example/Main.kt:42:17 Unresolved reference: missingApi
FAILURE: Build failed with an exception.
* What went wrong:
Execution failed for task ':app:compileDebugKotlin'.
> Compilation error. See log for more details
Caused by: org.jetbrains.kotlin.gradle.tasks.CompilationErrorException: Compilation error
Caused by: java.lang.IllegalStateException: compiler stopped
BUILD FAILED in 2s
12 actionable tasks: 2 executed, 10 up-to-date
"#;

        let run = parse(
            "./gradlew :app:assembleDebug",
            raw,
            1,
            "20260715T010203.004Z-a1b2c3d4",
        );

        assert_eq!(run.confidence, ParseConfidence::High);
        let event = run.events.first().unwrap();
        assert_eq!(event.kind, DiagnosticKind::KotlinCompiler);
        assert_eq!(event.severity, Severity::Error);
        assert_eq!(event.task.as_deref(), Some(":app:compileDebugKotlin"));
        assert_eq!(event.module.as_deref(), Some(":app"));
        assert_eq!(event.message, "Unresolved reference: missingApi");
        let location = event.location.as_ref().unwrap();
        assert_eq!(
            location.file,
            "/workspace/app/src/main/java/com/example/Main.kt"
        );
        assert_eq!((location.line, location.column), (42, Some(17)));
        assert_eq!(event.causes.len(), 2);
        assert_eq!(
            event.causes[1].message,
            "java.lang.IllegalStateException: compiler stopped"
        );
        assert!(!run
            .omissions
            .collapsed
            .contains_key("successful Gradle tasks"));
    }

    #[test]
    fn test_classify_required_android_gradle_families() {
        let cases = [
            (
                "e: /src/App.kt:1:2 Kotlin error",
                DiagnosticKind::KotlinCompiler,
            ),
            (
                "/src/App.java:9: error: cannot find symbol",
                DiagnosticKind::JavaCompiler,
            ),
            ("e: [ksp] No provider found", DiagnosticKind::Ksp),
            (
                "kapt: error: annotation processing failed",
                DiagnosticKind::Kapt,
            ),
            (
                "Compose Compiler: @Composable invocations failed",
                DiagnosticKind::ComposeCompiler,
            ),
            (
                "AAPT: error: resource color/missing not found",
                DiagnosticKind::Aapt2,
            ),
            (
                "Execution failed for task ':app:mergeDebugResources'. Duplicate resources",
                DiagnosticKind::ResourceMerge,
            ),
            (
                "Manifest merger failed with multiple errors",
                DiagnosticKind::ManifestMerger,
            ),
            (
                "Could not resolve com.example:missing:1.0",
                DiagnosticKind::DependencyResolution,
            ),
            (
                "Duplicate class com.example.Shared found in modules",
                DiagnosticKind::DuplicateClass,
            ),
            (
                "D8: Cannot fit requested classes in a single dex file",
                DiagnosticKind::D8,
            ),
            ("R8: Missing class com.example.Missing", DiagnosticKind::R8),
            (
                "src/main/AndroidManifest.xml:7: Error: lint problem [UnsafeOptInUsageError]",
                DiagnosticKind::Lint,
            ),
            (
                "com.example.WidgetTest > renders FAILED",
                DiagnosticKind::UnitTest,
            ),
            (
                "INSTRUMENTATION_FAILED: Process crashed",
                DiagnosticKind::InstrumentationTest,
            ),
        ];

        for (line, expected) in cases {
            assert_eq!(classify_line(line), Some(expected), "line: {line}");
        }
    }

    #[test]
    fn test_failed_unknown_gradle_output_is_low_confidence() {
        let raw = "custom plugin emitted an unfamiliar failure\n";

        let run = parse("./gradlew customTask", raw, 1, "run-id");

        assert_eq!(run.confidence, ParseConfidence::Low);
        assert!(run.events.is_empty());
    }

    #[test]
    fn test_successful_task_omissions_are_counted_from_actual_lines() {
        let raw = r#"> Task :app:preBuild UP-TO-DATE
> Task :app:compileDebugKotlin
> Task :app:assembleDebug
BUILD SUCCESSFUL in 1s
3 actionable tasks: 2 executed, 1 up-to-date
"#;

        let run = parse("./gradlew assembleDebug", raw, 0, "run-id");

        assert_eq!(run.confidence, ParseConfidence::High);
        assert_eq!(run.omissions.collapsed["successful Gradle tasks"], 3);
    }
}
