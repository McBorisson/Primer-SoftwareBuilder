#!/usr/bin/env python3
"""Tests for Claude adapter generation."""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent.parent
RECIPE_DIR = ROOT / "recipes" / "operating-system"
SHARED_DIR = ROOT / "adapters" / "_shared"


def run_cmd(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(args),
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


class ClaudeAdapterGenerationTests(unittest.TestCase):
    def test_generation_creates_expected_files(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-claude-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-claude-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            self.assertTrue((out / "CLAUDE.md").exists())
            self.assertTrue((out / ".claude" / "commands" / "primer-build.md").exists())
            self.assertTrue((out / ".claude" / "commands" / "primer-next-milestone.md").exists())
            self.assertTrue((out / ".claude" / "commands" / "primer-check.md").exists())
            self.assertTrue((out / ".claude" / "commands" / "primer-explain.md").exists())
            self.assertTrue((out / ".claude" / "commands" / "primer-status.md").exists())

    def test_state_block_defaults_and_recipe_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-claude-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-claude-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            content = read(out / "CLAUDE.md")
            self.assertIn("primer_state:", content)
            self.assertIn("recipe_id: operating-system", content)
            self.assertIn(f"recipe_path: {RECIPE_DIR.as_posix()}", content)
            self.assertIn(f"workspace_root: {out.resolve().as_posix()}", content)
            self.assertIn("milestone_id: 01-bootloader", content)
            self.assertIn("verified_milestone_id: null", content)
            self.assertIn("track: learner", content)
            self.assertIn("stack_id: c-x86", content)
            self.assertIn("`primer-build`", content)
            self.assertIn(
                "Use the local `primer` CLI as the source of truth for `primer-check`, `primer-status`, `primer-explain`, and `primer-next-milestone`.",
                content,
            )

    def test_track_and_milestone_overrides(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-claude-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-claude-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
                "--track",
                "builder",
                "--milestone-id",
                "03-vga-output",
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)
            content = read(out / "CLAUDE.md")
            self.assertIn("track: builder", content)
            self.assertIn("milestone_id: 03-vga-output", content)

    def test_workflow_commands_are_cli_backed(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-claude-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-claude-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            check_command = read(out / ".claude" / "commands" / "primer-check.md")
            self.assertIn("Run `primer check` from the current workspace root.", check_command)
            self.assertIn(read(SHARED_DIR / "check.md").strip(), check_command)

            status_command = read(out / ".claude" / "commands" / "primer-status.md")
            self.assertIn("Run `primer status` from the current workspace root.", status_command)
            self.assertIn(read(SHARED_DIR / "status.md").strip(), status_command)

            explain_command = read(out / ".claude" / "commands" / "primer-explain.md")
            self.assertIn("Run `primer explain` from the current workspace root.", explain_command)
            self.assertIn(read(SHARED_DIR / "explain.md").strip(), explain_command)

            next_command = read(out / ".claude" / "commands" / "primer-next-milestone.md")
            self.assertIn(
                "Run `primer next-milestone` from the current workspace root.", next_command
            )
            self.assertIn(read(SHARED_DIR / "next-milestone.md").strip(), next_command)

    def test_build_command_uses_cli_for_context(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-claude-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-claude-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            build_command = read(out / ".claude" / "commands" / "primer-build.md")
            self.assertIn("Run `primer build` from the current workspace root.", build_command)
            self.assertIn(read(SHARED_DIR / "build.md").strip(), build_command)

    def test_invalid_milestone_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-claude-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-claude-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
                "--milestone-id",
                "99-nope",
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("is not declared in milestones", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
