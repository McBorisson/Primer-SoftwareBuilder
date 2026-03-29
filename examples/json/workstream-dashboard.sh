#!/usr/bin/env bash
set -euo pipefail

repository_root="${1:-.}"
primer_bin="${PRIMER_BIN:-primer}"

workstreams_json="$(
  cd "$repository_root"
  "$primer_bin" workstream list --json
)"

WORKSTREAMS_JSON="$workstreams_json" python3 - <<'PY'
import json
import os

data = json.loads(os.environ["WORKSTREAMS_JSON"])
rows = data.get("workstreams") or []

print(f"Repository: {data['repository']}")
print(f"Active workstream: {data.get('active_workstream_id') or 'none'}")
print()

if not rows:
    print("No workstreams found.")
    raise SystemExit(0)

headers = ["Workstream", "Status", "Milestone", "Verified", "Count"]
table = []
for item in rows:
    table.append(
        [
            item["id"],
            "active" if item["active"] else "available",
            item["current_milestone_id"],
            "yes" if item["verified"] else "no",
            str(item["milestone_count"]),
        ]
    )

widths = [len(header) for header in headers]
for row in table:
    widths = [max(width, len(cell)) for width, cell in zip(widths, row)]

print("  ".join(header.ljust(width) for header, width in zip(headers, widths)))
print("  ".join("-" * width for width in widths))
for row in table:
    print("  ".join(cell.ljust(width) for cell, width in zip(row, widths)))
PY
