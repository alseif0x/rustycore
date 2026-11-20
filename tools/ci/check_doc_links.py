#!/usr/bin/env python3
"""Check common inline Markdown links in the supplied documents.

Checks relative file targets in `[text](target)` and `![alt](target)` links.
Fragments are ignored; fenced code and external/root-relative targets are skipped.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit


OPEN_FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
CLOSE_FENCE = re.compile(r"^ {0,3}(`+|~+)[ \t]*$")
LINK_START = re.compile(r"!?\[[^\]]*\]\(")
QUOTED_TITLE = re.compile(r"\s+(?:\"[^\"]*\"|'[^']*')\s*$")


def link_targets(line: str):
    """Yield inline link destinations, balancing path parentheses and spaces."""
    cursor = 0
    while match := LINK_START.search(line, cursor):
        start = match.end()
        depth = 1
        index = start
        while index < len(line) and depth:
            if line[index] == "\\":
                index += 2
                continue
            if line[index] == "(":
                depth += 1
            elif line[index] == ")":
                depth -= 1
            index += 1
        if depth:
            cursor = start
            continue

        destination = line[start : index - 1].strip()
        if destination.startswith("<") and ">" in destination:
            destination = destination[1 : destination.index(">")]
        else:
            destination = QUOTED_TITLE.sub("", destination).rstrip()
        yield re.sub(r"\\([()])", r"\1", destination)
        cursor = index


def document_lines(text: str):
    """Yield source lines outside fenced code blocks."""
    fence: tuple[str, int] | None = None
    for number, line in enumerate(text.splitlines(), start=1):
        if fence:
            closing = CLOSE_FENCE.match(line)
            if closing and closing.group(1)[0] == fence[0] and len(closing.group(1)) >= fence[1]:
                fence = None
            continue
        opening = OPEN_FENCE.match(line)
        if opening:
            marker = opening.group(1)
            fence = (marker[0], len(marker))
            continue
        yield number, line


def broken_links(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        return [f"{path}:1: cannot read document: {error}"]

    document = path.resolve()
    errors = []
    for number, line in document_lines(text):
        for destination in link_targets(line):
            try:
                url = urlsplit(destination)
            except ValueError:
                continue
            if url.scheme or destination.startswith("//") or url.path.startswith("/"):
                continue
            target = (document.parent / unquote(url.path)).resolve() if url.path else document
            if not target.exists():
                errors.append(f"{path}:{number}: broken Markdown link {destination!r}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="+", type=Path, help="Markdown documents to check")
    errors = [error for path in parser.parse_args().files for error in broken_links(path)]
    for error in errors:
        print(error, file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
