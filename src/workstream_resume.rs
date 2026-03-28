use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::PrimerState;
use crate::workflow::{self, Workflow, WorkflowSourceKind};

#[derive(Debug, Clone)]
pub struct ResolvedWorkstreamState {
    pub milestone_id: String,
    pub verified_milestone_id: Option<String>,
    pub resumed_previous_progress: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkstreamResumeRecord {
    schema_version: u32,
    milestone_id: String,
    verified_milestone_id: Option<String>,
    updated_at_unix_ms: u128,
}

pub fn sync_from_state(state: &PrimerState) -> Result<()> {
    if state.source.kind != WorkflowSourceKind::Workstream {
        return Ok(());
    }

    let path = resume_state_path(&state.workspace_root, &state.source.id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let record = WorkstreamResumeRecord {
        schema_version: 1,
        milestone_id: state.milestone_id.clone(),
        verified_milestone_id: state.verified_milestone_id.clone(),
        updated_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is set before UNIX_EPOCH")?
            .as_millis(),
    };
    let json = serde_json::to_string_pretty(&record)
        .context("failed to serialize workstream resume state")?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn resolve_for_workflow(
    workflow: &Workflow,
    workspace_root: &Path,
) -> Result<ResolvedWorkstreamState> {
    let first_milestone = workflow::resolve_initial_milestone(workflow, None)?;
    let Some(record) = load_record(workspace_root, &workflow.source.id)? else {
        return Ok(ResolvedWorkstreamState {
            milestone_id: first_milestone.id.clone(),
            verified_milestone_id: None,
            resumed_previous_progress: false,
        });
    };

    let milestone_id = if workflow
        .milestones
        .iter()
        .any(|milestone| milestone.id == record.milestone_id)
    {
        record.milestone_id
    } else {
        first_milestone.id.clone()
    };
    let verified_milestone_id = match record.verified_milestone_id {
        Some(verified_milestone_id) if verified_milestone_id == milestone_id => {
            Some(verified_milestone_id)
        }
        _ => None,
    };

    Ok(ResolvedWorkstreamState {
        resumed_previous_progress: milestone_id != first_milestone.id
            || verified_milestone_id.is_some(),
        milestone_id,
        verified_milestone_id,
    })
}

fn load_record(
    workspace_root: &Path,
    workstream_id: &str,
) -> Result<Option<WorkstreamResumeRecord>> {
    let path = resume_state_path(workspace_root, workstream_id);
    if !path.is_file() {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let record: WorkstreamResumeRecord = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse workstream resume state {}", path.display()))?;
    Ok(Some(record))
}

fn resume_state_path(workspace_root: &Path, workstream_id: &str) -> PathBuf {
    workspace_root
        .join(".primer")
        .join("runtime")
        .join("workstreams")
        .join(workstream_id)
        .join("resume-state.json")
}
