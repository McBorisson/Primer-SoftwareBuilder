# Contributing to Primer

Thanks for helping improve Primer.

We want Primer to become a trustworthy community library of guided learning paths. That is why the project is contract-driven: changes are accepted only when schema, structure, and behavior remain deterministic and verifiable.

If you want to contribute a new recipe, start with [docs/community-recipes.md](docs/community-recipes.md).

## Ways to Contribute

Not every contribution needs to be a full new recipe. Useful contributions include:

- new recipes
- improvements to an existing recipe's milestone boundaries, explanations, demos, or checks
- adapter and CLI improvements that make the workflow safer or easier to use
- documentation improvements that make the learning path easier to start and easier to finish

If you want to help shape the educational environment around Primer, those contributions are in scope too.

## How to Start

Use the lightest path that fits the size of the change:

- small documentation or test fixes: open a PR directly
- larger recipe, adapter, or CLI changes: open an issue or draft PR first
- new recipes: start with a recipe proposal and link to [docs/community-recipes.md](docs/community-recipes.md)

We do not currently use a separate discussion forum in this repo. For design discussion and early feedback, use GitHub issues or draft PRs.

## Local setup

Prerequisites:

- Bash
- Rust toolchain
- Cargo
- `pre-commit` (recommended for local validation)

Install and enable the git hooks:

```bash
pre-commit install
```

Typical flow:

1. Fork the repo and create a branch for your change.
2. Make the smallest change that proves the improvement.
3. Run the required checks.
4. Open a PR with a short summary of what changed and how you verified it.

## Core rules

1. Follow `recipe-spec.md` for recipe structure and schema.
2. Keep shared command behavior in `adapters/_shared/` as the source of truth.
3. Do not introduce adapter-specific behavior that contradicts shared command contracts.
4. Ensure tests remain green before opening a PR.

## Community Direction

Strong contributions usually optimize for:

- safety by default
- visible progress at each milestone
- deterministic verification
- high teaching value
- workflows that remain clear inside the supported AI tools

If a contribution adds breadth but makes the learning experience fuzzier, it is probably the wrong tradeoff.

## Before You Build a Full Recipe

If you are considering a new recipe, start with the framing before you write every milestone.

Open an issue or draft PR with:

- the recipe idea
- the intended learner outcome
- a milestone list
- expected local prerequisites
- what the learner should be able to demo at the end

This usually makes review faster and helps keep the scope teachable.

## Required checks before PR

Run the automated checks:

```bash
pre-commit run --all-files
```

And run the automated test suite:

```bash
cargo test
```

For recipe contributions, also review the recipe's own `README.md` and milestone scripts to make sure the documented prerequisites and checks still match reality.

## Recipe quality bar

A recipe is ready when:

- Every milestone includes `spec.md`, `agent.md`, `explanation.md`, `tests/check.sh`, and `demo.sh`.
- `agent.md` includes both learner and builder tracks.
- Learner track asks at least one question per milestone.
- Check scripts produce clear, actionable failures.
- Demo scripts are runnable and aligned with milestone goals.

A strong recipe also has:

- milestone names that describe visible progress
- explanations that help the learner understand why the step matters
- realistic prerequisites for a normal developer machine
- a scope that is narrow enough to review and finish

We are more likely to merge a focused, well-tested recipe than a broad ambitious one with fuzzy milestone boundaries.

## Adapter changes

When changing adapter generation:

1. Update the Rust generator logic in `src/adapter.rs`.
2. Keep outputs aligned with `adapters/_shared/*`.
3. Add or update Rust tests for generated outputs.

## Documentation expectations

Documentation is part of the product surface.

For recipe and workflow changes, make sure:

- the root `README.md` still points users to the right starting path
- the recipe `README.md` matches the actual prerequisites and demo flow
- contributor-facing docs still describe the current quality bar clearly

## PR expectations

Keep commits scoped where practical, for example schema, adapter, recipe, or tests.

In your PR description, include:

- which commands you ran
- pass/fail status
- known limitations or follow-up work
