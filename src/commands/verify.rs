use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Instant;
#[cfg(windows)]
use which::which;

use crate::cli::VerifyArgs;
use crate::retry_guidance::{self, RetryAssessment, RetryLevel};
use crate::state;
use crate::ui;
use crate::verification_history::{self, VerificationCommand, VerificationOutcome};
use crate::workflow;
use crate::workstream_resume;

#[derive(Serialize)]
struct VerifyJson {
    source: VerifySourceJson,
    track: String,
    workspace: String,
    milestone: VerifyMilestoneJson,
    next_milestone: Option<VerifyMilestoneRefJson>,
    command: VerifyCommandJson,
    outcome: String,
    duration_ms: u128,
    exit_code: Option<i32>,
    verified_state_after: bool,
    cleared_prior_verified_state: bool,
    verification: VerifySummaryJson,
    retry_signal: VerifyRetrySignalJson,
    verification_gate_after: VerifyGateJson,
    record_path: String,
    state_file: String,
    command_stdout: String,
    command_stderr: String,
    next_steps: Vec<String>,
}

#[derive(Serialize)]
struct VerifySourceJson {
    kind: String,
    id: String,
}

#[derive(Serialize)]
struct VerifyMilestoneJson {
    id: String,
    title: String,
}

#[derive(Serialize)]
struct VerifyMilestoneRefJson {
    id: String,
    title: String,
}

#[derive(Serialize)]
struct VerifyCommandJson {
    program: String,
    args: Vec<String>,
    script_path: String,
}

#[derive(Serialize)]
struct VerifySummaryJson {
    attempts: usize,
    passed_attempts: usize,
    failed_attempts: usize,
    failure_streak: usize,
}

#[derive(Serialize)]
struct VerifyRetrySignalJson {
    level: String,
    label: String,
}

#[derive(Serialize)]
struct VerifyGateJson {
    state: String,
    summary: String,
}

struct VerifyExecution {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

pub fn run(workspace_hint: &Path, args: VerifyArgs) -> Result<()> {
    let mut state = state::load_from_workspace(workspace_hint)?;
    let workflow = workflow::load(&state.source)?;
    let milestone = workflow::resolve_initial_milestone(&workflow, Some(&state.milestone_id))?;
    let milestone = milestone.clone();
    let milestone_index = workflow::milestone_index(&workflow, &state.milestone_id)?;
    let next_milestone = workflow.milestones.get(milestone_index + 1).cloned();

    let checks_dir = workflow
        .path
        .join("milestones")
        .join(&milestone.id)
        .join("tests");
    let verify_command = resolve_verify_command(&checks_dir)?;

    if !args.json {
        ui::section("Primer verify");
        println!();
        ui::info(&format!(
            "Running {} for {} from {}",
            ui::code(verify_command.script.display().to_string()),
            ui::code(&milestone.id),
            ui::code(state.workspace_root.display().to_string())
        ));
        println!();
    }

    let started_at = Instant::now();
    let execution = run_verify_command(&verify_command, &state.workspace_root, args.json)
        .with_context(|| format!("failed to execute {}", verify_command.script.display()))?;
    let duration = started_at.elapsed();
    let verification_command = VerificationCommand {
        program: &verify_command.program,
        args: &verify_command.args,
        script: &verify_command.script,
    };

    if !execution.status.success() {
        let cleared_prior_verified_state =
            state.verified_milestone_id.as_deref() == Some(milestone.id.as_str());
        if cleared_prior_verified_state {
            state.verified_milestone_id = None;
            state::write(&state)?;
            workstream_resume::sync_from_state(&state)?;
        }

        let record_path = verification_history::write_record(
            &state,
            &verification_command,
            VerificationOutcome::Failed,
            duration,
            execution.status.code(),
            false,
            cleared_prior_verified_state,
            Some("milestone verification failed"),
        )?;
        let verification_summary = verification_history::summarize_for_milestone(&state)?;
        let retry_assessment = retry_guidance::assess(&verification_summary);

        if args.json {
            let json = VerifyJson::from_failure(
                &state,
                &milestone,
                next_milestone.as_ref(),
                &verify_command,
                &execution,
                duration.as_millis(),
                cleared_prior_verified_state,
                &verification_summary,
                retry_assessment,
                &record_path,
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&json)
                    .context("failed to serialize verification output")?
            );
        } else {
            eprintln!();
            eprintln!(
                "Verification history for {}: {} attempt{}, {} failed, current failure streak {}.",
                milestone.id,
                verification_summary.attempts,
                if verification_summary.attempts == 1 {
                    ""
                } else {
                    "s"
                },
                verification_summary.failed_attempts,
                retry_assessment.failure_streak
            );
            if retry_assessment.should_suggest_explain() {
                eprintln!("Run primer explain for more context before the next retry.");
            }
            if retry_assessment.should_surface_if_stuck() {
                if let Some(split_if_stuck) = milestone.split_if_stuck.as_ref() {
                    eprintln!("If stuck: {split_if_stuck}");
                }
                eprintln!(
                    "If a different mode would help, switch tracks with `primer track learner` or `primer track builder`."
                );
            }
            if retry_assessment.should_flag_scope_risk() {
                eprintln!(
                    "This milestone may be too large or unclear. Consider splitting or clarifying it before more retries."
                );
            }
        }

        if cleared_prior_verified_state {
            bail!(
                "milestone {} verification failed; current verified state was cleared",
                milestone.id
            );
        }

        bail!("milestone {} verification failed", milestone.id);
    }

    state.verified_milestone_id = Some(milestone.id.clone());
    state::write(&state)?;
    workstream_resume::sync_from_state(&state)?;
    let record_path = verification_history::write_record(
        &state,
        &verification_command,
        VerificationOutcome::Passed,
        duration,
        execution.status.code(),
        true,
        false,
        Some("milestone verification passed"),
    )?;
    let verification_summary = verification_history::summarize_for_milestone(&state)?;
    let retry_assessment = retry_guidance::assess(&verification_summary);

    if args.json {
        let json = VerifyJson::from_success(
            &state,
            &milestone,
            next_milestone.as_ref(),
            &verify_command,
            &execution,
            duration.as_millis(),
            &verification_summary,
            retry_assessment,
            &record_path,
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&json)
                .context("failed to serialize verification output")?
        );
        return Ok(());
    }

    println!();
    ui::success(&format!("Verified {}", milestone.id));
    println!(
        "The current milestone is now marked as verified in {}. You can use the {} next.",
        ui::code(state.context_path.display().to_string()),
        ui::reference("skill", "primer-next-milestone")
    );

    Ok(())
}

impl VerifyJson {
    #[allow(clippy::too_many_arguments)]
    fn from_success(
        state: &state::PrimerState,
        milestone: &crate::recipe::Milestone,
        next_milestone: Option<&crate::recipe::Milestone>,
        verify_command: &CheckCommand,
        execution: &VerifyExecution,
        duration_ms: u128,
        verification_summary: &verification_history::VerificationSummary,
        retry_assessment: RetryAssessment,
        record_path: &Path,
    ) -> Self {
        VerifyJson {
            source: VerifySourceJson {
                kind: state.source.kind.as_str().to_string(),
                id: state.source.id.clone(),
            },
            track: state.track.clone(),
            workspace: state.workspace_root.display().to_string(),
            milestone: VerifyMilestoneJson {
                id: milestone.id.clone(),
                title: milestone.title.clone(),
            },
            next_milestone: next_milestone.map(|next| VerifyMilestoneRefJson {
                id: next.id.clone(),
                title: next.title.clone(),
            }),
            command: VerifyCommandJson::from_check_command(verify_command),
            outcome: "passed".to_string(),
            duration_ms,
            exit_code: execution.status.code(),
            verified_state_after: true,
            cleared_prior_verified_state: false,
            verification: VerifySummaryJson {
                attempts: verification_summary.attempts,
                passed_attempts: verification_summary.passed_attempts,
                failed_attempts: verification_summary.failed_attempts,
                failure_streak: verification_summary.failure_streak,
            },
            retry_signal: VerifyRetrySignalJson {
                level: retry_level_key(retry_assessment.level).to_string(),
                label: retry_assessment.label(),
            },
            verification_gate_after: VerifyGateJson {
                state: "open".to_string(),
                summary: "open - current milestone is verified".to_string(),
            },
            record_path: record_path.display().to_string(),
            state_file: state.context_path.display().to_string(),
            command_stdout: execution.stdout.clone(),
            command_stderr: execution.stderr.clone(),
            next_steps: success_next_steps(next_milestone),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn from_failure(
        state: &state::PrimerState,
        milestone: &crate::recipe::Milestone,
        next_milestone: Option<&crate::recipe::Milestone>,
        verify_command: &CheckCommand,
        execution: &VerifyExecution,
        duration_ms: u128,
        cleared_prior_verified_state: bool,
        verification_summary: &verification_history::VerificationSummary,
        retry_assessment: RetryAssessment,
        record_path: &Path,
    ) -> Self {
        VerifyJson {
            source: VerifySourceJson {
                kind: state.source.kind.as_str().to_string(),
                id: state.source.id.clone(),
            },
            track: state.track.clone(),
            workspace: state.workspace_root.display().to_string(),
            milestone: VerifyMilestoneJson {
                id: milestone.id.clone(),
                title: milestone.title.clone(),
            },
            next_milestone: next_milestone.map(|next| VerifyMilestoneRefJson {
                id: next.id.clone(),
                title: next.title.clone(),
            }),
            command: VerifyCommandJson::from_check_command(verify_command),
            outcome: "failed".to_string(),
            duration_ms,
            exit_code: execution.status.code(),
            verified_state_after: false,
            cleared_prior_verified_state,
            verification: VerifySummaryJson {
                attempts: verification_summary.attempts,
                passed_attempts: verification_summary.passed_attempts,
                failed_attempts: verification_summary.failed_attempts,
                failure_streak: verification_summary.failure_streak,
            },
            retry_signal: VerifyRetrySignalJson {
                level: retry_level_key(retry_assessment.level).to_string(),
                label: retry_assessment.label(),
            },
            verification_gate_after: VerifyGateJson {
                state: "blocked".to_string(),
                summary: "blocked - latest verification failed".to_string(),
            },
            record_path: record_path.display().to_string(),
            state_file: state.context_path.display().to_string(),
            command_stdout: execution.stdout.clone(),
            command_stderr: execution.stderr.clone(),
            next_steps: failure_next_steps(milestone, retry_assessment),
        }
    }
}

impl VerifyCommandJson {
    fn from_check_command(command: &CheckCommand) -> Self {
        VerifyCommandJson {
            program: command.program.to_string_lossy().into_owned(),
            args: command
                .args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            script_path: command.script.display().to_string(),
        }
    }
}

fn success_next_steps(next_milestone: Option<&crate::recipe::Milestone>) -> Vec<String> {
    match next_milestone {
        Some(next) => vec![
            format!(
                "Run primer next-milestone when you are ready to advance to {}",
                next.id
            ),
            "Run primer status to confirm the verification gate is open".to_string(),
        ],
        None => vec![
            "Workflow is complete.".to_string(),
            "Run primer status to confirm the final workflow state".to_string(),
        ],
    }
}

fn failure_next_steps(
    milestone: &crate::recipe::Milestone,
    retry_assessment: RetryAssessment,
) -> Vec<String> {
    let mut steps = vec![format!(
        "Run primer verify again for {} after fixing the current milestone",
        milestone.id
    )];
    if retry_assessment.should_suggest_explain() {
        steps.push("Run primer explain for more context before the next retry.".to_string());
    }
    if retry_assessment.should_surface_if_stuck() {
        if let Some(split_if_stuck) = milestone.split_if_stuck.as_ref() {
            steps.push(format!("If stuck: {split_if_stuck}"));
        }
        steps.push(
            "If a different mode would help, switch tracks with primer track learner or primer track builder."
                .to_string(),
        );
    }
    if retry_assessment.should_flag_scope_risk() {
        steps.push(
            "This milestone may be too large or unclear. Consider splitting or clarifying it before more retries."
                .to_string(),
        );
    }
    steps
}

fn run_verify_command(
    verify_command: &CheckCommand,
    workspace_root: &Path,
    capture_output: bool,
) -> Result<VerifyExecution> {
    if capture_output {
        let output = Command::new(&verify_command.program)
            .args(&verify_command.args)
            .current_dir(workspace_root)
            .output()?;
        return Ok(VerifyExecution {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let status = Command::new(&verify_command.program)
        .args(&verify_command.args)
        .current_dir(workspace_root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    Ok(VerifyExecution {
        status,
        stdout: String::new(),
        stderr: String::new(),
    })
}

fn retry_level_key(level: RetryLevel) -> &'static str {
    match level {
        RetryLevel::Clear => "clear",
        RetryLevel::Retrying => "retrying",
        RetryLevel::Stuck => "stuck",
        RetryLevel::Escalating => "escalating",
    }
}

struct CheckCommand {
    program: OsString,
    args: Vec<OsString>,
    script: PathBuf,
}

fn resolve_verify_command(checks_dir: &Path) -> Result<CheckCommand> {
    let verify_shell_script = checks_dir.join("verify.sh");
    let check_shell_script = checks_dir.join("check.sh");

    #[cfg(windows)]
    {
        let verify_cmd_script = checks_dir.join("verify.cmd");
        if verify_cmd_script.is_file() {
            return Ok(CheckCommand {
                program: OsString::from("cmd.exe"),
                args: vec![
                    OsString::from("/D"),
                    OsString::from("/C"),
                    verify_cmd_script.as_os_str().to_os_string(),
                ],
                script: verify_cmd_script,
            });
        }

        let verify_powershell_script = checks_dir.join("verify.ps1");
        if verify_powershell_script.is_file() {
            return Ok(CheckCommand {
                program: OsString::from("powershell"),
                args: vec![
                    OsString::from("-ExecutionPolicy"),
                    OsString::from("Bypass"),
                    OsString::from("-File"),
                    verify_powershell_script.as_os_str().to_os_string(),
                ],
                script: verify_powershell_script,
            });
        }

        if verify_shell_script.is_file() && which("bash").is_ok() {
            return Ok(CheckCommand {
                program: OsString::from("bash"),
                args: vec![verify_shell_script.as_os_str().to_os_string()],
                script: verify_shell_script,
            });
        }

        let check_cmd_script = checks_dir.join("check.cmd");
        if check_cmd_script.is_file() {
            return Ok(CheckCommand {
                program: OsString::from("cmd.exe"),
                args: vec![
                    OsString::from("/D"),
                    OsString::from("/C"),
                    check_cmd_script.as_os_str().to_os_string(),
                ],
                script: check_cmd_script,
            });
        }

        let check_powershell_script = checks_dir.join("check.ps1");
        if check_powershell_script.is_file() {
            return Ok(CheckCommand {
                program: OsString::from("powershell"),
                args: vec![
                    OsString::from("-ExecutionPolicy"),
                    OsString::from("Bypass"),
                    OsString::from("-File"),
                    check_powershell_script.as_os_str().to_os_string(),
                ],
                script: check_powershell_script,
            });
        }

        if check_shell_script.is_file() && which("bash").is_ok() {
            return Ok(CheckCommand {
                program: OsString::from("bash"),
                args: vec![check_shell_script.as_os_str().to_os_string()],
                script: check_shell_script,
            });
        }

        bail!(
            "milestone verification script not found for Windows in {}; expected verify.cmd, verify.ps1, verify.sh, check.cmd, check.ps1, or check.sh with bash on PATH",
            checks_dir.display()
        );
    }

    #[cfg(not(windows))]
    {
        if verify_shell_script.is_file() {
            return Ok(CheckCommand {
                program: OsString::from("bash"),
                args: vec![verify_shell_script.as_os_str().to_os_string()],
                script: verify_shell_script,
            });
        }

        if check_shell_script.is_file() {
            return Ok(CheckCommand {
                program: OsString::from("bash"),
                args: vec![check_shell_script.as_os_str().to_os_string()],
                script: check_shell_script,
            });
        }

        bail!(
            "milestone verification script not found in {}; expected verify.sh or check.sh",
            checks_dir.display()
        );
    }
}
