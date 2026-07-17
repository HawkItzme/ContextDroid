use crate::core::run_store::{FinalizeDetails, ProcessOutcome, RunStart, RunStore};
use crate::diagnostics::{
    DiagnosticEvent, DiagnosticRun, NeverWorseDecision, OutputMode, ParseConfidence,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct Contract {
    file: String,
    parser: String,
    kind: Option<String>,
    source_contains: Vec<String>,
    exit_code: i32,
    #[serde(default)]
    stderr: String,
    #[serde(default = "balanced")]
    output_mode: OutputMode,
    expected: Expected,
}

fn balanced() -> OutputMode {
    OutputMode::Balanced
}

#[derive(Default, Deserialize)]
struct Expected {
    confidence: String,
    decision: String,
    root_message: Option<String>,
    error_type: Option<String>,
    task: Option<String>,
    module: Option<String>,
    variant: Option<String>,
    location: Option<ExpectedLocation>,
    #[serde(default)]
    causes: Vec<String>,
    application_frame: Option<String>,
    #[serde(default)]
    dependency_coordinates: Vec<String>,
    test_assertion: Option<ExpectedAssertion>,
    #[serde(default)]
    details: BTreeMap<String, String>,
    #[serde(default)]
    preserved: BTreeMap<String, usize>,
    #[serde(default)]
    collapsed: BTreeMap<String, usize>,
    #[serde(default)]
    rendered_contains: Vec<String>,
}

#[derive(Deserialize)]
struct ExpectedLocation {
    file: String,
    line: u32,
    column: Option<u32>,
}

#[derive(Deserialize)]
struct ExpectedAssertion {
    expected: String,
    actual: String,
}

#[test]
fn fixture_contracts_parse_render_store_and_recover_raw() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/android");
    let contracts: Vec<Contract> =
        serde_json::from_str(&fs::read_to_string(fixture_root.join("contract.json")).unwrap())
            .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::new(temp.path().canonicalize().unwrap().join("runs"));

    for contract in contracts {
        let raw = fs::read_to_string(fixture_root.join(&contract.file)).unwrap();
        for expected in &contract.source_contains {
            assert!(
                raw.contains(expected),
                "{} lacks {expected:?}",
                contract.file
            );
        }
        let mut active = store
            .start(RunStart {
                command: contract.parser.clone(),
                cwd: fixture_root.clone(),
                profile: "contextdroid-safe".into(),
                output_mode: format!("{:?}", contract.output_mode).to_ascii_lowercase(),
            })
            .unwrap();
        active.write_stdout(raw.as_bytes()).unwrap();
        active.write_stderr(contract.stderr.as_bytes()).unwrap();
        let run_id = active.id().as_str().to_string();
        let diagnostic = parse_contract(&contract, &raw, &run_id);

        assert_eq!(
            format!("{:?}", diagnostic.confidence).to_ascii_lowercase(),
            contract.expected.confidence,
            "{} confidence",
            contract.file
        );
        let event = contract
            .kind
            .as_ref()
            .map(|kind| find_event(&diagnostic, kind, &contract.file));
        if let Some(event) = event {
            assert_event(event, &contract);
        } else if contract.exit_code != 0 {
            assert!(
                diagnostic.events.is_empty(),
                "{} unexpectedly claimed a semantic failure",
                contract.file
            );
            assert_eq!(
                diagnostic.confidence,
                ParseConfidence::Low,
                "{}",
                contract.file
            );
        }

        for (label, expected) in &contract.expected.preserved {
            assert_eq!(
                diagnostic.omissions.preserved.get(label),
                Some(expected),
                "{} preserved omission count {label}",
                contract.file
            );
        }
        for (label, expected) in &contract.expected.collapsed {
            assert_eq!(
                diagnostic.omissions.collapsed.get(label),
                Some(expected),
                "{} collapsed omission count {label}",
                contract.file
            );
        }

        let rendered =
            crate::diagnostics::render_checked(&diagnostic, &raw, contract.output_mode, 5);
        assert_eq!(
            format!("{:?}", rendered.decision).to_ascii_lowercase(),
            contract.expected.decision,
            "{} never-worse decision",
            contract.file
        );
        if rendered.decision == NeverWorseDecision::Semantic {
            assert!(
                !contract.expected.rendered_contains.is_empty(),
                "{} semantic selection needs rendered evidence assertions",
                contract.file
            );
            for expected in &contract.expected.rendered_contains {
                assert!(
                    rendered.output.contains(expected),
                    "{} semantic output lost {expected:?}",
                    contract.file
                );
            }
        } else {
            assert_eq!(
                rendered.output.as_bytes(),
                raw.as_bytes(),
                "{} raw fallback must be byte-identical",
                contract.file
            );
        }

        let stored = active
            .finalize(
                ProcessOutcome::ExitCode(contract.exit_code),
                &serde_json::to_string(&diagnostic).unwrap(),
                &rendered.output,
                FinalizeDetails {
                    parser: Some(contract.parser.clone()),
                    confidence: Some(contract.expected.confidence.clone()),
                    raw_fallback: rendered.decision != NeverWorseDecision::Semantic,
                    never_worse_fallback: matches!(
                        rendered.decision,
                        NeverWorseDecision::RawIncompleteEvidence
                            | NeverWorseDecision::RawNotSmaller
                    ),
                    fixture_preservation: Some(true),
                    exit_code_parity: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(stored.metadata.exit_code, Some(contract.exit_code));
        assert_eq!(stored.metadata.signal, None);
        assert_eq!(stored.metadata.exit_code_parity, Some(true));
        assert_eq!(stored.metadata.run_id.as_str(), run_id);
        let parsed_id = crate::core::run_store::RunId::parse(&run_id).unwrap();
        let recovered = store.load(&parsed_id).unwrap();
        assert_eq!(recovered.metadata.run_id.as_str(), run_id);
        assert_eq!(recovered.read_stdout().unwrap(), raw.as_bytes());
        assert_eq!(
            recovered.read_stderr().unwrap(),
            contract.stderr.as_bytes(),
            "{} exact stderr recovery",
            contract.file
        );
    }

    assert_signal_parity(&store, &fixture_root);
}

#[test]
#[ignore = "manual reproducible benchmark report; run with --ignored --nocapture"]
fn emit_alpha_fixture_benchmark() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/android");
    let contracts: Vec<Contract> =
        serde_json::from_str(&fs::read_to_string(fixture_root.join("contract.json")).unwrap())
            .unwrap();
    let selected = [
        "gradle_success.txt",
        "kotlin_failure.txt",
        "resource_merge_failure.txt",
        "manifest_failure.txt",
        "unit_test_failure.txt",
        "java_crash.txt",
    ];
    for contract in contracts
        .iter()
        .filter(|contract| selected.contains(&contract.file.as_str()))
    {
        let raw = fs::read_to_string(fixture_root.join(&contract.file)).unwrap();
        let started = std::time::Instant::now();
        let mut last = None;
        for _ in 0..1_000 {
            let diagnostic = parse_contract(contract, &raw, "benchmark-run");
            last = Some(crate::diagnostics::render_checked(
                &diagnostic,
                &raw,
                contract.output_mode,
                5,
            ));
        }
        let elapsed = started.elapsed();
        let diagnostic = parse_contract(contract, &raw, "benchmark-run");
        let rendered = last.unwrap();
        println!(
            "{}",
            serde_json::json!({
                "fixture": contract.file,
                "raw_bytes": raw.len(),
                "returned_bytes": rendered.output.len(),
                "raw_tokens_estimate": raw.len().div_ceil(4),
                "returned_tokens_estimate": rendered.output.len().div_ceil(4),
                "confidence": format!("{:?}", diagnostic.confidence).to_ascii_lowercase(),
                "decision": format!("{:?}", rendered.decision).to_ascii_lowercase(),
                "average_parser_render_latency_us": elapsed.as_micros() / 1_000,
                "fixture_preservation": true,
                "raw_recovery_or_rerun": false
            })
        );
    }
}

fn find_event<'a>(run: &'a DiagnosticRun, kind: &str, file: &str) -> &'a DiagnosticEvent {
    run.events
        .iter()
        .find(|event| format!("{:?}", event.kind) == kind)
        .unwrap_or_else(|| panic!("{file} did not preserve {kind}"))
}

fn assert_event(event: &DiagnosticEvent, contract: &Contract) {
    let expected = &contract.expected;
    assert_eq!(
        Some(event.message.as_str()),
        expected.root_message.as_deref(),
        "{} root message",
        contract.file
    );
    assert_eq!(event.error_type.as_deref(), expected.error_type.as_deref());
    assert_eq!(event.task.as_deref(), expected.task.as_deref());
    assert_eq!(event.module.as_deref(), expected.module.as_deref());
    assert_eq!(event.variant.as_deref(), expected.variant.as_deref());
    if let Some(location) = &expected.location {
        let actual = event.location.as_ref().expect("required source location");
        assert_eq!(actual.file, location.file, "{} source file", contract.file);
        assert_eq!(actual.line, location.line, "{} source line", contract.file);
        assert_eq!(
            actual.column, location.column,
            "{} source column",
            contract.file
        );
    } else {
        assert!(
            event.location.is_none(),
            "{} unexpected location",
            contract.file
        );
    }
    assert_eq!(
        event
            .causes
            .iter()
            .map(|cause| cause.message.as_str())
            .collect::<Vec<_>>(),
        expected
            .causes
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "{} caused-by chain",
        contract.file
    );
    if let Some(frame) = &expected.application_frame {
        let actual = event
            .frames
            .iter()
            .find(|candidate| {
                candidate.ownership == crate::diagnostics::FrameOwnership::Application
            })
            .expect("required application-owned frame");
        assert_eq!(&actual.text, frame, "{} application frame", contract.file);
    }
    assert_eq!(
        event.dependency_coordinates, expected.dependency_coordinates,
        "{} dependency coordinates",
        contract.file
    );
    match (&event.test_assertion, &expected.test_assertion) {
        (Some(actual), Some(expected)) => {
            assert_eq!(actual.expected, expected.expected);
            assert_eq!(actual.actual, expected.actual);
        }
        (None, None) => {}
        _ => panic!("{} test assertion mismatch", contract.file),
    }
    for (key, value) in &expected.details {
        assert_eq!(
            event.details.get(key),
            Some(value),
            "{} detail {key}",
            contract.file
        );
    }
}

fn assert_signal_parity(store: &RunStore, fixture_root: &Path) {
    let active = store
        .start(RunStart {
            command: "synthetic signal fixture".into(),
            cwd: fixture_root.to_path_buf(),
            profile: "contextdroid-safe".into(),
            output_mode: "lossless".into(),
        })
        .unwrap();
    let stored = active
        .finalize(
            ProcessOutcome::Signal(9),
            "{}",
            "",
            FinalizeDetails {
                fixture_preservation: Some(true),
                exit_code_parity: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(stored.metadata.exit_code, None);
    assert_eq!(stored.metadata.signal, Some(9));
    assert_eq!(ProcessOutcome::Signal(9).shell_exit_code(), 137);
    assert_eq!(stored.metadata.exit_code_parity, Some(true));
}

fn parse_contract(contract: &Contract, raw: &str, run_id: &str) -> DiagnosticRun {
    use super::adb::AdbCommand;
    use super::logcat::LogcatMode;
    let stack_config = super::stack::AndroidStackConfig {
        application_ids: vec!["com.example.app".into()],
        source_prefixes: vec!["com.example".into()],
        generated_prefixes: Vec::new(),
    };
    match contract.parser.as_str() {
        "gradle" | "passthrough" => super::gradle::parse_with_config(
            "./gradlew task",
            raw,
            contract.exit_code,
            run_id,
            &stack_config,
        ),
        "adb-devices" => super::adb::parse(
            "adb devices",
            raw,
            contract.exit_code,
            run_id,
            AdbCommand::Devices,
        ),
        "adb-install" => super::adb::parse(
            "adb install",
            raw,
            contract.exit_code,
            run_id,
            AdbCommand::Install,
        ),
        "adb-uninstall" => super::adb::parse(
            "adb uninstall",
            raw,
            contract.exit_code,
            run_id,
            AdbCommand::Uninstall,
        ),
        "adb-am" => super::adb::parse(
            "adb shell am",
            raw,
            contract.exit_code,
            run_id,
            AdbCommand::ShellAm,
        ),
        "adb-pm" => super::adb::parse(
            "adb shell pm",
            raw,
            contract.exit_code,
            run_id,
            AdbCommand::ShellPm,
        ),
        parser if parser.starts_with("logcat-") => {
            let mode = match parser {
                "logcat-crash" => LogcatMode::Crash,
                "logcat-anr" => LogcatMode::Anr,
                "logcat-strictmode" => LogcatMode::Strictmode,
                "logcat-binder" => LogcatMode::Binder,
                "logcat-native" => LogcatMode::Native,
                _ => LogcatMode::All,
            };
            super::logcat::parse_with_config(raw, run_id, mode, None, None, &stack_config)
        }
        parser => panic!("unknown contract parser {parser}"),
    }
}
