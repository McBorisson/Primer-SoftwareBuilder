#!/usr/bin/env python3
"""Conformance tests for shared command behavior and state transitions."""

from __future__ import annotations

import re
import sys
import unittest
from dataclasses import dataclass
from pathlib import Path
from typing import Any

try:
    import yaml
except Exception as exc:  # pragma: no cover
    print(f"dependency: PyYAML import failed: {exc}", file=sys.stderr)
    sys.exit(2)


ROOT = Path(__file__).resolve().parent.parent.parent
FIXTURES = ROOT / "tests" / "fixtures" / "context"
RECIPE_YAML = ROOT / "recipes" / "operating-system" / "recipe.yaml"


class StateError(ValueError):
    pass


@dataclass(frozen=True)
class PrimerState:
    recipe_id: str
    recipe_path: str
    workspace_root: str
    milestone_id: str
    verified_milestone_id: str | None
    track: str
    stack_id: str


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def load_recipe_milestones(path: Path) -> list[str]:
    doc = yaml.safe_load(read_text(path))
    items = doc.get("milestones", [])
    return [m["id"] for m in items if isinstance(m, dict) and "id" in m]


def parse_state_block(context_text: str) -> PrimerState:
    blocks = re.findall(r"```yaml\s*(.*?)```", context_text, flags=re.DOTALL)
    for raw in blocks:
        doc = yaml.safe_load(raw)
        if not isinstance(doc, dict) or "primer_state" not in doc:
            continue
        state = doc["primer_state"]
        if not isinstance(state, dict):
            raise StateError("primer_state must be a mapping")

        required = [
            "recipe_id",
            "recipe_path",
            "workspace_root",
            "milestone_id",
            "track",
            "stack_id",
        ]
        missing = [k for k in required if k not in state]
        if missing:
            raise StateError(f"missing required fields: {', '.join(missing)}")

        for k in required:
            if not isinstance(state[k], str) or not state[k].strip():
                raise StateError(f"field must be non-empty string: {k}")

        if not Path(state["recipe_path"]).is_absolute():
            raise StateError("recipe_path must be an absolute path")
        if not Path(state["workspace_root"]).is_absolute():
            raise StateError("workspace_root must be an absolute path")

        verified = state.get("verified_milestone_id")
        if verified is not None and not isinstance(verified, str):
            raise StateError("verified_milestone_id must be null or a string")

        return PrimerState(
            recipe_id=state["recipe_id"],
            recipe_path=state["recipe_path"],
            workspace_root=state["workspace_root"],
            milestone_id=state["milestone_id"],
            verified_milestone_id=verified,
            track=state["track"],
            stack_id=state["stack_id"],
        )
    raise StateError("no primer_state YAML block found")


def mark_checked(state: PrimerState, milestones: list[str], check_ok: bool) -> tuple[PrimerState, str]:
    if state.milestone_id not in milestones:
        raise StateError(f"unknown milestone_id: {state.milestone_id}")
    if not check_ok:
        return state, "check_failed"

    return (
        PrimerState(
            recipe_id=state.recipe_id,
            recipe_path=state.recipe_path,
            workspace_root=state.workspace_root,
            milestone_id=state.milestone_id,
            verified_milestone_id=state.milestone_id,
            track=state.track,
            stack_id=state.stack_id,
        ),
        "verified",
    )


def next_milestone(state: PrimerState, milestones: list[str]) -> tuple[PrimerState, str]:
    if state.milestone_id not in milestones:
        raise StateError(f"unknown milestone_id: {state.milestone_id}")
    if state.verified_milestone_id != state.milestone_id:
        return state, "not_verified"

    idx = milestones.index(state.milestone_id)
    if idx == len(milestones) - 1:
        return state, "complete"

    return (
        PrimerState(
            recipe_id=state.recipe_id,
            recipe_path=state.recipe_path,
            workspace_root=state.workspace_root,
            milestone_id=milestones[idx + 1],
            verified_milestone_id=None,
            track=state.track,
            stack_id=state.stack_id,
        ),
        "advanced",
    )


def status_payload(state: PrimerState, milestones: list[str]) -> dict[str, Any]:
    if state.milestone_id not in milestones:
        raise StateError(f"unknown milestone_id: {state.milestone_id}")
    idx = milestones.index(state.milestone_id)
    next_id = None if idx == len(milestones) - 1 else milestones[idx + 1]
    return {
        "recipe_id": state.recipe_id,
        "workspace_root": state.workspace_root,
        "track": state.track,
        "stack_id": state.stack_id,
        "current_milestone_id": state.milestone_id,
        "current_verified": state.verified_milestone_id == state.milestone_id,
        "completed_count": idx,
        "total_count": len(milestones),
        "next_milestone_id": next_id,
    }


class SharedCommandConformanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.milestones = load_recipe_milestones(RECIPE_YAML)

    def test_parse_happy_fixture(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        self.assertEqual(state.recipe_id, "operating-system")
        self.assertEqual(state.recipe_path, "/tmp/primer/recipes/operating-system")
        self.assertEqual(state.workspace_root, "/tmp/my-os")
        self.assertEqual(state.milestone_id, "01-bootloader")
        self.assertIsNone(state.verified_milestone_id)
        self.assertEqual(state.track, "learner")
        self.assertEqual(state.stack_id, "c-x86")

    def test_parse_invalid_fixture_missing_track(self) -> None:
        with self.assertRaises(StateError):
            parse_state_block(read_text(FIXTURES / "invalid-missing-track.md"))

    def test_check_marks_current_milestone_verified(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        nxt, result = mark_checked(state, self.milestones, check_ok=True)
        self.assertEqual(result, "verified")
        self.assertEqual(nxt.verified_milestone_id, "01-bootloader")

    def test_check_failure_no_state_change(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        nxt, result = mark_checked(state, self.milestones, check_ok=False)
        self.assertEqual(result, "check_failed")
        self.assertEqual(nxt, state)

    def test_next_milestone_requires_prior_check(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        nxt, result = next_milestone(state, self.milestones)
        self.assertEqual(result, "not_verified")
        self.assertEqual(nxt, state)

    def test_next_milestone_happy_path(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        verified, _ = mark_checked(state, self.milestones, check_ok=True)
        nxt, result = next_milestone(verified, self.milestones)
        self.assertEqual(result, "advanced")
        self.assertEqual(nxt.milestone_id, "02-kernel-entry")
        self.assertIsNone(nxt.verified_milestone_id)
        self.assertEqual(nxt.recipe_id, state.recipe_id)
        self.assertEqual(nxt.track, state.track)
        self.assertEqual(nxt.stack_id, state.stack_id)

    def test_next_milestone_final_is_complete_no_state_change(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "final-08.md"))
        nxt, result = next_milestone(state, self.milestones)
        self.assertEqual(result, "complete")
        self.assertEqual(nxt, state)

    def test_status_payload(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        payload = status_payload(state, self.milestones)
        self.assertFalse(payload["current_verified"])
        self.assertEqual(payload["completed_count"], 0)
        self.assertEqual(payload["total_count"], 8)
        self.assertEqual(payload["next_milestone_id"], "02-kernel-entry")

    def test_status_final_payload(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "final-08.md"))
        payload = status_payload(state, self.milestones)
        self.assertTrue(payload["current_verified"])
        self.assertEqual(payload["completed_count"], 7)
        self.assertIsNone(payload["next_milestone_id"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
