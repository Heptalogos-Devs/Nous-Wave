#!/usr/bin/env python3
"""Check source-file size against the repository's source-shape policy.

The checker intentionally measures physical lines. It is a small structural
guardrail, not a parser or a substitute for ownership-level review.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from dataclasses import asdict, dataclass
from typing import Iterable, Sequence

from config.source_shape import (
    EXCLUDED_PARTS,
    FAIL_LINES,
    SOURCE_SUFFIXES,
    WARN_LINES,
)

ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class Finding:
    path: str
    lines: int


@dataclass(frozen=True)
class Report:
    scanned_files: int
    warnings: tuple[Finding, ...]
    failures: tuple[Finding, ...]


def relative_path(path: Path) -> Path:
    return path.relative_to(ROOT)


def excluded(path: Path) -> bool:
    rel = relative_path(path)
    return any(part in EXCLUDED_PARTS for part in rel.parts)


def physical_lines(path: Path) -> int:
    with path.open("rb") as f:
        return sum(1 for _ in f)


def validate_policy() -> None:
    if not 0 < WARN_LINES < FAIL_LINES:
        raise ValueError(
            "source-shape policy requires 0 < warning threshold < failure threshold"
        )
    if not SOURCE_SUFFIXES:
        raise ValueError("source-shape policy must define at least one source suffix")


def resolve_scope(raw_path: str) -> Path:
    candidate = Path(raw_path)
    if not candidate.is_absolute():
        candidate = ROOT / candidate

    resolved = candidate.resolve()
    try:
        resolved.relative_to(ROOT)
    except ValueError as exc:
        raise ValueError(f"scan path is outside the repository: {raw_path}") from exc

    if not resolved.exists():
        raise ValueError(f"scan path does not exist: {raw_path}")
    return resolved


def iter_scope_files(scope: Path) -> Iterable[Path]:
    if scope.is_file():
        yield scope
        return

    for directory, directories, filenames in os.walk(scope, topdown=True):
        directory_path = Path(directory)
        directories[:] = sorted(
            name
            for name in directories
            if name not in EXCLUDED_PARTS
            and not (directory_path / name).is_symlink()
        )
        for name in sorted(filenames):
            path = directory_path / name
            if not path.is_symlink():
                yield path


def iter_source_files(scopes: Sequence[Path]) -> Iterable[Path]:
    seen: set[Path] = set()
    for scope in scopes:
        for path in iter_scope_files(scope):
            if path.suffix not in SOURCE_SUFFIXES or excluded(path):
                continue
            resolved = path.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            yield resolved


def scan(scopes: Sequence[Path]) -> Report:
    warnings: list[Finding] = []
    failures: list[Finding] = []
    scanned_files = 0

    for path in sorted(iter_source_files(scopes)):
        scanned_files += 1
        count = physical_lines(path)
        finding = Finding(relative_path(path).as_posix(), count)
        if count > FAIL_LINES:
            failures.append(finding)
        elif count > WARN_LINES:
            warnings.append(finding)

    return Report(scanned_files, tuple(warnings), tuple(failures))


def arguments(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--path",
        action="append",
        dest="paths",
        metavar="PATH",
        help="scan a repository-relative file or directory; repeatable",
    )
    parser.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
        help="select human-readable or machine-readable output",
    )
    return parser.parse_args(argv)


def print_text(report: Report) -> None:
    for finding in report.warnings:
        print(f"WARN source-shape {finding.lines:5d} lines  {finding.path}")

    for finding in report.failures:
        print(f"FAIL source-shape {finding.lines:5d} lines  {finding.path}")

    if report.failures:
        print(
            f"\n{len(report.failures)} Rust source file(s) exceed the hard "
            f"{FAIL_LINES}-line limit.",
            file=sys.stderr,
        )
        return

    print(
        "source-shape OK "
        f"({report.scanned_files} file(s) scanned; "
        f"{len(report.warnings)} above warning threshold {WARN_LINES}; "
        f"none above hard limit {FAIL_LINES})"
    )


def print_json(report: Report) -> None:
    payload = {
        "status": "fail" if report.failures else "pass",
        "scanned_files": report.scanned_files,
        "warnings": [asdict(finding) for finding in report.warnings],
        "failures": [asdict(finding) for finding in report.failures],
        "thresholds": {"warning_lines": WARN_LINES, "failure_lines": FAIL_LINES},
    }
    print(json.dumps(payload, indent=2))


def main(argv: Sequence[str] | None = None) -> int:
    args = arguments(sys.argv[1:] if argv is None else argv)
    try:
        validate_policy()
        scopes = [resolve_scope(raw_path) for raw_path in (args.paths or ["."])]
        report = scan(scopes)
    except (OSError, ValueError) as exc:
        print(f"source-shape ERROR: {exc}", file=sys.stderr)
        return 2

    if args.format == "json":
        print_json(report)
    else:
        print_text(report)
    return 1 if report.failures else 0

if __name__ == "__main__":
    raise SystemExit(main())
