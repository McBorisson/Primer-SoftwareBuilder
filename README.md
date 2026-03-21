# Primer

Primer is a learner-first recipe library for building substantial software projects with AI coding agents using milestone contracts, checks, and demos.

## Table of Contents

- [Start here (learners)](#start-here-learners)
- [Available recipes (projects)](#available-recipes-projects)
- [How to start any recipe](#how-to-start-any-recipe)
- [Prerequisites](#prerequisites)
- [Milestone workflow (inside your AI tool)](#milestone-workflow-inside-your-ai-tool)
- [Repository layout](#repository-layout)
- [Contributing](#contributing)

## Start here (learners)

If you want to start a project, follow this flow:

1. Pick a recipe from **Available recipes** below.
2. Generate adapter files for your AI tool.
3. Open your project with that tool and follow milestone commands.
4. Use `build` to work only on the current milestone.
5. Use `check` to verify that milestone.
6. Use `next-milestone` only after `check` has passed.

## Available recipes (projects)

Current catalog:

| Recipe ID | Project | Difficulty | Path |
|---|---|---|---|
| `operating-system` | Build Your Own Operating System | `hard` | `recipes/operating-system` |

To list recipe folders directly:

```bash
find recipes -mindepth 1 -maxdepth 1 -type d -exec basename {} \;
```

## How to start any recipe

Do not use the `primer` repo itself as the learner project workspace. Create a separate target directory so project files like `Makefile`, `boot.asm`, and `kernel.c` belong to the learner project, not this recipe library.

This matters because the learner is building a real project. The project `Makefile` and source tree are expected to evolve milestone by milestone, but those changes should happen in the learner workspace, not inside the recipe library.

Set the recipe and target workspace:

```bash
PRIMER_ROOT=/path/to/primer
RECIPE_ID=operating-system
mkdir -p ../my-os
cd ../my-os
```

Validate recipe contract:

```bash
"$PRIMER_ROOT/scripts/validate-recipe" "$PRIMER_ROOT/recipes/$RECIPE_ID"
```

Generate Claude Code adapter files:

```bash
"$PRIMER_ROOT/scripts/generate-claude-adapter" "$PRIMER_ROOT/recipes/$RECIPE_ID" --output-dir .
```

This creates:

- `CLAUDE.md`
- `.claude/commands/next-milestone.md`
- `.claude/commands/check.md`
- `.claude/commands/explain.md`
- `.claude/commands/status.md`

Generate Codex adapter files:

```bash
"$PRIMER_ROOT/scripts/generate-codex-adapter" "$PRIMER_ROOT/recipes/$RECIPE_ID" --output-dir .
```

This creates:

- `AGENTS.md`
- `.codex/next-milestone.md`
- `.codex/check.md`
- `.codex/explain.md`
- `.codex/status.md`

## Prerequisites

- `python3`: used by generators and validation scripts
- `nasm`: used from milestone 01
- `qemu-system-i386`: used to run and verify the OS image
- `make`: used inside the learner project workspace
- `i686-elf-gcc` and `i686-elf-ld`: required from milestone 02 onward for bare-metal 32-bit C on macOS/Linux

Why these matter:

- The recipe library generates instructions and commands.
- The learner project workspace contains the actual source files and project `Makefile`.
- The OS milestones progressively require a real cross-compilation toolchain; system compilers are not enough for the later bare-metal C milestones.

## Milestone workflow (inside your AI tool)

Use these commands/tasks milestone by milestone:

- `build`: implement only the current milestone, step by step
- `check`: run current milestone verification and mark it verified on success
- `next-milestone`: advance only after the current milestone is already verified
- `explain`: read the deep-dive explanation
- `status`: show current milestone and progress

## Repository layout

- `recipe-spec.md`: canonical recipe contract for v0.1
- `recipes/`: recipes and milestone contracts
- `adapters/_shared/`: shared command behavior + state model
- `scripts/`: validators and adapter generators
- `tests/`: recipe validation, shared command conformance, adapter generation tests

## Contributing

See `CONTRIBUTING.md` for contributor checks, quality gates, adapter standards, and test requirements.
