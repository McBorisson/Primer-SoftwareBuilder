# Build Safely With Primer

Use this path if you already want to ship code with an AI agent, but you want tighter boundaries than a broad prompt.

Primer is strongest when you use it as a verification-first execution loop:

1. inspect the current milestone
2. build only that scope
3. run verification
4. advance only after it passes

## The Core Safety Rules

Primer is designed around a few hard constraints:

- one milestone should mean one bounded capability change
- verification is the gate for progress
- state should stay visible at every step
- changing tracks should not require reinitializing the workspace

If you keep those rules intact, the workflow stays predictable.

## The Commands That Matter During Execution

Inside an active workspace, the main commands are:

- `primer status`
- `primer build`
- `primer verify`
- `primer next-milestone`
- `primer explain`
- `primer track learner`
- `primer track builder`

The highest-signal loop is:

```text
primer-status
primer-build
primer-verify
primer-next-milestone
```

## How To Stay Safe In Practice

Use these habits:

- run `primer status` before and after verification-heavy work
- use `primer build` as the contract for what is in scope right now
- do not treat partial implementation as progress until `primer verify` passes
- if retries start stacking up, use `primer status` and `primer explain` before broadening the change
- switch to learner track when you need more explanation, and back to builder track when the next step is clear

## Machine-Readable Integrations

If you want wrappers and automation instead of raw terminal output, use Primer's existing JSON surfaces.

Start with [../examples/json/README.md](../examples/json/README.md):

- `status-summary.sh`
- `verify-summary.sh`
- `workstream-dashboard.sh`

Those examples show how to react to `status --json`, `verify --json`, and `workstream list --json` without adding more CLI flags.

## Brownfield Safety

If you are working inside an existing repository instead of a generated recipe workspace, use [use-primer-in-existing-repo.md](./use-primer-in-existing-repo.md).
