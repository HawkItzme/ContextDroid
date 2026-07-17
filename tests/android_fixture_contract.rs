use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Contract {
    file: String,
    parser: String,
    kind: Option<String>,
    source_contains: Vec<String>,
    exit_code: i32,
    expected: Expected,
}

#[derive(Debug, Deserialize)]
struct Expected {
    confidence: String,
    decision: String,
    root_message: Option<String>,
    #[serde(default)]
    causes: Vec<String>,
    location: Option<Location>,
    application_frame: Option<String>,
    #[serde(default)]
    dependency_coordinates: Vec<String>,
    test_assertion: Option<TestAssertion>,
    #[serde(default)]
    rendered_contains: Vec<String>,
    #[serde(default)]
    preserved: BTreeMap<String, usize>,
    #[serde(default)]
    collapsed: BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
struct Location {
    file: String,
    line: u32,
    column: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TestAssertion {
    expected: String,
    actual: String,
}

#[test]
fn fixture_manifest_is_complete_nonempty_synthetic_and_semantic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/android");
    let contracts: Vec<Contract> =
        serde_json::from_str(&fs::read_to_string(root.join("contract.json")).unwrap()).unwrap();
    assert_eq!(
        contracts.len(),
        30,
        "every required fixture family has a contract"
    );
    for contract in &contracts {
        assert!(!contract.parser.is_empty());
        assert!(matches!(contract.exit_code, 0 | 1));
        assert!(matches!(
            contract.expected.confidence.as_str(),
            "low" | "medium" | "high"
        ));
        assert!(matches!(
            contract.expected.decision.as_str(),
            "semantic"
                | "rawlossless"
                | "rawlowconfidence"
                | "rawincompleteevidence"
                | "rawnotsmaller"
        ));
        if contract.kind.is_some() {
            assert!(
                contract
                    .expected
                    .root_message
                    .as_deref()
                    .is_some_and(|message| !message.is_empty()),
                "{} needs an exact parsed root message",
                contract.file
            );
        }
        if contract.expected.decision == "semantic" {
            assert!(
                !contract.expected.rendered_contains.is_empty(),
                "{} needs rendered semantic assertions",
                contract.file
            );
        }
        let text = fs::read_to_string(root.join(&contract.file))
            .unwrap_or_else(|error| panic!("{}: {error}", contract.file));
        assert!(
            !text.is_empty(),
            "fixture must not be empty: {}",
            contract.file
        );
        for expected in &contract.source_contains {
            assert!(
                text.contains(expected),
                "{} lacks {expected:?}",
                contract.file
            );
        }
        assert!(!text.contains("token="));
        assert!(!text.contains("password="));
        assert!(!text.contains("BEGIN PRIVATE KEY"));
    }

    let kotlin = fixture(&contracts, "kotlin_failure.txt");
    let location = kotlin.expected.location.as_ref().unwrap();
    assert!(location.file.ends_with("Main.kt"));
    assert_eq!((location.line, location.column), (42, Some(17)));
    assert!(!kotlin.expected.causes.is_empty());
    assert!(!kotlin.expected.preserved.is_empty());
    assert!(!kotlin.expected.collapsed.is_empty());

    let crash = fixture(&contracts, "java_crash.txt");
    assert!(crash.expected.application_frame.is_some());

    let dependency = fixture(&contracts, "dependency_failure.txt");
    assert_eq!(
        dependency.expected.dependency_coordinates,
        ["com.example:missing-library:1.0.0"]
    );

    let unit_test = fixture(&contracts, "unit_test_failure.txt");
    let assertion = unit_test.expected.test_assertion.as_ref().unwrap();
    assert_eq!(
        (&assertion.expected, &assertion.actual),
        (&"ready".into(), &"loading".into())
    );

    for file in ["malformed_output.txt", "unknown_output.txt"] {
        let contract = fixture(&contracts, file);
        assert_eq!(contract.expected.confidence, "low");
        assert_eq!(contract.expected.decision, "rawlowconfidence");
        assert!(contract.kind.is_none());
    }
}

fn fixture<'a>(contracts: &'a [Contract], file: &str) -> &'a Contract {
    contracts
        .iter()
        .find(|contract| contract.file == file)
        .unwrap()
}
