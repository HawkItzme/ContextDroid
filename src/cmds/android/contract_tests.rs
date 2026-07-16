use crate::core::run_store::{FinalizeDetails, ProcessOutcome, RunStart, RunStore};
use crate::diagnostics::{DiagnosticRun, OutputMode, ParseConfidence};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct Contract {
    file: String,
    parser: String,
    kind: Option<String>,
    contains: Vec<String>,
    exit_code: i32,
}

#[test]
fn fixture_contracts_parse_render_store_and_recover_raw() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/android");
    let contracts: Vec<Contract> =
        serde_json::from_str(&fs::read_to_string(fixture_root.join("contract.json")).unwrap())
            .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::new(temp.path().join("runs"));

    for contract in contracts {
        let raw = fs::read_to_string(fixture_root.join(&contract.file)).unwrap();
        let mut active = store
            .start(RunStart {
                command: contract.parser.clone(),
                cwd: fixture_root.clone(),
                profile: "contextdroid-safe".into(),
                output_mode: "balanced".into(),
            })
            .unwrap();
        active.write_stdout(raw.as_bytes()).unwrap();
        let run_id = active.id().as_str().to_string();
        let diagnostic = parse_contract(&contract, &raw, &run_id);
        if let Some(kind) = &contract.kind {
            assert!(
                diagnostic
                    .events
                    .iter()
                    .any(|event| format!("{:?}", event.kind) == *kind),
                "{} did not preserve {kind}",
                contract.file
            );
        } else if contract.exit_code != 0 {
            assert_eq!(
                diagnostic.confidence,
                ParseConfidence::Low,
                "{}",
                contract.file
            );
        }
        let rendered = crate::diagnostics::render(&diagnostic, &raw, OutputMode::Balanced, 5);
        for expected in &contract.contains {
            assert!(
                raw.contains(expected),
                "{} lost source evidence",
                contract.file
            );
        }
        let stored = active
            .finalize(
                ProcessOutcome::ExitCode(contract.exit_code),
                &serde_json::to_string(&diagnostic).unwrap(),
                &rendered,
                FinalizeDetails {
                    parser: Some(contract.parser.clone()),
                    confidence: format!("{:?}", diagnostic.confidence).to_ascii_lowercase(),
                    raw_fallback: diagnostic.confidence == ParseConfidence::Low,
                    fixture_preservation: true,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(stored.metadata.exit_code, Some(contract.exit_code));
        assert_eq!(
            store
                .load(&stored.metadata.run_id)
                .unwrap()
                .read_stdout()
                .unwrap(),
            raw.as_bytes()
        );
    }
}

fn parse_contract(contract: &Contract, raw: &str, run_id: &str) -> DiagnosticRun {
    use super::adb::AdbCommand;
    use super::logcat::LogcatMode;
    match contract.parser.as_str() {
        "gradle" | "passthrough" => {
            super::gradle::parse("./gradlew task", raw, contract.exit_code, run_id)
        }
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
            super::logcat::parse(raw, run_id, mode, None, None)
        }
        parser => panic!("unknown contract parser {parser}"),
    }
}
