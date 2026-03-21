# Shared Command: `check`

Run verification for the current milestone.

## Inputs

- Context file with `primer_state`
- Recipe path from context file
- Current milestone `tests/check.sh`

## Behavior

1. Read and validate `primer_state`.
2. Resolve current milestone from `recipe.yaml`.
3. Execute `tests/check.sh` for current milestone.
4. Return pass/fail with script output.

## State Mutation

None. State is read-only for this command.

## Failure Modes

- Missing/malformed `primer_state`
- Unknown current milestone
- Missing `tests/check.sh`
- Script non-zero exit
