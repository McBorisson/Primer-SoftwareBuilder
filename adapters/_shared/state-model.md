# Primer State Model (v0.1)

All adapters must embed a machine-readable state block in their context file.

## Canonical Block

```yaml
primer_state:
  recipe_id: operating-system
  milestone_id: 01-bootloader
  track: learner
  stack_id: c-x86
```

## Field Rules

- `recipe_id`: required, immutable during a recipe run
- `milestone_id`: required, mutable via `next-milestone` only
- `track`: required, immutable for v0.1 command flow
- `stack_id`: required, immutable for v0.1 command flow

## Read Rules

- Commands must read `primer_state` from the context file.
- If block is missing or malformed, command fails with explicit state error.
- If `milestone_id` is not declared in recipe milestones, command fails.

## Write Rules

- `next-milestone` may update `milestone_id` only.
- `check`, `explain`, and `status` must not mutate state.
- If current milestone is final, `next-milestone` must not mutate state.

## Determinism

Given the same state, recipe, and command outcome (`check.sh`/`demo.sh` success), state transitions must be deterministic.
