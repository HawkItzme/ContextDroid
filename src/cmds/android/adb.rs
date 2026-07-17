use crate::core::runner::{self, RunOptions};
use crate::diagnostics::{
    DiagnosticEvent, DiagnosticKind, DiagnosticRun, OmissionReport, ParseConfidence,
    ParserIdentity, Severity,
};
use anyhow::Result;
use std::collections::BTreeMap;
use std::ffi::OsString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdbCommand {
    Devices,
    Install,
    Uninstall,
    ShellAm,
    ShellPm,
    Dumpsys,
    Logcat,
    BinaryPassthrough,
    Unsupported,
}

pub fn classify(args: &[String]) -> AdbCommand {
    let Some(first) = args.first().map(String::as_str) else {
        return AdbCommand::Unsupported;
    };
    if matches!(
        first,
        "exec-out" | "pull" | "push" | "bugreport" | "backup" | "restore" | "sideload" | "sync"
    ) {
        return AdbCommand::BinaryPassthrough;
    }
    match first {
        "devices" => AdbCommand::Devices,
        "install" | "install-multiple" => AdbCommand::Install,
        "uninstall" => AdbCommand::Uninstall,
        "logcat" => AdbCommand::Logcat,
        "shell" => match args.get(1).map(String::as_str) {
            Some("am") => AdbCommand::ShellAm,
            Some("pm") => AdbCommand::ShellPm,
            Some("dumpsys")
                if matches!(
                    args.get(2).map(String::as_str),
                    Some("activity" | "package" | "meminfo")
                ) =>
            {
                AdbCommand::Dumpsys
            }
            _ => AdbCommand::Unsupported,
        },
        _ => AdbCommand::Unsupported,
    }
}

pub fn run(args: &[String], verbose: u8) -> Result<i32> {
    let kind = classify(args);
    if matches!(
        kind,
        AdbCommand::Unsupported | AdbCommand::BinaryPassthrough | AdbCommand::Logcat
    ) {
        let os_args: Vec<OsString> = args.iter().map(OsString::from).collect();
        return runner::run_passthrough("adb", &os_args, verbose);
    }
    let mut command = crate::core::utils::resolved_command("adb");
    command.args(args);
    let display = args.join(" ");
    let original = format!("adb {display}");
    runner::run_diagnostic(
        command,
        "adb",
        &display,
        move |raw, exit_code, run_id| Ok(parse(&original, raw, exit_code, run_id, kind)),
        RunOptions::default(),
    )
}

pub fn parse(
    command: &str,
    raw: &str,
    exit_code: i32,
    run_id: &str,
    kind: AdbCommand,
) -> DiagnosticRun {
    let mut events = Vec::new();
    if kind == AdbCommand::Devices {
        for (raw_line, line) in raw.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with("List of devices attached") {
                continue;
            }
            let mut fields = line.split_whitespace();
            let (Some(serial), Some(state)) = (fields.next(), fields.next()) else {
                continue;
            };
            let mut details = BTreeMap::new();
            details.insert("serial".into(), serial.into());
            details.insert("state".into(), state.into());
            for field in fields {
                if let Some((key, value)) = field.split_once(':') {
                    details.insert(key.into(), value.into());
                }
            }
            events.push(event(kind, Severity::Info, line, details, raw_line));
        }
    } else {
        for (raw_line, line) in raw.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            let is_error = lower.contains("failure [")
                || lower.starts_with("adb: failed")
                || lower.starts_with("error:")
                || lower.contains("exception occurred");
            let is_success = line.trim() == "Success"
                || line.starts_with("Starting:")
                || line.starts_with("Broadcast completed:")
                || line.starts_with("package:");
            if is_error || is_success {
                events.push(event(
                    DiagnosticKind::Adb,
                    if is_error {
                        Severity::Error
                    } else {
                        Severity::Info
                    },
                    line,
                    BTreeMap::new(),
                    raw_line,
                ));
            }
        }
    }

    let recognized_empty_devices = kind == AdbCommand::Devices
        && exit_code == 0
        && raw
            .lines()
            .any(|line| line.starts_with("List of devices attached"));
    let confidence = if !events.is_empty() || recognized_empty_devices {
        ParseConfidence::High
    } else {
        ParseConfidence::Low
    };
    let mut omissions = OmissionReport::default();
    if !events.is_empty() {
        omissions
            .preserved
            .insert("ADB results".into(), events.len());
    }
    DiagnosticRun {
        run_id: run_id.into(),
        command: command.into(),
        parser: ParserIdentity {
            name: "adb".into(),
            version: 1,
        },
        confidence,
        events,
        omissions,
    }
}

fn event(
    kind: impl Into<DiagnosticKind>,
    severity: Severity,
    message: &str,
    details: BTreeMap<String, String>,
    raw_line: usize,
) -> DiagnosticEvent {
    DiagnosticEvent {
        kind: kind.into(),
        severity,
        message: message.to_string(),
        error_type: None,
        task: None,
        module: None,
        variant: None,
        location: None,
        causes: Vec::new(),
        frames: Vec::new(),
        dependency_coordinates: Vec::new(),
        test_assertion: None,
        details,
        raw_line: Some(raw_line),
    }
}

impl From<AdbCommand> for DiagnosticKind {
    fn from(_: AdbCommand) -> Self {
        DiagnosticKind::Adb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{DiagnosticKind, ParseConfidence, Severity};

    #[test]
    fn test_classify_supported_and_binary_adb_commands() {
        assert_eq!(
            classify(&["devices".into(), "-l".into()]),
            AdbCommand::Devices
        );
        assert_eq!(
            classify(&["install".into(), "app.apk".into()]),
            AdbCommand::Install
        );
        assert_eq!(
            classify(&["shell".into(), "am".into(), "start".into()]),
            AdbCommand::ShellAm
        );
        for args in [
            vec!["exec-out".into(), "screencap".into(), "-p".into()],
            vec!["bugreport".into(), "report.zip".into()],
            vec!["pull".into(), "/sdcard/data".into()],
            vec!["push".into(), "payload".into(), "/sdcard/".into()],
        ] {
            assert_eq!(classify(&args), AdbCommand::BinaryPassthrough);
        }
        assert_eq!(
            classify(&["forward".into(), "tcp:1".into()]),
            AdbCommand::Unsupported
        );
    }

    #[test]
    fn test_parse_adb_devices_preserves_serial_state_and_details() {
        let raw = "List of devices attached\nemulator-5554\tdevice product:sdk_gphone64_x86_64 model:sdk_gphone64 device:emu64xa\nR5CT20ABC\toffline\n";

        let run = parse("adb devices -l", raw, 0, "run-id", AdbCommand::Devices);

        assert_eq!(run.confidence, ParseConfidence::High);
        assert_eq!(run.events.len(), 2);
        assert_eq!(run.events[0].kind, DiagnosticKind::Adb);
        assert_eq!(run.events[0].severity, Severity::Info);
        assert_eq!(run.events[0].details["serial"], "emulator-5554");
        assert_eq!(run.events[0].details["state"], "device");
        assert_eq!(run.events[1].details["state"], "offline");
    }

    #[test]
    fn test_parse_adb_install_failure_preserves_exact_failure() {
        let raw = "Performing Streamed Install\nadb: failed to install app.apk: Failure [INSTALL_FAILED_VERSION_DOWNGRADE: Downgrade detected]\n";

        let run = parse("adb install app.apk", raw, 1, "run-id", AdbCommand::Install);

        assert_eq!(run.confidence, ParseConfidence::High);
        assert_eq!(run.events.len(), 1);
        assert_eq!(run.events[0].severity, Severity::Error);
        assert_eq!(
            run.events[0].message,
            "adb: failed to install app.apk: Failure [INSTALL_FAILED_VERSION_DOWNGRADE: Downgrade detected]"
        );
    }

    #[test]
    fn test_failed_unrecognized_adb_output_is_low_confidence() {
        let run = parse(
            "adb shell am custom",
            "vendor-specific failure\n",
            1,
            "run-id",
            AdbCommand::ShellAm,
        );

        assert_eq!(run.confidence, ParseConfidence::Low);
    }
}
