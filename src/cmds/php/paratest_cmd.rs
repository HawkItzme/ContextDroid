//! ParaTest runner filter.

use super::test_output::filter_test_runner_output;
use super::utils::php_tool_command;
use crate::core::runner;
use anyhow::Result;

pub fn run(args: &[String], verbose: u8) -> Result<i32> {
    let mut cmd = php_tool_command("paratest");

    let has_no_progress = args.iter().any(|a| a == "--no-progress");
    if !has_no_progress {
        cmd.arg("--no-progress");
    }

    for arg in args {
        cmd.arg(arg);
    }

    if verbose > 0 {
        eprintln!("Running: paratest {}", args.join(" "));
    }

    runner::run_filtered(
        cmd,
        "paratest",
        &args.join(" "),
        filter_test_runner_output,
        runner::RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paratest_strips_banner_seed_and_progress() {
        // ParaTest prints its own banner and a "Random Seed:" line on top of
        // PHPUnit-style dot progress; only the result summary should survive.
        let output = "ParaTest v7.3.0 upon PHPUnit 10.5.0 by Sebastian Bergmann and contributors.\n\
                      Random Seed:   1234567890\n\
                      ..........                                        10 / 10 (100%)\n\n\
                      OK (10 tests, 25 assertions)\n";
        let filtered = filter_test_runner_output(output);
        assert!(!filtered.contains("ParaTest v7.3.0"), "got: {}", filtered);
        assert!(!filtered.contains("Random Seed:"), "got: {}", filtered);
        assert!(!filtered.contains("10 / 10 (100%)"), "got: {}", filtered);
        assert!(
            filtered.contains("OK (10 tests, 25 assertions)"),
            "got: {}",
            filtered
        );
    }

    #[test]
    fn test_paratest_keeps_failures() {
        let output = "ParaTest v7.3.0 upon PHPUnit 10.5.0\n\
                      ..F.\n\
                      There was 1 failure:\n\
                      1) App\\Tests\\UserTest::testEmail\n\
                      Failed asserting that false is true.\n";
        let filtered = filter_test_runner_output(output);
        assert!(!filtered.contains("ParaTest v7.3.0"), "got: {}", filtered);
        assert!(
            filtered.contains("App\\Tests\\UserTest::testEmail"),
            "got: {}",
            filtered
        );
        assert!(
            filtered.contains("Failed asserting that false is true."),
            "got: {}",
            filtered
        );
    }
}
