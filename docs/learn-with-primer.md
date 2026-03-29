# Learn With Primer

Use this path if you are new to Primer, new to milestone-based AI workflows, or deliberately learning while you build.

Primer is a good teaching workflow when you want:

- one bounded milestone at a time
- visible progress instead of vague prompt drift
- explanations before or alongside code changes
- verification that shows when a step is actually done

## Recommended Starting Path

Start with the `cli-tool` recipe and `--track learner`.

Why this path:

- the verification loop is fast
- the milestones are easy to inspect
- the project is practical without being overwhelming
- it shows Primer's core loop clearly

Use these commands first:

```bash
primer list
primer init cli-tool --tool codex --track learner --path ~/projects/task-cli-demo
cd ~/projects/task-cli-demo
primer doctor cli-tool --milestone 01-bootstrap
```

After setup, open the generated workspace in your AI tool and use the Primer actions inside that workspace:

- `primer-status`
- `primer-build`
- `primer-verify`
- `primer-next-milestone`
- `primer-explain`

## Best Demo

For the shortest end-to-end walkthrough, use [5-minute-primer.md](./5-minute-primer.md).

That demo shows the full trust loop:

1. initialize
2. inspect status
3. verify too early and fail
4. build one milestone
5. verify successfully
6. advance safely

## How To Learn Well With Primer

Keep the workflow strict:

- read the current milestone before coding
- implement only the current milestone scope
- treat failed verification as information, not as progress
- use `primer explain` when you want deeper context
- stay in learner track until the step boundaries feel natural

## Good Follow-Ups

After `cli-tool`, the next best step is usually `interpreter-mini`.

Use `operating-system` later, once you want a more advanced guided lab and are comfortable with Primer's workflow conventions.

## If You Want To Contribute Learning Paths

See [community-recipes.md](./community-recipes.md) and [../CONTRIBUTING.md](../CONTRIBUTING.md).
