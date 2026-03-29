#!/usr/bin/env bash
set -euo pipefail

workspace_root="${1:-.}"
primer_bin="${PRIMER_BIN:-primer}"

set +e
verify_json="$(
  cd "$workspace_root"
  "$primer_bin" verify --json
)"
verify_status=$?
set -e

if [ -z "$verify_json" ]; then
  echo "primer verify --json produced no stdout" >&2
  exit "$verify_status"
fi

VERIFY_JSON="$verify_json" python3 - <<'PY'
import json
import os

data = json.loads(os.environ["VERIFY_JSON"])
next_steps = data.get("next_steps") or []
next_step = next_steps[0] if next_steps else "none"
command_output = (
    data.get("command_stdout", "").strip()
    or data.get("command_stderr", "").strip()
    or "no verify command output"
)
command_preview = command_output.splitlines()[0]
summary = "verified and safe to advance" if data["outcome"] == "passed" else "still blocked"

print(f"Milestone: {data['milestone']['id']} ({data['milestone']['title']})")
print(f"Outcome: {data['outcome']} - {summary}")
print(f"Verification gate: {data['verification_gate_after']['summary']}")
print(f"Retry signal: {data['retry_signal']['label']}")
print(
    "Attempts: "
    f"{data['verification']['attempts']} total, "
    f"{data['verification']['failed_attempts']} failed"
)
print(
    "Verify command: "
    f"{data['command']['program']} {' '.join(data['command']['args'])}"
)
print(f"Command output: {command_preview}")
print(f"Next step: {next_step}")
PY

exit "$verify_status"
