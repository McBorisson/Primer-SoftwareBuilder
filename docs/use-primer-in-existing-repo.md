# Use Primer In An Existing Repository

Use this path when you want Primer's milestone loop inside a repository that already exists.

Primer handles this through repository-local workstreams instead of recipe initialization.

## The Brownfield Flow

From the repository root:

```bash
primer workstream list
primer workstream analyze --goal "Reduce auth pipeline complexity"
primer workstream init auth-refactor --goal "Reduce auth pipeline complexity" --tool codex --track learner
primer workstream switch auth-refactor
```

That flow does four important things:

- shows whether workstreams already exist
- suggests likely first-milestone boundaries
- scaffolds a repository-local workstream
- activates the chosen workstream in the root adapter context

## What Primer Writes

For a workstream such as `auth-refactor`, Primer writes:

- `.primer/workstreams/auth-refactor/workstream.yaml`
- `.primer/workstreams/auth-refactor/workstream-intent.md`
- `.primer/workstreams/auth-refactor/milestones/...`
- `.primer/runtime/workstreams/auth-refactor/...`

The intent file is deliberately small. Use it to keep:

- the goal
- non-goals
- constraints
- done-when

short and durable while you refine the real milestone sequence.

## Recommended Working Loop

After initialization, the loop looks almost the same as a recipe-backed workspace:

1. run `primer build`
2. inspect the current milestone and optional workstream intent
3. implement only that milestone
4. run `primer verify`
5. advance with `primer next-milestone` only after verification passes

If you create more than one workstream, use `primer workstream switch <workstream-id>` to move between them safely.

## Useful JSON Surfaces

Brownfield automation usually starts with:

- `primer workstream list --json`
- `primer workstream analyze --json`
- `primer status --json`
- `primer verify --json`

If you want a compact repo view immediately, start with [../examples/json/workstream-dashboard.sh](../examples/json/workstream-dashboard.sh).
