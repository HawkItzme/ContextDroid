use crate::core::run_store::{RunId, RunStore};
use anyhow::{Context, Result};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowView {
    Summary,
    Errors,
    Warnings,
    Causes,
    Json,
    Raw,
}

pub fn render_run(store: &RunStore, id: &RunId, view: ShowView) -> Result<Vec<u8>> {
    let run = store.load(id)?;
    match view {
        ShowView::Summary => Ok(run.read_summary()?.into_bytes()),
        ShowView::Json => Ok(run.read_diagnostics()?.into_bytes()),
        ShowView::Raw => {
            store.mark_recovery_requested(id)?;
            crate::core::run_analytics::mark_recovery_silent(id.as_str());
            render_raw(run.read_stdout()?, run.read_stderr()?)
        }
        ShowView::Errors => filter_events(&run.read_diagnostics()?, "error"),
        ShowView::Warnings => filter_events(&run.read_diagnostics()?, "warning"),
        ShowView::Causes => render_causes(&run.read_diagnostics()?),
    }
}

fn render_raw(stdout: Vec<u8>, stderr: Vec<u8>) -> Result<Vec<u8>> {
    if stdout.is_empty() {
        return Ok(stderr);
    }
    if stderr.is_empty() {
        return Ok(stdout);
    }
    let mut output = Vec::with_capacity(stdout.len() + stderr.len() + 32);
    output.extend_from_slice(b"== stdout ==\n");
    output.extend_from_slice(&stdout);
    if !stdout.ends_with(b"\n") {
        output.push(b'\n');
    }
    output.extend_from_slice(b"== stderr ==\n");
    output.extend_from_slice(&stderr);
    Ok(output)
}

fn diagnostics_value(json: &str) -> Result<Value> {
    serde_json::from_str(json).context("diagnostics artifact is corrupt")
}

fn filter_events(json: &str, severity: &str) -> Result<Vec<u8>> {
    let mut value = diagnostics_value(json)?;
    let events = value
        .get_mut("events")
        .and_then(Value::as_array_mut)
        .context("diagnostics artifact has no events array")?;
    events.retain(|event| event.get("severity").and_then(Value::as_str) == Some(severity));
    serde_json::to_vec_pretty(&value).context("failed to render diagnostics")
}

fn render_causes(json: &str) -> Result<Vec<u8>> {
    let value = diagnostics_value(json)?;
    let events = value
        .get("events")
        .and_then(Value::as_array)
        .context("diagnostics artifact has no events array")?;
    let mut output = String::new();
    for cause in events
        .iter()
        .filter_map(|event| event.get("causes"))
        .filter_map(Value::as_array)
        .flatten()
    {
        if let Some(message) = cause.get("message").and_then(Value::as_str) {
            output.push_str(message);
            output.push('\n');
        }
    }
    Ok(output.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::run_store::{FinalizeDetails, ProcessOutcome, RunStart};

    fn stored_run() -> (tempfile::TempDir, RunStore, RunId) {
        let temp = tempfile::tempdir().unwrap();
        let store = RunStore::new(temp.path().to_path_buf());
        let mut run = store
            .start(RunStart {
                command: "./gradlew lintDebug".into(),
                cwd: "/workspace/app".into(),
                profile: "contextdroid-safe".into(),
                output_mode: "balanced".into(),
            })
            .unwrap();
        let id = run.id().clone();
        run.write_stdout(b"stdout raw\n").unwrap();
        run.write_stderr(b"stderr raw\n").unwrap();
        run.finalize(
            ProcessOutcome::ExitCode(1),
            r#"{"schema_version":1,"events":[{"severity":"error","message":"bad resource","causes":[{"message":"missing color"}]},{"severity":"warning","message":"deprecated API","causes":[]}]}"#,
            "compact summary",
            FinalizeDetails::default(),
        )
        .unwrap();
        (temp, store, id)
    }

    #[test]
    fn test_show_raw_labels_separate_streams_and_marks_recovery() {
        let (_temp, store, id) = stored_run();

        let output = render_run(&store, &id, ShowView::Raw).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "== stdout ==\nstdout raw\n== stderr ==\nstderr raw\n"
        );
        assert!(store.load(&id).unwrap().metadata.recovery_requested);
    }

    #[test]
    fn test_show_errors_and_warnings_filter_structured_events() {
        let (_temp, store, id) = stored_run();

        let errors = String::from_utf8(render_run(&store, &id, ShowView::Errors).unwrap()).unwrap();
        let warnings =
            String::from_utf8(render_run(&store, &id, ShowView::Warnings).unwrap()).unwrap();

        assert!(errors.contains("bad resource"));
        assert!(!errors.contains("deprecated API"));
        assert!(warnings.contains("deprecated API"));
        assert!(!warnings.contains("bad resource"));
    }

    #[test]
    fn test_show_causes_preserves_cause_messages() {
        let (_temp, store, id) = stored_run();

        let causes = String::from_utf8(render_run(&store, &id, ShowView::Causes).unwrap()).unwrap();

        assert_eq!(causes, "missing color\n");
    }

    #[test]
    fn test_show_summary_and_json_return_stored_artifacts() {
        let (_temp, store, id) = stored_run();

        assert_eq!(
            String::from_utf8(render_run(&store, &id, ShowView::Summary).unwrap()).unwrap(),
            "compact summary"
        );
        let json = String::from_utf8(render_run(&store, &id, ShowView::Json).unwrap()).unwrap();
        assert!(json.contains("schema_version"));
        assert!(json.contains("deprecated API"));
    }
}
