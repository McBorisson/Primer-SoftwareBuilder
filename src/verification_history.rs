use anyhow::{Context, Result};
use serde::Serialize;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::state::PrimerState;

static RECORD_COUNTER: AtomicU64 = AtomicU64::new(1);

pub enum VerificationOutcome {
    Passed,
    Failed,
}

pub struct VerificationCommand<'a> {
    pub program: &'a OsStr,
    pub args: &'a [OsString],
    pub script: &'a Path,
}

#[derive(Serialize)]
struct VerificationRecord {
    schema_version: u32,
    recipe_id: String,
    workspace_root: PathBuf,
    milestone_id: String,
    track: String,
    outcome: &'static str,
    recorded_at_unix_ms: u128,
    duration_ms: u128,
    verified_state_after: bool,
    cleared_prior_verified_state: bool,
    exit_code: Option<i32>,
    summary: Option<String>,
    command: VerificationCommandRecord,
}

#[derive(Serialize)]
struct VerificationCommandRecord {
    program: String,
    args: Vec<String>,
    script_path: PathBuf,
}

#[allow(clippy::too_many_arguments)]
pub fn write_record(
    state: &PrimerState,
    command: &VerificationCommand<'_>,
    outcome: VerificationOutcome,
    duration: Duration,
    exit_code: Option<i32>,
    verified_state_after: bool,
    cleared_prior_verified_state: bool,
    summary: Option<&str>,
) -> Result<PathBuf> {
    let records_dir = state
        .workspace_root
        .join(".primer")
        .join("runtime")
        .join("verifications")
        .join(&state.milestone_id);
    fs::create_dir_all(&records_dir)
        .with_context(|| format!("failed to create {}", records_dir.display()))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is set before UNIX_EPOCH")?;
    let counter = RECORD_COUNTER.fetch_add(1, Ordering::Relaxed);
    let filename = format!(
        "{}-{:09}-{}-{}.json",
        timestamp.as_secs(),
        timestamp.subsec_nanos(),
        std::process::id(),
        counter
    );
    let path = records_dir.join(filename);

    let record = VerificationRecord {
        schema_version: 1,
        recipe_id: state.recipe_id.clone(),
        workspace_root: state.workspace_root.clone(),
        milestone_id: state.milestone_id.clone(),
        track: state.track.clone(),
        outcome: match outcome {
            VerificationOutcome::Passed => "passed",
            VerificationOutcome::Failed => "failed",
        },
        recorded_at_unix_ms: timestamp.as_millis(),
        duration_ms: duration.as_millis(),
        verified_state_after,
        cleared_prior_verified_state,
        exit_code,
        summary: summary.map(ToOwned::to_owned),
        command: VerificationCommandRecord {
            program: command.program.to_string_lossy().into_owned(),
            args: command
                .args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            script_path: command.script.to_path_buf(),
        },
    };

    let json =
        serde_json::to_string_pretty(&record).context("failed to serialize verification record")?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}
