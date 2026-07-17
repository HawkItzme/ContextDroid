use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMode {
    Lossless,
    Balanced,
    Aggressive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParseConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticKind {
    KotlinCompiler,
    JavaCompiler,
    Ksp,
    Kapt,
    ComposeCompiler,
    Aapt2,
    ResourceMerge,
    ManifestMerger,
    DependencyResolution,
    DuplicateClass,
    D8,
    R8,
    Lint,
    UnitTest,
    InstrumentationTest,
    Gradle,
    Adb,
    LogcatCrash,
    Anr,
    StrictMode,
    Binder,
    NativeCrash,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParserIdentity {
    pub name: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cause {
    pub error_type: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrameOwnership {
    Application,
    Generated,
    ThirdParty,
    AndroidFramework,
    KotlinCoroutine,
    GradlePlugin,
    Native,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifiedFrame {
    pub text: String,
    pub ownership: FrameOwnership,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestAssertion {
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub kind: DiagnosticKind,
    pub severity: Severity,
    pub message: String,
    pub error_type: Option<String>,
    pub task: Option<String>,
    pub module: Option<String>,
    pub variant: Option<String>,
    pub location: Option<SourceLocation>,
    pub causes: Vec<Cause>,
    pub frames: Vec<ClassifiedFrame>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_coordinates: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_assertion: Option<TestAssertion>,
    pub details: BTreeMap<String, String>,
    /// Zero-based source line used only to attach medium-confidence raw context.
    pub raw_line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFingerprint(pub String);

impl DiagnosticEvent {
    pub fn fingerprint(&self) -> DiagnosticFingerprint {
        DiagnosticFingerprint(format!(
            "{:?}|{:?}|{}|{}|{}",
            self.kind,
            self.severity,
            self.message,
            self.location
                .as_ref()
                .map(|location| location.file.as_str())
                .unwrap_or_default(),
            self.location
                .as_ref()
                .map(|location| location.line)
                .unwrap_or_default()
        ))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmissionReport {
    pub preserved: BTreeMap<String, usize>,
    pub collapsed: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticRun {
    pub run_id: String,
    pub command: String,
    pub parser: ParserIdentity,
    pub confidence: ParseConfidence,
    pub events: Vec<DiagnosticEvent>,
    pub omissions: OmissionReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NeverWorseDecision {
    Semantic,
    RawLossless,
    RawLowConfidence,
    RawIncompleteEvidence,
    RawNotSmaller,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedDiagnostic {
    pub output: String,
    pub decision: NeverWorseDecision,
}

pub fn assess_confidence(
    failed: bool,
    events: &[DiagnosticEvent],
    parser_error: bool,
) -> ParseConfidence {
    if parser_error || (failed && events.is_empty()) {
        return ParseConfidence::Low;
    }
    if failed
        && events.iter().any(|event| {
            event.severity == Severity::Error
                && (!event.message.trim().is_empty())
                && (event.location.is_some() || event.task.is_some() || !event.causes.is_empty())
        })
    {
        ParseConfidence::High
    } else if events.is_empty() {
        ParseConfidence::Low
    } else {
        ParseConfidence::Medium
    }
}

pub fn render(run: &DiagnosticRun, raw: &str, mode: OutputMode, context_lines: usize) -> String {
    if mode == OutputMode::Lossless || run.confidence == ParseConfidence::Low {
        return raw.to_string();
    }

    let mut output = String::new();
    for event in &run.events {
        let _ = writeln!(
            output,
            "[{:?}] {:?}: {}",
            event.severity, event.kind, event.message
        );
        if let Some(task) = &event.task {
            let _ = writeln!(output, "Task: {task}");
        }
        if let Some(module) = &event.module {
            let _ = writeln!(output, "Module: {module}");
        }
        if let Some(variant) = &event.variant {
            let _ = writeln!(output, "Variant: {variant}");
        }
        if let Some(error_type) = &event.error_type {
            let _ = writeln!(output, "Error type: {error_type}");
        }
        if let Some(location) = &event.location {
            let _ = write!(output, "Location: {}:{}", location.file, location.line);
            if let Some(column) = location.column {
                let _ = write!(output, ":{column}");
            }
            output.push('\n');
        }
        for cause in &event.causes {
            let _ = writeln!(output, "Caused by: {}", cause.message);
        }
        for coordinate in &event.dependency_coordinates {
            let _ = writeln!(output, "Dependency: {coordinate}");
        }
        if let Some(assertion) = &event.test_assertion {
            let _ = writeln!(output, "Expected: {}", assertion.expected);
            let _ = writeln!(output, "Actual: {}", assertion.actual);
        }
        for (key, value) in &event.details {
            let _ = writeln!(output, "{key}: {value}");
        }
        for frame in &event.frames {
            let _ = writeln!(output, "  {}", frame.text);
        }
    }

    if run.confidence == ParseConfidence::Medium {
        append_raw_context(&mut output, run, raw, context_lines);
    }
    append_omissions(&mut output, &run.omissions);
    if !run.events.is_empty() {
        let _ = writeln!(output, "Run: {}", run.run_id);
        let _ = writeln!(output, "Raw: contextdroid show {} --raw", run.run_id);
    }
    output
}

pub fn render_checked(
    run: &DiagnosticRun,
    raw: &str,
    mode: OutputMode,
    context_lines: usize,
) -> RenderedDiagnostic {
    if mode == OutputMode::Lossless {
        return RenderedDiagnostic {
            output: raw.to_string(),
            decision: NeverWorseDecision::RawLossless,
        };
    }
    if run.confidence == ParseConfidence::Low {
        return RenderedDiagnostic {
            output: raw.to_string(),
            decision: NeverWorseDecision::RawLowConfidence,
        };
    }
    if run.events.is_empty() {
        return RenderedDiagnostic {
            output: raw.to_string(),
            decision: NeverWorseDecision::RawIncompleteEvidence,
        };
    }
    if run
        .events
        .iter()
        .any(|event| event.severity == Severity::Error && event.message.trim().is_empty())
    {
        return RenderedDiagnostic {
            output: raw.to_string(),
            decision: NeverWorseDecision::RawIncompleteEvidence,
        };
    }
    let semantic = render(run, raw, mode, context_lines);
    if semantic.len() >= raw.len() {
        return RenderedDiagnostic {
            output: raw.to_string(),
            decision: NeverWorseDecision::RawNotSmaller,
        };
    }
    RenderedDiagnostic {
        output: semantic,
        decision: NeverWorseDecision::Semantic,
    }
}

fn append_raw_context(output: &mut String, run: &DiagnosticRun, raw: &str, context_lines: usize) {
    let lines: Vec<&str> = raw.lines().collect();
    let Some(anchor) = run.events.iter().filter_map(|event| event.raw_line).min() else {
        return;
    };
    let end_anchor = run
        .events
        .iter()
        .filter_map(|event| event.raw_line)
        .max()
        .unwrap_or(anchor);
    let start = anchor.saturating_sub(context_lines);
    let end = end_anchor
        .saturating_add(context_lines)
        .saturating_add(1)
        .min(lines.len());
    output.push_str("Raw context:\n");
    for line in &lines[start..end] {
        let _ = writeln!(output, "{line}");
    }
}

fn append_omissions(output: &mut String, omissions: &OmissionReport) {
    if !omissions.preserved.is_empty() {
        output.push_str("Preserved:\n");
        for (label, count) in &omissions.preserved {
            let _ = writeln!(output, "- {count} {label}");
        }
    }
    if !omissions.collapsed.is_empty() {
        output.push_str("Collapsed:\n");
        for (label, count) in &omissions.collapsed {
            let _ = writeln!(output, "- {count} {label}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed_run(confidence: ParseConfidence) -> DiagnosticRun {
        DiagnosticRun {
            run_id: "20260715T010203.004Z-a1b2c3d4".into(),
            command: "./gradlew assembleDebug".into(),
            parser: ParserIdentity {
                name: "gradle".into(),
                version: 1,
            },
            confidence,
            events: vec![DiagnosticEvent {
                kind: DiagnosticKind::KotlinCompiler,
                severity: Severity::Error,
                message: "Unresolved reference: missingApi".into(),
                error_type: None,
                task: Some(":app:compileDebugKotlin".into()),
                module: Some(":app".into()),
                variant: Some("debug".into()),
                location: Some(SourceLocation {
                    file: "app/src/main/java/com/example/Main.kt".into(),
                    line: 42,
                    column: Some(17),
                }),
                causes: Vec::new(),
                frames: Vec::new(),
                dependency_coordinates: Vec::new(),
                test_assertion: None,
                details: Default::default(),
                raw_line: Some(2),
            }],
            omissions: OmissionReport::default(),
        }
    }

    #[test]
    fn test_failed_command_without_diagnostics_is_low_confidence() {
        let assessment = assess_confidence(true, &[], false);

        assert_eq!(assessment, ParseConfidence::Low);
    }

    #[test]
    fn test_low_confidence_renderer_returns_raw_unchanged() {
        let raw = "FAILURE: Build failed\nunknown plugin output\n";
        let mut run = failed_run(ParseConfidence::Low);
        run.events.clear();

        let rendered = render(&run, raw, OutputMode::Balanced, 5);

        assert_eq!(rendered, raw);
    }

    #[test]
    fn test_medium_confidence_includes_five_lines_of_raw_context() {
        let raw = (0..12)
            .map(|line| format!("line-{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let run = failed_run(ParseConfidence::Medium);

        let rendered = render(&run, &raw, OutputMode::Balanced, 5);

        assert!(rendered.contains("line-0"));
        assert!(rendered.contains("line-7"));
        assert!(!rendered.contains("line-8"));
    }

    #[test]
    fn test_compact_renderer_reports_actual_omissions_and_recovery() {
        let mut run = failed_run(ParseConfidence::High);
        run.omissions.preserved.insert("root errors".into(), 1);
        run.omissions
            .collapsed
            .insert("successful Gradle tasks".into(), 38);

        let rendered = render(&run, "raw", OutputMode::Balanced, 5);

        assert!(rendered.contains("Preserved:\n- 1 root errors"));
        assert!(rendered.contains("Collapsed:\n- 38 successful Gradle tasks"));
        assert!(rendered.contains("Run: 20260715T010203.004Z-a1b2c3d4"));
        assert!(rendered.contains("contextdroid show 20260715T010203.004Z-a1b2c3d4 --raw"));
    }

    #[test]
    fn test_aggressive_never_overrides_low_confidence_fallback() {
        let run = failed_run(ParseConfidence::Low);

        assert_eq!(
            render(&run, "raw evidence", OutputMode::Aggressive, 5),
            "raw evidence"
        );
    }

    #[test]
    fn test_checked_render_replays_raw_when_semantic_is_not_smaller() {
        let raw = "failed\n";
        let rendered = render_checked(
            &failed_run(ParseConfidence::High),
            raw,
            OutputMode::Balanced,
            5,
        );
        assert_eq!(rendered.output, raw);
        assert_eq!(rendered.decision, NeverWorseDecision::RawNotSmaller);
    }

    #[test]
    fn test_checked_render_uses_semantic_only_when_evidence_is_complete_and_smaller() {
        let raw = format!("{}\n", "verbose successful task chatter".repeat(200));
        let rendered = render_checked(
            &failed_run(ParseConfidence::High),
            &raw,
            OutputMode::Balanced,
            5,
        );
        assert_eq!(rendered.decision, NeverWorseDecision::Semantic);
        assert!(rendered.output.len() < raw.len());
    }
}
