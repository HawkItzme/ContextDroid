#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradleFamily {
    Build,
    Test,
    ConnectedTest,
    Lint,
    Dependencies,
    Mixed,
}

const OPTIONS_WITH_VALUE: &[&str] = &[
    "-p",
    "--project-dir",
    "-g",
    "--gradle-user-home",
    "-I",
    "--init-script",
    "-c",
    "--settings-file",
    "--include-build",
    "--max-workers",
    "--priority",
    "--console",
    "--warning-mode",
];

pub fn classify_gradle_args(args: &[String]) -> Option<GradleFamily> {
    let mut families = Vec::new();
    let mut skip_value = false;
    for arg in args {
        if skip_value {
            skip_value = false;
            continue;
        }
        if OPTIONS_WITH_VALUE.contains(&arg.as_str()) {
            skip_value = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        let task = arg
            .trim_matches(['\'', '"'])
            .rsplit(':')
            .next()
            .unwrap_or(arg)
            .to_ascii_lowercase();
        if task == "clean" {
            continue;
        }
        families.push(classify_task(&task)?);
    }
    let first = *families.first()?;
    if families.iter().all(|family| *family == first) {
        Some(first)
    } else {
        Some(GradleFamily::Mixed)
    }
}

fn classify_task(task: &str) -> Option<GradleFamily> {
    if task.starts_with("connected") || task.starts_with("manageddevice") {
        Some(GradleFamily::ConnectedTest)
    } else if task.starts_with("test") || task == "check" {
        Some(GradleFamily::Test)
    } else if ["assemble", "bundle", "build", "install", "uninstall"]
        .iter()
        .any(|prefix| task.starts_with(prefix))
    {
        Some(GradleFamily::Build)
    } else if task.contains("lint") {
        Some(GradleFamily::Lint)
    } else if task == "dependencies" || task == "dependencyinsight" {
        Some(GradleFamily::Dependencies)
    } else {
        None
    }
}

pub fn adb_is_safe_text(args: &[String]) -> bool {
    match args {
        [command, ..]
            if matches!(
                command.as_str(),
                "devices" | "install" | "install-multiple" | "uninstall"
            ) =>
        {
            true
        }
        [shell, am, action, ..]
            if shell == "shell"
                && am == "am"
                && matches!(
                    action.as_str(),
                    "start" | "startservice" | "broadcast" | "force-stop"
                ) =>
        {
            true
        }
        [shell, pm, action, tail @ ..]
            if shell == "shell"
                && pm == "pm"
                && (action == "path"
                    || action == "resolve-activity"
                    || (action == "list"
                        && tail.first().is_some_and(|value| value == "packages"))) =>
        {
            true
        }
        [shell, dumpsys, service, narrowing @ ..]
            if shell == "shell"
                && dumpsys == "dumpsys"
                && matches!(service.as_str(), "activity" | "package" | "meminfo")
                && narrowing.iter().any(|arg| !arg.starts_with('-')) =>
        {
            true
        }
        [command] if command == "logcat" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn gradle_classifier_handles_clean_modules_and_flag_values() {
        assert_eq!(
            classify_gradle_args(&args(&["clean", "assembleDebug"])),
            Some(GradleFamily::Build)
        );
        assert_eq!(
            classify_gradle_args(&args(&["clean", "testDebugUnitTest"])),
            Some(GradleFamily::Test)
        );
        assert_eq!(
            classify_gradle_args(&args(&[":app:assembleDebug", ":lib:bundleRelease"])),
            Some(GradleFamily::Build)
        );
        assert_eq!(
            classify_gradle_args(&args(&["--project-dir", "custom", "assembleDebug"])),
            Some(GradleFamily::Build)
        );
        assert_eq!(classify_gradle_args(&args(&["clean"])), None);
        assert_eq!(
            classify_gradle_args(&args(&["assembleDebug", "customTask"])),
            None
        );
    }

    #[test]
    fn adb_classifier_is_explicit_and_fail_closed() {
        assert!(adb_is_safe_text(&args(&[
            "shell",
            "am",
            "force-stop",
            "com.example"
        ])));
        assert!(adb_is_safe_text(&args(&[
            "shell", "pm", "list", "packages"
        ])));
        assert!(adb_is_safe_text(&args(&[
            "shell",
            "dumpsys",
            "meminfo",
            "com.example"
        ])));
        assert!(!adb_is_safe_text(&args(&["shell", "dumpsys", "meminfo"])));
        assert!(!adb_is_safe_text(&args(&["exec-out", "screencap", "-p"])));
        assert!(adb_is_safe_text(&args(&["logcat"])));
        assert!(!adb_is_safe_text(&args(&["logcat", "-d"])));
    }
}
