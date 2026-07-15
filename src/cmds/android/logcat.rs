use crate::core::runner::{self, RunOptions};
use crate::diagnostics::{
    Cause, ClassifiedFrame, DiagnosticEvent, DiagnosticKind, DiagnosticRun, FrameOwnership,
    OmissionReport, ParseConfidence, ParserIdentity, Severity,
};
use anyhow::Result;
use clap::ValueEnum;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LogcatMode {
    All,
    Crash,
    Anr,
    Strictmode,
    Binder,
    Native,
    Raw,
}

struct ParsedLine<'a> {
    timestamp: &'a str,
    pid: &'a str,
    tid: &'a str,
    priority: &'a str,
    tag: &'a str,
    message: &'a str,
}

lazy_static! {
    static ref THREADTIME: Regex = Regex::new(
        r"^(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d+)\s+(?:\S+\s+)?(\d+)\s+(\d+)\s+([VDIWEAF])\s+([^:]+):\s?(.*)$"
    )
    .unwrap();
    static ref PROCESS: Regex = Regex::new(r"^Process:\s*([^,]+),\s*PID:\s*(\d+)").unwrap();
}

pub fn build_adb_args(pid: Option<u32>, since: Option<&str>, raw_args: &[String]) -> Vec<String> {
    let mut args = vec!["logcat".to_string()];
    if let Some(pid) = pid {
        args.push(format!("--pid={pid}"));
    }
    if let Some(since) = since {
        args.push("-T".into());
        args.push(since.into());
    }
    args.extend(raw_args.iter().cloned());
    args
}

pub fn run(
    mode: LogcatMode,
    package: Option<&str>,
    pid: Option<u32>,
    since: Option<&str>,
    raw_args: &[String],
    verbose: u8,
) -> Result<i32> {
    let args = build_adb_args(pid, since, raw_args);
    let os_args: Vec<OsString> = args.iter().map(OsString::from).collect();
    if mode == LogcatMode::Raw {
        return runner::run_passthrough("adb", &os_args, verbose);
    }
    let mut command = crate::core::utils::resolved_command("adb");
    command.args(&args);
    let display = args.join(" ");
    let package = package.map(str::to_string);
    runner::run_diagnostic(
        command,
        "adb",
        &display,
        move |raw, _exit_code, run_id| parse(raw, run_id, mode, package.as_deref(), pid),
        RunOptions::default(),
    )
}

pub fn parse(
    raw: &str,
    run_id: &str,
    mode: LogcatMode,
    package: Option<&str>,
    pid: Option<u32>,
) -> DiagnosticRun {
    let lines: Vec<&str> = raw.lines().collect();
    let mut marker = None;
    for (index, line) in lines.iter().enumerate() {
        let parsed = parse_line(line);
        let message = parsed.as_ref().map_or(*line, |line| line.message);
        if let Some(kind) = classify_message(message) {
            if mode_matches(mode, &kind) {
                marker = Some((index, kind, parsed));
                break;
            }
        }
    }

    let mut events = Vec::new();
    let mut used = BTreeSet::new();
    if let Some((marker_index, kind, parsed_marker)) = marker {
        let mut details: BTreeMap<String, String> = BTreeMap::new();
        if let Some(line) = &parsed_marker {
            details.insert("timestamp".into(), line.timestamp.into());
            details.insert("pid".into(), line.pid.into());
            details.insert("tid".into(), line.tid.into());
            details.insert("priority".into(), line.priority.into());
            details.insert("tag".into(), line.tag.into());
        }
        let mut causes = Vec::new();
        let mut frames = Vec::new();
        let marker_message = parsed_marker
            .as_ref()
            .map_or(lines[marker_index], |line| line.message)
            .to_string();
        details.insert("incident".into(), marker_message.clone());
        let mut root_message = marker_message;
        let mut error_type = None;
        used.insert(marker_index);
        for (index, line) in lines.iter().enumerate().skip(marker_index) {
            let parsed = parse_line(line);
            let message = parsed.as_ref().map_or(*line, |line| line.message).trim();
            if let Some(thread) = message.strip_prefix("FATAL EXCEPTION: ") {
                details.insert("thread".into(), thread.into());
                used.insert(index);
            }
            if let Some(captures) = PROCESS.captures(message) {
                details.insert("process".into(), captures[1].trim().into());
                details.insert("package".into(), captures[1].trim().into());
                details.insert("pid".into(), captures[2].into());
                used.insert(index);
            }
            if let Some(process) = message.strip_prefix("ANR in ") {
                details.insert("process".into(), process.trim().into());
                details.insert("package".into(), process.trim().into());
                used.insert(index);
            }
            if let Some(reason) = message.strip_prefix("Reason: ") {
                details.insert("anr_reason".into(), reason.into());
                used.insert(index);
            }
            if let Some(cause) = message.strip_prefix("Caused by: ") {
                causes.push(Cause {
                    error_type: cause.split_once(':').map(|(kind, _)| kind.into()),
                    message: cause.into(),
                });
                used.insert(index);
            }
            if !message.starts_with("Caused by: ") {
                if let Some((kind, _)) = message.split_once(": ") {
                    if (kind.ends_with("Exception") || kind.ends_with("Error"))
                        && !kind.contains(' ')
                    {
                        root_message = message.to_string();
                        error_type = Some(kind.to_string());
                        used.insert(index);
                    }
                }
            }
            if message.starts_with("at ") || message.starts_with('#') {
                frames.push(ClassifiedFrame {
                    text: message.into(),
                    ownership: FrameOwnership::Unknown,
                    location: None,
                });
                used.insert(index);
            }
        }

        let package_matches = package.is_none_or(|expected| {
            details
                .get("package")
                .or_else(|| details.get("process"))
                .is_some_and(|actual| actual == expected)
                || raw.contains(expected)
        });
        let pid_matches = pid.is_none_or(|expected| {
            details
                .get("pid")
                .and_then(|actual| actual.parse::<u32>().ok())
                == Some(expected)
        });
        if package_matches && pid_matches {
            events.push(DiagnosticEvent {
                kind,
                severity: Severity::Error,
                message: root_message,
                error_type,
                task: None,
                module: None,
                variant: None,
                location: None,
                causes,
                frames,
                details,
                raw_line: Some(marker_index),
            });
        }
    }

    let confidence = if events.is_empty() {
        ParseConfidence::Low
    } else if events[0].details.contains_key("timestamp")
        && events[0].details.contains_key("pid")
        && events[0].details.contains_key("tid")
    {
        ParseConfidence::High
    } else {
        ParseConfidence::Medium
    };
    let mut omissions = OmissionReport::default();
    if !events.is_empty() {
        omissions
            .preserved
            .insert("Logcat incidents".into(), events.len());
        let unrelated = lines.len().saturating_sub(used.len());
        if unrelated > 0 {
            omissions
                .collapsed
                .insert("unrelated Logcat lines".into(), unrelated);
        }
    }
    DiagnosticRun {
        run_id: run_id.into(),
        command: "adb logcat".into(),
        parser: ParserIdentity {
            name: "logcat".into(),
            version: 1,
        },
        confidence,
        events,
        omissions,
    }
}

pub fn classify_message(message: &str) -> Option<DiagnosticKind> {
    let lower = message.to_ascii_lowercase();
    if lower.contains("fatal signal") || lower.contains("tombstone") || lower.starts_with("*** ***")
    {
        Some(DiagnosticKind::NativeCrash)
    } else if lower.contains("anr in ") || lower.starts_with("anr in ") {
        Some(DiagnosticKind::Anr)
    } else if lower.contains("strictmode") || lower.contains("strict mode") {
        Some(DiagnosticKind::StrictMode)
    } else if lower.contains("deadobjectexception")
        || lower.contains("transactiontoolargeexception")
        || lower.contains("binder died")
    {
        Some(DiagnosticKind::Binder)
    } else if lower.contains("fatal exception:") {
        Some(DiagnosticKind::LogcatCrash)
    } else {
        None
    }
}

fn mode_matches(mode: LogcatMode, kind: &DiagnosticKind) -> bool {
    mode == LogcatMode::All
        || matches!(
            (mode, kind),
            (LogcatMode::Crash, DiagnosticKind::LogcatCrash)
                | (LogcatMode::Anr, DiagnosticKind::Anr)
                | (LogcatMode::Strictmode, DiagnosticKind::StrictMode)
                | (LogcatMode::Binder, DiagnosticKind::Binder)
                | (LogcatMode::Native, DiagnosticKind::NativeCrash)
        )
}

fn parse_line(line: &str) -> Option<ParsedLine<'_>> {
    let captures = THREADTIME.captures(line)?;
    Some(ParsedLine {
        timestamp: captures.get(1)?.as_str(),
        pid: captures.get(2)?.as_str(),
        tid: captures.get(3)?.as_str(),
        priority: captures.get(4)?.as_str(),
        tag: captures.get(5)?.as_str().trim(),
        message: captures.get(6)?.as_str(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{DiagnosticKind, ParseConfidence};

    #[test]
    fn test_parse_java_crash_preserves_logcat_identity_causes_and_frames() {
        let raw = r#"07-15 10:11:12.123  1000  4242  4243 E AndroidRuntime: FATAL EXCEPTION: main
07-15 10:11:12.124  1000  4242  4243 E AndroidRuntime: Process: com.example.app, PID: 4242
07-15 10:11:12.125  1000  4242  4243 E AndroidRuntime: java.lang.IllegalStateException: boom
07-15 10:11:12.126  1000  4242  4243 E AndroidRuntime:     at com.example.app.MainActivity.onCreate(MainActivity.kt:42)
07-15 10:11:12.127  1000  4242  4243 E AndroidRuntime: Caused by: java.lang.NullPointerException: missing
07-15 10:11:12.128  1000  4242  4243 I ActivityManager: Process com.example.app has died
"#;

        let run = parse(
            raw,
            "run-id",
            LogcatMode::Crash,
            Some("com.example.app"),
            None,
        );

        assert_eq!(run.confidence, ParseConfidence::High);
        let crash = run.events.first().unwrap();
        assert_eq!(crash.kind, DiagnosticKind::LogcatCrash);
        assert_eq!(crash.details["timestamp"], "07-15 10:11:12.123");
        assert_eq!(crash.details["pid"], "4242");
        assert_eq!(crash.details["tid"], "4243");
        assert_eq!(crash.details["thread"], "main");
        assert_eq!(crash.details["process"], "com.example.app");
        assert_eq!(
            crash.error_type.as_deref(),
            Some("java.lang.IllegalStateException")
        );
        assert_eq!(crash.message, "java.lang.IllegalStateException: boom");
        assert_eq!(
            crash.causes[0].message,
            "java.lang.NullPointerException: missing"
        );
        assert!(crash
            .frames
            .iter()
            .any(|frame| frame.text.contains("MainActivity.onCreate")));
    }

    #[test]
    fn test_parse_anr_preserves_reason() {
        let raw = r#"07-15 11:00:00.000  1000  1111  2222 E ActivityManager: ANR in com.example.app
07-15 11:00:00.001  1000  1111  2222 E ActivityManager: Reason: Input dispatching timed out
"#;

        let run = parse(raw, "run-id", LogcatMode::Anr, None, None);

        assert_eq!(run.events[0].kind, DiagnosticKind::Anr);
        assert_eq!(
            run.events[0].details["anr_reason"],
            "Input dispatching timed out"
        );
    }

    #[test]
    fn test_classify_logcat_modes() {
        assert_eq!(
            classify_message("StrictMode policy violation: disk read"),
            Some(DiagnosticKind::StrictMode)
        );
        assert_eq!(
            classify_message("android.os.DeadObjectException: binder died"),
            Some(DiagnosticKind::Binder)
        );
        assert_eq!(
            classify_message("Fatal signal 11 (SIGSEGV), tombstone written"),
            Some(DiagnosticKind::NativeCrash)
        );
    }

    #[test]
    fn test_unrelated_logcat_output_falls_back_to_raw() {
        let raw = "07-15 10:00:00.000  1000  123  124 I Choreographer: Skipped 1 frames\n";

        let run = parse(raw, "run-id", LogcatMode::All, None, None);

        assert_eq!(run.confidence, ParseConfidence::Low);
        assert!(run.events.is_empty());
    }

    #[test]
    fn test_build_adb_args_preserves_pid_since_and_raw_args() {
        let args = build_adb_args(
            Some(4242),
            Some("07-15 10:00:00.000"),
            &["-d".into(), "-v".into(), "threadtime".into()],
        );

        assert_eq!(
            args,
            vec![
                "logcat",
                "--pid=4242",
                "-T",
                "07-15 10:00:00.000",
                "-d",
                "-v",
                "threadtime"
            ]
        );
    }
}
