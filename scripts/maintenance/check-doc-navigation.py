#!/usr/bin/env python3
"""Check local Markdown targets, return paths and ancestor INDEX coverage."""
import argparse
import tomllib
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[2]


def local_targets(path):
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"(?ms)^\s*(```|~~~).*?^\s*\1[^\n]*$", "", text)
    references = dict(re.findall(r"(?m)^\s*\[([^]]+)\]:\s*(\S+)", text))
    destinations = re.findall(r"(?<!!)\[[^]]*\]\((<[^>]+>|[^\s)]+)(?:\s+[^)]*)?\)", text)
    for label, reference in re.findall(r"(?<!!)\[([^]]+)\]\[([^]]*)\]", text):
        destination = references.get(reference or label)
        if destination:
            destinations.append(destination)
    for destination in destinations:
        url = urlsplit(destination.strip("<>"))
        if url.scheme or url.netloc or not url.path:
            continue
        yield (path.parent / unquote(url.path)).resolve()


def check(config_path=None):
    with (config_path or ROOT / ".config/scripts/doc-navigation.toml").open("rb") as stream:
        config = tomllib.load(stream)
    ignored = tuple(directory.rstrip("/") + "/" for directory in config["ignored_directories"])
    exempt_names = set(config["return_exempt_names"])
    exempt_paths = {ROOT / name for name in config["return_exempt_paths"]}
    names = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT,
    ).decode().split("\0")
    documents = {
        (ROOT / name).resolve()
        for name in names
        if name.endswith(".md") and not name.startswith(ignored)
        and (ROOT / name).is_file()
    }
    routes = {path for path in documents if path.name == "INDEX.md"}
    human = {path for path in documents if path.name not in exempt_names}
    links = {path: set(local_targets(path)) for path in documents}
    errors = []

    def label(path):
        return path.relative_to(ROOT).as_posix()

    for source, targets in links.items():
        for target in targets:
            if not target.exists():
                errors.append(f"BROKEN {label(source)} -> {target}")
    for page in sorted(human):
        if page in exempt_paths:
            continue
        parents = [route for route in routes if route.parent in page.parents]
        owner = max(parents, key=lambda route: len(route.parts)) if parents else None
        if owner is None or page not in links[owner]:
            errors.append(f"UNINDEXED {label(page)} ({label(owner) if owner else 'no ancestor INDEX'})")
        incoming = {source for source in human | routes if page in links[source]}
        if not incoming:
            errors.append(f"ORPHAN {label(page)}")
        elif not (links[page] & incoming):
            errors.append(f"NO_RETURN {label(page)}")
    for error in errors:
        print(error)
    print(f"documents={len(documents)} human={len(human)} indexes={len(routes)} issues={len(errors)}")
    return bool(errors)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=ROOT / ".config/scripts/doc-navigation.toml")
    args = parser.parse_args()
    sys.exit(check(args.config))
