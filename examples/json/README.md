# JSON Examples

These examples show how to build small wrappers around Primer's existing JSON surfaces.

They use shell plus `python3` for JSON parsing, so they work without `jq`.

Assumptions:

- `primer` is available in `PATH`, or you set `PRIMER_BIN=/path/to/primer`
- `python3` is available

Scripts:

- `status-summary.sh`: read `primer status --json` and print a compact milestone summary
- `verify-summary.sh`: run `primer verify --json`, print a compact result summary, and exit with the same status as `primer verify`
- `workstream-dashboard.sh`: read `primer workstream list --json` and print a compact repository dashboard

Usage:

```bash
bash examples/json/status-summary.sh /path/to/workspace
bash examples/json/verify-summary.sh /path/to/workspace
bash examples/json/workstream-dashboard.sh /path/to/repository
```

Optional override:

```bash
PRIMER_BIN=./target/debug/primer bash examples/json/status-summary.sh /path/to/workspace
```
