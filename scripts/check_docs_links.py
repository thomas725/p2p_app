#!/usr/bin/env python3
"""Check that every relative link in the repository's Markdown files resolves.

Renaming or moving a document is the easiest way to leave a broken link
behind, and a broken link in a doc nobody opens is invisible until someone
does open it. This walks the tracked Markdown files, extracts the links, and
verifies that each relative target exists.

Usage:
    python3 scripts/check_docs_links.py [path ...]

With no arguments it checks every tracked ``*.md`` file. Exits 0 when all
links resolve, 1 otherwise.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Inline links: [text](target) — the target may be <bracketed>.
INLINE_LINK = re.compile(r"!?\[[^\]]*\]\(\s*(<[^>]*>|[^)\s]+)(?:\s+[\"'][^)]*[\"'])?\s*\)")
# Reference definitions: [id]: target
REF_DEF = re.compile(r"^\s{0,3}\[[^\]]+\]:\s*(<[^>]*>|\S+)", re.MULTILINE)
# Fenced code blocks are skipped wholesale: a shell comment that looks like a
# link is not a link.
FENCE = re.compile(r"^```.*?^```", re.MULTILINE | re.DOTALL)
# Inline code spans are skipped too: this document itself writes
# `[text](target)` to show what the checker looks for, and that is not a link.
INLINE_CODE = re.compile(r"(`+)(?!`).*?\1", re.DOTALL)

# Links that are not files on disk.
SKIP_PREFIXES = ("http://", "https://", "mailto:", "tel:", "data:", "//")


def iter_markdown_files(paths: list[str]) -> list[Path]:
    """Return the Markdown files to check."""
    if paths:
        files: list[Path] = []
        for raw in paths:
            path = Path(raw)
            if path.is_dir():
                files.extend(sorted(path.rglob("*.md")))
            elif path.suffix == ".md":
                files.append(path)
        return files

    # No arguments: use git to find tracked Markdown files, so build outputs
    # and vendored trees are never scanned. Fall back to a filesystem walk when
    # the script runs outside a git checkout.
    try:
        import subprocess

        out = subprocess.run(
            ["git", "ls-files", "*.md"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        tracked = [Path(line) for line in out.splitlines() if line.strip()]
        # `.github/workflow-logs/` holds machine-generated snapshots of past
        # workflow runs. They are history, not documentation, and a link that
        # no longer resolves there is not something a reader can act on.
        tracked = [p for p in tracked if not str(p).startswith(".github/workflow-logs/")]
        if tracked:
            return sorted(tracked)
    except (OSError, subprocess.CalledProcessError, ImportError):
        pass
    return sorted(p for p in Path(".").rglob("*.md") if "workflow-logs" not in str(p))


def extract_targets(text: str) -> list[str]:
    """Return every link target in a Markdown document."""
    text = FENCE.sub("", text)
    text = INLINE_CODE.sub("", text)
    targets = [m.group(1) for m in INLINE_LINK.finditer(text)]
    targets += [m.group(1) for m in REF_DEF.finditer(text)]
    return targets


def resolve(target: str, source: Path) -> Path | None:
    """Map a link target to a path on disk, or None when it is not a file link."""
    target = target.strip()
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1]
    if not target or target.startswith("#"):
        return None
    if target.startswith(SKIP_PREFIXES):
        return None
    # Drop the anchor and query string.
    target = target.split("#", 1)[0].split("?", 1)[0]
    if not target:
        return None
    if target.startswith("/"):
        return Path(target)
    return (source.parent / target).resolve()


def main(argv: list[str]) -> int:
    files = iter_markdown_files(argv[1:])
    if not files:
        print("no Markdown files found")
        return 0

    broken: list[tuple[Path, str, str]] = []
    checked = 0

    for path in files:
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as exc:
            print(f"error: cannot read {path}: {exc}", file=sys.stderr)
            return 1

        for target in extract_targets(text):
            resolved = resolve(target, path)
            if resolved is None:
                continue
            checked += 1
            if not resolved.exists():
                broken.append((path, target, str(resolved)))

    for source, target, resolved in broken:
        print(f"broken link: {source} -> {target}  (looked for {resolved})")

    if broken:
        print(f"\n{len(broken)} broken link(s) in {len(files)} file(s)")
        return 1

    print(f"ok: {checked} relative link(s) across {len(files)} Markdown file(s) resolve")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
