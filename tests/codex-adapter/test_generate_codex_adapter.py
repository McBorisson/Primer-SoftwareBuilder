#!/usr/bin/env python3
"""Tests for Codex adapter generation and parity with shared contracts."""

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


class CodexAdapterGenerationTests(unittest.TestCase):
    def test_generation_creates_expected_files(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-codex-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-codex-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            self.assertTrue((out / "AGENTS.md").exists())
            self.assertTrue((out / ".codex" / "build.md").exists())
            self.assertTrue((out / ".codex" / "next-milestone.md").exists())
            self.assertTrue((out / ".codex" / "check.md").exists())
            self.assertTrue((out / ".codex" / "explain.md").exists())
            self.assertTrue((out / ".codex" / "status.md").exists())

    def test_state_block_defaults_and_recipe_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-codex-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-codex-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            content = read(out / "AGENTS.md")
            self.assertIn("primer_state:", content)
            self.assertIn("recipe_id: operating-system", content)
            self.assertIn(f"recipe_path: {RECIPE_DIR.as_posix()}", content)
            self.assertIn(f"workspace_root: {out.resolve().as_posix()}", content)
            self.assertIn("milestone_id: 01-bootloader", content)
            self.assertIn("verified_milestone_id: null", content)
            self.assertIn("track: learner", content)
            self.assertIn("stack_id: c-x86", content)
            self.assertIn(".codex/build.md", content)

    def test_track_and_milestone_overrides(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-codex-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-codex-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
                "--track",
                "builder",
                "--milestone-id",
                "03-vga-output",
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)
            content = read(out / "AGENTS.md")
            self.assertIn("track: builder", content)
            self.assertIn("milestone_id: 03-vga-output", content)

    def test_shared_tasks_are_copied_exactly(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-codex-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-codex-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr)

            for name in ["build.md", "next-milestone.md", "check.md", "explain.md", "status.md"]:
                generated = read(out / ".codex" / name)
                shared = read(SHARED_DIR / name)
                self.assertEqual(generated, shared, msg=f"mismatch in {name}")

    def test_invalid_milestone_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-codex-gen-") as tmp:
            out = Path(tmp)
            result = run_cmd(
                "scripts/generate-codex-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out),
                "--milestone-id",
                "99-nope",
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("is not declared in milestones", result.stderr)

    def test_command_parity_with_claude_adapter(self) -> None:
        with tempfile.TemporaryDirectory(prefix="primer-parity-") as tmp:
            base = Path(tmp)
            out_claude = base / "claude"
            out_codex = base / "codex"
            out_claude.mkdir(parents=True, exist_ok=True)
            out_codex.mkdir(parents=True, exist_ok=True)

            c_result = run_cmd(
                "scripts/generate-claude-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out_claude),
            )
            self.assertEqual(c_result.returncode, 0, msg=c_result.stderr)

            x_result = run_cmd(
                "scripts/generate-codex-adapter",
                str(RECIPE_DIR),
                "--output-dir",
                str(out_codex),
            )
            self.assertEqual(x_result.returncode, 0, msg=x_result.stderr)

            for name in ["build.md", "next-milestone.md", "check.md", "explain.md", "status.md"]:
                claude_cmd = read(out_claude / ".claude" / "commands" / name)
                codex_cmd = read(out_codex / ".codex" / name)
                self.assertEqual(claude_cmd, codex_cmd, msg=f"parity mismatch in {name}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
