use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Contract {
    file: String,
    parser: String,
    kind: Option<String>,
    contains: Vec<String>,
    exit_code: i32,
}

#[test]
fn fixture_manifest_is_complete_nonempty_and_synthetic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/android");
    let contracts: Vec<Contract> =
        serde_json::from_str(&fs::read_to_string(root.join("contract.json")).unwrap()).unwrap();
    assert_eq!(
        contracts.len(),
        30,
        "every required fixture family has a contract"
    );
    for contract in contracts {
        assert!(!contract.parser.is_empty());
        assert!(matches!(contract.exit_code, 0 | 1));
        if let Some(kind) = &contract.kind {
            assert!(!kind.is_empty());
        }
        let text = fs::read_to_string(root.join(&contract.file))
            .unwrap_or_else(|error| panic!("{}: {error}", contract.file));
        assert!(
            !text.is_empty(),
            "fixture must not be empty: {}",
            contract.file
        );
        for expected in contract.contains {
            assert!(
                text.contains(&expected),
                "{} lacks {expected:?}",
                contract.file
            );
        }
        assert!(!text.contains("token="));
        assert!(!text.contains("password="));
        assert!(!text.contains("BEGIN PRIVATE KEY"));
    }
}
