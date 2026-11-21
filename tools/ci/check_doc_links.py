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
    """Yield source lines outside fenced and inline code blocks."""
    fence: tuple[str, int] | None = None
    visible_lines = []
    for line in text.splitlines():
        if fence:
            closing = CLOSE_FENCE.match(line)
            if closing and closing.group(1)[0] == fence[0] and len(closing.group(1)) >= fence[1]:
                fence = None
            visible_lines.append("")
            continue
        opening = OPEN_FENCE.match(line)
        if opening:
            marker = opening.group(1)
            fence = (marker[0], len(marker))
            visible_lines.append("")
            continue
        visible_lines.append(line)

    masked = mask_inline_code("\n".join(visible_lines))
    yield from enumerate(masked.splitlines(), start=1)


def mask_inline_code(text: str) -> str:
    """Replace balanced Markdown code spans with spaces, preserving newlines."""
    characters = list(text)
    cursor = 0
    while cursor < len(text):
        if text[cursor] != "`":
            cursor += 1
            continue

        opening_end = cursor + 1
        while opening_end < len(text) and text[opening_end] == "`":
            opening_end += 1
        slash_cursor = cursor - 1
        while slash_cursor >= 0 and text[slash_cursor] == "\\":
            slash_cursor -= 1
        if (cursor - slash_cursor - 1) % 2:
            cursor = opening_end
            continue

        width = opening_end - cursor
        closing = opening_end
        closing_end = None
        while closing < len(text):
            if text[closing] != "`":
                closing += 1
                continue
            run_end = closing + 1
            while run_end < len(text) and text[run_end] == "`":
                run_end += 1
            if run_end - closing == width:
                closing_end = run_end
                break
            closing = run_end

        if closing_end is None:
            cursor = opening_end
            continue
        for index in range(cursor, closing_end):
            if characters[index] not in "\r\n":
                characters[index] = " "
        cursor = closing_end
    return "".join(characters)


def target_exists(document: Path, path: str) -> bool:
    """Resolve ordinary files and extensionless VitePress Markdown routes."""
    target = (document.parent / unquote(path)).resolve() if path else document
    if target.exists():
        return True
    if path and not target.suffix:
        return (target.with_suffix(".md").is_file()
                or (target / "index.md").is_file())
    return False


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
            if not target_exists(document, url.path):
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
