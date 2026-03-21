# Shared Command: `next-milestone`

Advance to the next milestone only after current verification passes.

## Inputs

- Context file with `primer_state`
- Recipe path from context file
- Current milestone `tests/check.sh`
- Current milestone `demo.sh`

## Behavior

1. Read and validate `primer_state`.
2. Resolve current milestone from `recipe.yaml`.
3. Run `tests/check.sh` for current milestone.
4. If check fails, stop and return failure; do not update state.
5. Run `demo.sh` for current milestone.
6. If demo fails, stop and return failure; do not update state.
7. If current milestone is final, return completion summary; do not update state.
8. Otherwise set `primer_state.milestone_id` to the next declared milestone.
9. Load next milestone `spec.md` and `agent.md`.
10. Follow track behavior:
  - learner: introduce and ask required learner question(s)
  - builder: begin implementation directly

## State Mutation

- Allowed: `primer_state.milestone_id`
- Forbidden: changes to `recipe_id`, `track`, `stack_id`

## Failure Modes

- Missing/malformed `primer_state`
- Unknown current milestone
- `check.sh` non-zero
- `demo.sh` non-zero
