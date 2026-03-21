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
    milestone_id: str
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
        required = ["recipe_id", "milestone_id", "track", "stack_id"]
        missing = [k for k in required if k not in state]
        if missing:
            raise StateError(f"missing required fields: {', '.join(missing)}")
        for k in required:
            if not isinstance(state[k], str) or not state[k].strip():
                raise StateError(f"field must be non-empty string: {k}")
        return PrimerState(
            recipe_id=state["recipe_id"],
            milestone_id=state["milestone_id"],
            track=state["track"],
            stack_id=state["stack_id"],
        )
    raise StateError("no primer_state YAML block found")


def next_milestone(
    state: PrimerState,
    milestones: list[str],
    check_ok: bool,
    demo_ok: bool,
) -> tuple[PrimerState, str]:
    if state.milestone_id not in milestones:
        raise StateError(f"unknown milestone_id: {state.milestone_id}")
    if not check_ok:
        return state, "check_failed"
    if not demo_ok:
        return state, "demo_failed"
    idx = milestones.index(state.milestone_id)
    if idx == len(milestones) - 1:
        return state, "complete"

    return (
        PrimerState(
            recipe_id=state.recipe_id,
            milestone_id=milestones[idx + 1],
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
        "track": state.track,
        "stack_id": state.stack_id,
        "current_milestone_id": state.milestone_id,
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
        self.assertEqual(state.milestone_id, "01-bootloader")
        self.assertEqual(state.track, "learner")
        self.assertEqual(state.stack_id, "c-x86")

    def test_parse_invalid_fixture_missing_track(self) -> None:
        with self.assertRaises(StateError):
            parse_state_block(read_text(FIXTURES / "invalid-missing-track.md"))

    def test_next_milestone_happy_path(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        nxt, result = next_milestone(state, self.milestones, check_ok=True, demo_ok=True)
        self.assertEqual(result, "advanced")
        self.assertEqual(nxt.milestone_id, "02-kernel-entry")
        self.assertEqual(nxt.recipe_id, state.recipe_id)
        self.assertEqual(nxt.track, state.track)
        self.assertEqual(nxt.stack_id, state.stack_id)

    def test_next_milestone_check_failure_no_state_change(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        nxt, result = next_milestone(state, self.milestones, check_ok=False, demo_ok=True)
        self.assertEqual(result, "check_failed")
        self.assertEqual(nxt, state)

    def test_next_milestone_demo_failure_no_state_change(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        nxt, result = next_milestone(state, self.milestones, check_ok=True, demo_ok=False)
        self.assertEqual(result, "demo_failed")
        self.assertEqual(nxt, state)

    def test_next_milestone_final_is_complete_no_state_change(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "final-08.md"))
        nxt, result = next_milestone(state, self.milestones, check_ok=True, demo_ok=True)
        self.assertEqual(result, "complete")
        self.assertEqual(nxt, state)

    def test_status_payload(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "happy-01.md"))
        payload = status_payload(state, self.milestones)
        self.assertEqual(payload["completed_count"], 0)
        self.assertEqual(payload["total_count"], 8)
        self.assertEqual(payload["next_milestone_id"], "02-kernel-entry")

    def test_status_final_payload(self) -> None:
        state = parse_state_block(read_text(FIXTURES / "final-08.md"))
        payload = status_payload(state, self.milestones)
        self.assertEqual(payload["completed_count"], 7)
        self.assertIsNone(payload["next_milestone_id"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
