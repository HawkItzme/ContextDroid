//! Shared command execution skeleton for filter modules.

use anyhow::{Context, Result};
use std::process::Command;

use crate::core::run_store::{ActiveRun, FinalizeDetails, ProcessOutcome, RunStart, RunStore};
use crate::core::stream::{self, FilterMode, StdinMode, StreamFilter};
use crate::core::tracking;

/// Compose `filtered` with an optional recovery `hint`, cap the total at `raw`
/// (never emit more tokens than the command), print it, and return what was
/// emitted so the caller tracks exactly that.
pub fn emit_guarded(filtered: &str, hint: Option<&str>, raw: &str) -> String {
    let body = match hint {
        Some(h) => format!("{}\n{}", filtered, h),
        None => filtered.to_string(),
    };
    let shown = crate::core::guard::never_worse(raw, &body).to_string();
    println!("{}", shown);
    shown
}

pub fn print_with_hint(
    filtered: &str,
    tee_raw: &str,
    guard_raw: &str,
    tee_label: &str,
    exit_code: i32,
) -> String {
    let hint = crate::core::tee::tee_and_hint(tee_raw, tee_label, exit_code);
    emit_guarded(filtered, hint.as_deref(), guard_raw)
}

#[derive(Default)]
pub struct RunOptions<'a> {
    pub tee_label: Option<&'a str>,
    pub filter_stdout_only: bool,
    pub skip_filter_on_failure: bool,
    pub no_trailing_newline: bool,
    /// Forward rtk's own stdin to the child process. Needed for commands that
    /// can read from a pipe (e.g. `cat file | rtk wc`); without it the child
    /// gets an empty stdin and reports zero.
    pub inherit_stdin: bool,
}

impl<'a> RunOptions<'a> {
    pub fn with_tee(label: &'a str) -> Self {
        Self {
            tee_label: Some(label),
            ..Default::default()
        }
    }

    pub fn stdout_only() -> Self {
        Self {
            filter_stdout_only: true,
            ..Default::default()
        }
    }

    pub fn tee(mut self, label: &'a str) -> Self {
        self.tee_label = Some(label);
        self
    }

    pub fn early_exit_on_failure(mut self) -> Self {
        self.skip_filter_on_failure = true;
        self
    }

    pub fn no_trailing_newline(mut self) -> Self {
        self.no_trailing_newline = true;
        self
    }

    pub fn inherit_stdin(mut self) -> Self {
        self.inherit_stdin = true;
        self
    }
}

pub type CaptureFilter<'a> = Box<dyn Fn(&str) -> String + 'a>;
pub type ExitAwareCaptureFilter<'a> = Box<dyn Fn(&str, i32) -> String + 'a>;

pub enum RunMode<'a> {
    Filtered(CaptureFilter<'a>),
    FilteredWithExit(ExitAwareCaptureFilter<'a>),
    Streamed(Box<dyn StreamFilter + 'a>),
    Passthrough,
}

fn start_durable_run(cmd_label: &str) -> Option<ActiveRun> {
    let store = match RunStore::default_store() {
        Ok(store) => store,
        Err(error) => {
            eprintln!("[contextdroid] raw recovery unavailable: {error:#}");
            return None;
        }
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("[contextdroid] raw recovery unavailable: {error}");
            return None;
        }
    };
    match store.start(RunStart {
        command: cmd_label.to_string(),
        cwd,
        profile: crate::product::DEFAULT_PROFILE.to_string(),
        output_mode: "balanced".to_string(),
    }) {
        Ok(run) => Some(run),
        Err(error) => {
            eprintln!("[contextdroid] raw recovery unavailable: {error:#}");
            None
        }
    }
}

fn capture_durably(
    cmd: &mut Command,
    cmd_label: &str,
    inherit_stdin: bool,
) -> Result<Option<(ActiveRun, ProcessOutcome, String, String)>> {
    let Some(mut run) = start_durable_run(cmd_label) else {
        return Ok(None);
    };
    let outcome = run.capture_command(cmd, inherit_stdin)?;
    let stdout = String::from_utf8_lossy(&run.read_stdout()?).into_owned();
    let stderr = String::from_utf8_lossy(&run.read_stderr()?).into_owned();
    Ok(Some((run, outcome, stdout, stderr)))
}

fn recovery_hint(run: Option<&ActiveRun>) -> Option<String> {
    run.map(|run| {
        format!(
            "Run: {}\nRaw: contextdroid show {} --raw",
            run.id().as_str(),
            run.id().as_str()
        )
    })
}

fn finalize_durable(
    run: Option<ActiveRun>,
    outcome: ProcessOutcome,
    summary: &str,
    raw_fallback: bool,
) {
    finalize_durable_artifacts(
        run,
        outcome,
        summary,
        "{\"schema_version\":1,\"events\":[]}",
        FinalizeDetails {
            parser: Some("legacy".to_string()),
            confidence: "unvalidated".to_string(),
            raw_fallback,
            ..FinalizeDetails::default()
        },
    );
}

fn finalize_durable_artifacts(
    run: Option<ActiveRun>,
    outcome: ProcessOutcome,
    summary: &str,
    diagnostics_json: &str,
    details: FinalizeDetails,
) {
    let Some(run) = run else {
        return;
    };
    match run.finalize(outcome, diagnostics_json, summary, details) {
        Ok(stored) => crate::core::run_analytics::record_silent(&stored.metadata),
        Err(error) => eprintln!("[contextdroid] failed to finalize raw recovery: {error:#}"),
    }
    if let Ok(store) = RunStore::default_store() {
        if let Err(error) = store.prune(Default::default()) {
            eprintln!("[contextdroid] failed to prune raw runs: {error:#}");
        }
    }
}

pub fn run_diagnostic<F>(
    mut cmd: Command,
    tool_name: &str,
    args_display: &str,
    parser: F,
    opts: RunOptions<'_>,
) -> Result<i32>
where
    F: Fn(&str, i32, &str) -> crate::diagnostics::DiagnosticRun,
{
    let timer = tracking::TimedExecution::start();
    let cmd_label = format!("{} {}", tool_name, args_display);
    let Some((run, outcome, raw_stdout, raw_stderr)) =
        capture_durably(&mut cmd, &cmd_label, opts.inherit_stdin)
            .with_context(|| format!("Failed to run {}", tool_name))?
    else {
        let stdin_mode = if opts.inherit_stdin {
            StdinMode::Inherit
        } else {
            StdinMode::Null
        };
        let result = stream::run_streaming(&mut cmd, stdin_mode, FilterMode::CaptureOnly)
            .with_context(|| format!("Failed to run {}", tool_name))?;
        print!("{}", result.raw_stdout);
        eprint!("{}", result.raw_stderr);
        return Ok(result.exit_code);
    };

    let exit_code = outcome.shell_exit_code();
    let raw = format!("{}{}", raw_stdout, raw_stderr);
    let diagnostic = parser(&raw, exit_code, run.id().as_str());
    let confidence = match diagnostic.confidence {
        crate::diagnostics::ParseConfidence::High => "high",
        crate::diagnostics::ParseConfidence::Medium => "medium",
        crate::diagnostics::ParseConfidence::Low => "low",
    };
    let raw_fallback = diagnostic.confidence == crate::diagnostics::ParseConfidence::Low;
    let parser_name = diagnostic.parser.name.clone();
    let mut diagnostics = serde_json::to_value(&diagnostic)?;
    if let Some(object) = diagnostics.as_object_mut() {
        object.insert("schema_version".into(), serde_json::Value::from(1));
    }
    let diagnostics_json = serde_json::to_string_pretty(&diagnostics)?;
    let omission_preserved = diagnostic.omissions.preserved.values().sum::<usize>() as u64;
    let omission_collapsed = diagnostic.omissions.collapsed.values().sum::<usize>() as u64;
    let shown = crate::diagnostics::render(
        &diagnostic,
        &raw,
        crate::diagnostics::OutputMode::Balanced,
        5,
    );
    finalize_durable_artifacts(
        Some(run),
        outcome,
        &shown,
        &diagnostics_json,
        FinalizeDetails {
            parser: Some(parser_name),
            confidence: confidence.to_string(),
            raw_fallback,
            parser_error: false,
            omission_preserved,
            omission_collapsed,
            fixture_preservation: true,
        },
    );

    if raw_fallback {
        print!("{}", raw_stdout);
        eprint!("{}", raw_stderr);
    } else {
        print!("{}", shown);
    }
    timer.track(
        &cmd_label,
        &format!("contextdroid {}", cmd_label),
        &raw,
        &shown,
    );
    Ok(exit_code)
}

fn run_captured_filter<F>(
    mut cmd: Command,
    tool_name: &str,
    cmd_label: &str,
    filter_fn: F,
    opts: RunOptions<'_>,
    timer: tracking::TimedExecution,
) -> Result<i32>
where
    F: Fn(&str, i32) -> String,
{
    let durable = capture_durably(&mut cmd, cmd_label, opts.inherit_stdin)
        .with_context(|| format!("Failed to run {}", tool_name))?;
    let (durable_run, outcome, raw_stdout, raw_stderr) = match durable {
        Some((run, outcome, stdout, stderr)) => (Some(run), outcome, stdout, stderr),
        None => {
            let stdin_mode = if opts.inherit_stdin {
                StdinMode::Inherit
            } else {
                StdinMode::Null
            };
            let result = stream::run_streaming(&mut cmd, stdin_mode, FilterMode::CaptureOnly)
                .with_context(|| format!("Failed to run {}", tool_name))?;
            (
                None,
                ProcessOutcome::ExitCode(result.exit_code),
                result.raw_stdout,
                result.raw_stderr,
            )
        }
    };
    let exit_code = outcome.shell_exit_code();
    let raw = format!("{}{}", raw_stdout, raw_stderr);

    if opts.skip_filter_on_failure && exit_code != 0 {
        if !raw_stdout.trim().is_empty() {
            print!("{}", raw_stdout);
        }
        if !raw_stderr.trim().is_empty() {
            eprint!("{}", raw_stderr);
        }
        finalize_durable(durable_run, outcome, &raw, true);
        timer.track(
            cmd_label,
            &format!("contextdroid {}", cmd_label),
            &raw,
            &raw,
        );
        return Ok(exit_code);
    }

    let text_to_filter = if opts.filter_stdout_only {
        &raw_stdout
    } else {
        &raw
    };
    let filtered = filter_fn(text_to_filter, exit_code);

    let raw_for_tracking = if opts.filter_stdout_only {
        &raw_stdout
    } else {
        &raw
    };

    let shown = if durable_run.is_some() {
        emit_guarded(
            &filtered,
            recovery_hint(durable_run.as_ref()).as_deref(),
            raw_for_tracking,
        )
    } else if let Some(label) = opts.tee_label {
        print_with_hint(&filtered, &raw, raw_for_tracking, label, exit_code)
    } else {
        let guarded = crate::core::guard::never_worse(raw_for_tracking, &filtered).to_string();
        if opts.no_trailing_newline {
            print!("{}", guarded);
        } else {
            println!("{}", guarded);
        }
        guarded
    };

    timer.track(
        cmd_label,
        &format!("contextdroid {}", cmd_label),
        raw_for_tracking,
        &shown,
    );
    finalize_durable(durable_run, outcome, &shown, false);
    Ok(exit_code)
}

pub fn run(
    mut cmd: Command,
    tool_name: &str,
    args_display: &str,
    mode: RunMode<'_>,
    opts: RunOptions<'_>,
) -> Result<i32> {
    let timer = tracking::TimedExecution::start();
    let cmd_label = format!("{} {}", tool_name, args_display);

    match mode {
        RunMode::Filtered(filter_fn) => run_captured_filter(
            cmd,
            tool_name,
            &cmd_label,
            move |text, _| filter_fn(text),
            opts,
            timer,
        ),
        RunMode::FilteredWithExit(filter_fn) => run_captured_filter(
            cmd,
            tool_name,
            &cmd_label,
            move |text, exit_code| filter_fn(text, exit_code),
            opts,
            timer,
        ),
        RunMode::Streamed(filter) => {
            if let Some((run, outcome, raw_stdout, raw_stderr)) =
                capture_durably(&mut cmd, &cmd_label, false)
                    .with_context(|| format!("Failed to run {}", tool_name))?
            {
                let exit_code = outcome.shell_exit_code();
                let raw = format!("{}{}", raw_stdout, raw_stderr);
                let mut filter = filter;
                let mut filtered = String::new();
                for line in raw_stdout.lines().chain(raw_stderr.lines()) {
                    if let Some(output) = filter.feed_line(line) {
                        filtered.push_str(&output);
                    }
                }
                filtered.push_str(&filter.flush());
                if let Some(post) = filter.on_exit(exit_code, &raw) {
                    filtered.push_str(&post);
                }
                let shown = emit_guarded(&filtered, recovery_hint(Some(&run)).as_deref(), &raw);
                finalize_durable(Some(run), outcome, &shown, false);
                timer.track(
                    &cmd_label,
                    &format!("contextdroid {}", cmd_label),
                    &raw,
                    &shown,
                );
                return Ok(exit_code);
            }

            let result =
                stream::run_streaming(&mut cmd, StdinMode::Null, FilterMode::Streaming(filter))
                    .with_context(|| format!("Failed to run {}", tool_name))?;

            if let Some(label) = opts.tee_label {
                if let Some(hint) =
                    crate::core::tee::tee_and_hint(&result.raw, label, result.exit_code)
                {
                    println!("{}", hint);
                }
            }

            timer.track(
                &cmd_label,
                &format!("contextdroid {}", cmd_label),
                &result.raw,
                &result.filtered,
            );
            Ok(result.exit_code)
        }
        RunMode::Passthrough => {
            let result =
                stream::run_streaming(&mut cmd, StdinMode::Inherit, FilterMode::Passthrough)
                    .with_context(|| format!("Failed to run {}", tool_name))?;

            timer.track_passthrough(&cmd_label, &format!("rtk {} (passthrough)", cmd_label));
            Ok(result.exit_code)
        }
    }
}

pub fn run_filtered<F>(
    cmd: Command,
    tool_name: &str,
    args_display: &str,
    filter_fn: F,
    opts: RunOptions<'_>,
) -> Result<i32>
where
    F: Fn(&str) -> String,
{
    run(
        cmd,
        tool_name,
        args_display,
        RunMode::Filtered(Box::new(filter_fn)),
        opts,
    )
}

pub fn run_filtered_with_exit<F>(
    cmd: Command,
    tool_name: &str,
    args_display: &str,
    filter_fn: F,
    opts: RunOptions<'_>,
) -> Result<i32>
where
    F: Fn(&str, i32) -> String,
{
    run(
        cmd,
        tool_name,
        args_display,
        RunMode::FilteredWithExit(Box::new(filter_fn)),
        opts,
    )
}

pub fn run_passthrough(tool: &str, args: &[std::ffi::OsString], verbose: u8) -> Result<i32> {
    if verbose > 0 {
        eprintln!("{} passthrough: {:?}", tool, args);
    }
    let mut cmd = crate::core::utils::resolved_command(tool);
    cmd.args(args);
    let args_str = tracking::args_display(args);
    run(
        cmd,
        tool,
        &args_str,
        RunMode::Passthrough,
        RunOptions::default(),
    )
}

pub fn run_streamed(
    cmd: Command,
    tool_name: &str,
    args_display: &str,
    filter: Box<dyn StreamFilter + '_>,
    opts: RunOptions<'_>,
) -> Result<i32> {
    run(
        cmd,
        tool_name,
        args_display,
        RunMode::Streamed(filter),
        opts,
    )
}
