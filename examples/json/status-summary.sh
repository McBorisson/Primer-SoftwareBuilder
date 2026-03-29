#!/usr/bin/env bash
set -euo pipefail

workspace_root="${1:-.}"
primer_bin="${PRIMER_BIN:-primer}"

status_json="$(
  cd "$workspace_root"
  "$primer_bin" status --json
)"

STATUS_JSON="$status_json" python3 - <<'PY'
import json
import os

data = json.loads(os.environ["STATUS_JSON"])
current = data["current_milestone"]
goal = current.get("goal") or "not set"
last_attempt = data["verification"].get("last")
last_summary = last_attempt["summary"] if last_attempt else "no verification attempts yet"
next_steps = data.get("next_steps") or []
next_step = next_steps[0] if next_steps else "none"

print(f"Workspace: {data['workspace']}")
print(f"Source: {data['source']['kind']} {data['source']['id']}")
print(f"Milestone: {current['id']} ({current['title']})")
print(f"Workflow state: {data['workflow_state']}")
print(
    "Verification gate: "
    f"{data['verification_gate']['state']} - {data['verification_gate']['summary']}"
)
print(f"Retry signal: {data['retry_signal']['label']}")
print(f"Goal: {goal}")
print(f"Last verification: {last_summary}")
print(f"Next step: {next_step}")
PY
