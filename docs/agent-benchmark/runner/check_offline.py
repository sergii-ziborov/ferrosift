#!/usr/bin/env python3
"""Validate agent-benchmark manifests and run FerroSift offline criteria.

This is a skeleton for FS-A09. It does not drive an LLM. Agent runs against
variants A–D are recorded separately; this script only checks fixture shape
and the FerroSift clean-process path (variants C/D offline oracles).
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TASKS = ROOT / "tasks"


def ferrosift_bin() -> str:
    return os.environ.get("FERROSIFT_BIN") or shutil.which("ferrosift") or "ferrosift"


def load_tasks(task_id: str | None) -> list[Path]:
    dirs = sorted(p for p in TASKS.iterdir() if p.is_dir())
    if task_id:
        dirs = [p for p in dirs if p.name == task_id]
        if not dirs:
            raise SystemExit(f"unknown task: {task_id}")
    return dirs


def validate_task(task_dir: Path) -> dict:
    manifest_path = task_dir / "task.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema") != "ferrosift.agent_benchmark.task.v1":
        raise SystemExit(f"{manifest_path}: unexpected schema")
    for key in ("id", "group", "title", "prompt", "input", "success", "variants"):
        if key not in manifest:
            raise SystemExit(f"{manifest_path}: missing {key}")
    input_path = task_dir / manifest["input"]
    if not input_path.is_file():
        raise SystemExit(f"{manifest_path}: missing input {input_path.name}")
    return manifest


def run_cmd(args: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )


def check_run_recipe(task_dir: Path, manifest: dict) -> None:
    offline = manifest["offline_ferrosift"]
    with tempfile.TemporaryDirectory(prefix="fs-bench-") as tmp:
        tmp_path = Path(tmp)
        recipe = tmp_path / "recipe.json"
        recipe.write_text(json.dumps(offline["recipe"]), encoding="utf-8")
        output = tmp_path / "out.txt"
        proc = run_cmd(
            [
                ferrosift_bin(),
                "run",
                "--format",
                offline["format"],
                "--input-kind",
                manifest.get("input_kind", "bytes"),
                "--recipe",
                str(recipe),
                "--input",
                str(task_dir / manifest["input"]),
                "--output",
                str(output),
            ]
        )
        if proc.returncode != 0:
            raise SystemExit(proc.stderr or proc.stdout or "ferrosift run failed")
        expected = (task_dir / manifest["success"]["path"]).read_text(encoding="utf-8").rstrip("\n")
        actual = output.read_text(encoding="utf-8").rstrip("\n")
        if actual != expected:
            raise SystemExit(
                f"{manifest['id']}: output mismatch\nexpected={expected!r}\nactual={actual!r}"
            )


def check_repro(task_dir: Path, manifest: dict) -> None:
    offline = manifest["offline_ferrosift"]
    with tempfile.TemporaryDirectory(prefix="fs-bench-") as tmp:
        tmp_path = Path(tmp)
        recipe = tmp_path / "recipe.json"
        recipe.write_text(json.dumps(offline["recipe"]), encoding="utf-8")
        case_dir = tmp_path / "case"
        export = run_cmd(
            [
                ferrosift_bin(),
                "repro",
                "export",
                "--format",
                offline["format"],
                "--input-kind",
                manifest.get("input_kind", "bytes"),
                "--recipe",
                str(recipe),
                "--input",
                str(task_dir / manifest["input"]),
                "--out-dir",
                str(case_dir),
            ]
        )
        if export.returncode != 0:
            raise SystemExit(export.stderr or export.stdout or "repro export failed")
        check = run_cmd([ferrosift_bin(), "repro", "check", "--case", str(case_dir)])
        if check.returncode != 0:
            raise SystemExit(check.stderr or check.stdout or "repro check failed")
        expected = (task_dir / manifest["success"]["path"]).read_bytes().rstrip(b"\n")
        # expected.json is a tagged FerroSift Value.
        value = json.loads((case_dir / "expected.json").read_text(encoding="utf-8"))
        if value.get("kind") == "text":
            actual = value["value"]["text"].encode("utf-8")
        elif value.get("kind") == "bytes":
            actual = bytes(value["value"])
        else:
            raise SystemExit(f"{manifest['id']}: unsupported expected kind {value.get('kind')}")
        if actual != expected:
            raise SystemExit(
                f"{manifest['id']}: repro value mismatch vs independent file"
            )


def check_candidates(task_dir: Path, manifest: dict) -> None:
    offline = manifest["offline_ferrosift"]
    proc = run_cmd(
        [
            ferrosift_bin(),
            "candidates",
            "--input-kind",
            manifesto_input_kind(manifest),
            "--input",
            str(task_dir / manifest["input"]),
            "--candidates",
            str(task_dir / offline["candidates"]),
        ]
    )
    if proc.returncode != 0:
        raise SystemExit(proc.stderr or proc.stdout or "candidates failed")
    report = json.loads(proc.stdout)
    if report.get("schema") != "ferrosift.candidates.v1":
        raise SystemExit(f"{manifest['id']}: unexpected candidates schema")
    warnings = report.get("warnings") or []
    if not any("calibrated" in w for w in warnings):
        raise SystemExit(f"{manifest['id']}: missing non-probability warning")
    winning = (task_dir / offline["winning_id_file"]).read_text(encoding="utf-8").strip()
    passed = [
        row["id"]
        for row in report["results"]
        if row.get("error_code") is None
        and row.get("checks_total", 0) > 0
        and row.get("checks_passed") == row.get("checks_total")
    ]
    if winning not in passed or len(passed) != 1:
        raise SystemExit(
            f"{manifest['id']}: expected sole winner {winning!r}, got {passed!r}"
        )


def manifesto_input_kind(manifest: dict) -> str:
    return manifest.get("input_kind", "bytes")


def run_offline(task_dir: Path, manifest: dict) -> None:
    offline = manifest.get("offline_ferrosift")
    if not offline:
        print(f"SKIP {manifest['id']}: no offline_ferrosift oracle")
        return
    kind = offline["kind"]
    if kind == "run_recipe":
        check_run_recipe(task_dir, manifest)
    elif kind == "repro_export_check":
        check_repro(task_dir, manifest)
    elif kind == "candidates_file":
        check_candidates(task_dir, manifest)
    else:
        raise SystemExit(f"{manifest['id']}: unknown offline kind {kind}")
    print(f"PASS {manifest['id']} ({kind})")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", help="Run a single task id directory name")
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="Only validate task.json shape and input presence",
    )
    args = parser.parse_args()

    variants = json.loads((ROOT / "variants.json").read_text(encoding="utf-8"))
    if variants.get("schema") != "ferrosift.agent_benchmark.variants.v1":
        raise SystemExit("variants.json: unexpected schema")
    if {v["id"] for v in variants["variants"]} != {"A", "B", "C", "D"}:
        raise SystemExit("variants.json: expected ids A–D")

    for task_dir in load_tasks(args.task):
        manifest = validate_task(task_dir)
        print(f"OK  {manifest['id']} manifest")
        if not args.validate_only:
            run_offline(task_dir, manifest)


if __name__ == "__main__":
    main()
