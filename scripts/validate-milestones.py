#!/usr/bin/env python3
"""Validate milestone directory contract for a recipe."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

try:
    import yaml
except Exception as exc:  # pragma: no cover
    print(f"dependency: PyYAML import failed: {exc}", file=sys.stderr)
    sys.exit(2)


REQUIRED_FILES = [
    "spec.md",
    "agent.md",
    "explanation.md",
    "tests/check.sh",
    "demo.sh",
]


def error(path: Path, field: str, message: str) -> str:
    return f"{path}: {field}: {message}"


def as_recipe_dir(arg: str) -> Path:
    p = Path(arg)
    if p.is_file():
        return p.parent
    return p


def load_yaml(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as f:
        return yaml.safe_load(f)


def validate(recipe_dir: Path) -> list[str]:
    errs: list[str] = []
    recipe_yaml = recipe_dir / "recipe.yaml"
    if not recipe_yaml.exists():
        return [error(recipe_yaml, "recipe.yaml", "file not found")]

    try:
        doc = load_yaml(recipe_yaml)
    except Exception as exc:
        return [error(recipe_yaml, "recipe.yaml", f"invalid YAML: {exc}")]

    milestones = doc.get("milestones") if isinstance(doc, dict) else None
    if not isinstance(milestones, list):
        return [error(recipe_yaml, "milestones", "must be a non-empty array before milestone validation")]

    declared_ids: list[str] = []
    for i, item in enumerate(milestones):
        if not isinstance(item, dict) or not isinstance(item.get("id"), str):
            errs.append(error(recipe_yaml, f"milestones[{i}].id", "must be a string"))
            continue
        declared_ids.append(item["id"])

    milestones_root = recipe_dir / "milestones"
    if not milestones_root.exists():
        errs.append(error(milestones_root, "milestones", "directory not found"))
        return errs

    if not milestones_root.is_dir():
        errs.append(error(milestones_root, "milestones", "must be a directory"))
        return errs

    for ms_id in declared_ids:
        ms_dir = milestones_root / ms_id
        if not ms_dir.exists():
            errs.append(error(ms_dir, "milestone", "declared in recipe.yaml but directory is missing"))
            continue
        if not ms_dir.is_dir():
            errs.append(error(ms_dir, "milestone", "must be a directory"))
            continue
        for rel in REQUIRED_FILES:
            req_path = ms_dir / rel
            if not req_path.exists():
                errs.append(error(req_path, "required-file", "missing"))

    actual_dirs = sorted([p.name for p in milestones_root.iterdir() if p.is_dir()])
    declared_set = set(declared_ids)
    for name in actual_dirs:
        if name not in declared_set:
            errs.append(
                error(
                    milestones_root / name,
                    "milestone",
                    "directory exists but is not declared in recipe.yaml",
                )
            )

    return errs


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: scripts/validate-milestones.py <recipe-dir|recipe.yaml>", file=sys.stderr)
        return 2

    recipe_dir = as_recipe_dir(sys.argv[1])
    errs = validate(recipe_dir)
    if errs:
        for e in errs:
            print(e, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
