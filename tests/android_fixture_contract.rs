use std::fs;
use std::path::Path;

#[test]
fn required_android_fixture_groups_exist_and_are_nonempty() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/android");
    let required = [
        "gradle_success.txt",
        "kotlin_failure.txt",
        "java_failure.txt",
        "ksp_failure.txt",
        "kapt_failure.txt",
        "compose_failure.txt",
        "aapt2_failure.txt",
        "resource_merge_failure.txt",
        "manifest_failure.txt",
        "dependency_failure.txt",
        "duplicate_class_failure.txt",
        "d8_failure.txt",
        "r8_failure.txt",
        "lint_failure.txt",
        "unit_test_failure.txt",
        "instrumentation_failure.txt",
        "adb_devices.txt",
        "adb_install_failure.txt",
        "adb_uninstall_success.txt",
        "adb_shell_am.txt",
        "adb_shell_pm.txt",
        "java_crash.txt",
        "kotlin_coroutine_crash.txt",
        "anr.txt",
        "strictmode.txt",
        "binder_death.txt",
        "native_crash.txt",
        "malformed_output.txt",
        "unknown_output.txt",
        "verbose_passthrough.txt",
    ];
    for name in required {
        let bytes = fs::read(root.join(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!bytes.is_empty(), "fixture must not be empty: {name}");
    }
}

#[test]
fn fixture_corpus_uses_only_synthetic_example_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/android");
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) != Some("txt") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("token="));
        assert!(!text.contains("password="));
        assert!(!text.contains("BEGIN PRIVATE KEY"));
    }
}
