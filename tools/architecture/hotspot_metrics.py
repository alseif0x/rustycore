"""Existing Rust physical/logical size metrics and their regression checks."""
from __future__ import annotations

import json
import pathlib
import re
import subprocess
from typing import Any

from architecture_common import ArchitectureError, REPO_ROOT


RUST_WHITESPACE = b" \t\r\n"
RUST_IDENTIFIER_BYTES = frozenset(
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"
)
RUST_LITERAL_OR_COMMENT_TRANSLATION = bytes(
    byte if byte in (ord("\n"), ord("\r")) else ord(" ")
    for byte in range(256)
)
EXACT_CFG_TEST_ATTRIBUTE = re.compile(
    rb"\s*cfg\s*\(\s*test\s*\)\s*\Z"
)


def blank_rust_noncode(masked: bytearray, start: int, end: int) -> None:
    """Blank a comment/literal while preserving offsets and line boundaries."""
    masked[start:end] = masked[start:end].translate(
        RUST_LITERAL_OR_COMMENT_TRANSLATION
    )


def rust_raw_string_end(source: bytes, start: int) -> int | None:
    """Return the end of a raw string beginning at start, if one begins there."""
    if start > 0 and source[start - 1] in RUST_IDENTIFIER_BYTES:
        return None
    prefix_length = 0
    for prefix in (b"br", b"cr", b"r"):
        if source.startswith(prefix, start):
            prefix_length = len(prefix)
            break
    if not prefix_length:
        return None

    cursor = start + prefix_length
    while cursor < len(source) and source[cursor] == ord("#"):
        cursor += 1
    hashes = source[start + prefix_length : cursor]
    if cursor >= len(source) or source[cursor] != ord('"'):
        return None

    terminator = b'"' + hashes
    closing = source.find(terminator, cursor + 1)
    return len(source) if closing < 0 else closing + len(terminator)


def rust_char_literal_end(source: bytes, start: int) -> int | None:
    """Distinguish a Rust character literal from a lifetime or loop label."""
    cursor = start + 1
    if cursor >= len(source) or source[cursor] in (ord("\n"), ord("\r")):
        return None
    if source[cursor] == ord("\\"):
        cursor += 1
        if cursor >= len(source):
            return None
        if source[cursor] == ord("x"):
            cursor += 3
        elif source[cursor] == ord("u") and cursor + 1 < len(source):
            if source[cursor + 1] != ord("{"):
                return None
            closing = source.find(b"}", cursor + 2)
            if closing < 0:
                return None
            cursor = closing + 1
        else:
            cursor += 1
    else:
        first = source[cursor]
        if first < 0x80:
            cursor += 1
        elif first & 0xE0 == 0xC0:
            cursor += 2
        elif first & 0xF0 == 0xE0:
            cursor += 3
        elif first & 0xF8 == 0xF0:
            cursor += 4
        else:
            return None
    if cursor < len(source) and source[cursor] == ord("'"):
        return cursor + 1
    return None


def mask_rust_noncode(source: str) -> bytes:
    """Lexically blank Rust comments and literals without changing byte offsets."""
    original = source.encode("utf-8")
    masked = bytearray(original)
    cursor = 0
    while cursor < len(original):
        if original.startswith(b"//", cursor):
            end = original.find(b"\n", cursor + 2)
            end = len(original) if end < 0 else end
            blank_rust_noncode(masked, cursor, end)
            cursor = end
            continue
        if original.startswith(b"/*", cursor):
            depth = 1
            end = cursor + 2
            while end < len(original) and depth:
                if original.startswith(b"/*", end):
                    depth += 1
                    end += 2
                elif original.startswith(b"*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            blank_rust_noncode(masked, cursor, end)
            cursor = end
            continue
        raw_end = None
        if original[cursor] in (ord("b"), ord("c"), ord("r")):
            raw_end = rust_raw_string_end(original, cursor)
        if raw_end is not None:
            blank_rust_noncode(masked, cursor, raw_end)
            cursor = raw_end
            continue
        if original[cursor] == ord('"'):
            end = cursor + 1
            while end < len(original):
                if original[end] == ord("\\"):
                    end = min(end + 2, len(original))
                elif original[end] == ord('"'):
                    end += 1
                    break
                else:
                    end += 1
            blank_rust_noncode(masked, cursor, end)
            cursor = end
            continue
        if original[cursor] == ord("'"):
            char_end = rust_char_literal_end(original, cursor)
            if char_end is not None:
                blank_rust_noncode(masked, cursor, char_end)
                cursor = char_end
                continue
        cursor += 1
    return bytes(masked)


def skip_rust_whitespace(source: bytes, cursor: int) -> int:
    while cursor < len(source) and source[cursor] in RUST_WHITESPACE:
        cursor += 1
    return cursor


def matching_rust_delimiter(
    source: bytes, start: int, opening: int, closing: int
) -> int | None:
    depth = 0
    for cursor in range(start, len(source)):
        if source[cursor] == opening:
            depth += 1
        elif source[cursor] == closing:
            depth -= 1
            if depth == 0:
                return cursor
    return None


def rust_identifier(source: bytes, cursor: int) -> tuple[str | None, int]:
    if cursor >= len(source) or source[cursor] not in RUST_IDENTIFIER_BYTES:
        return None, cursor
    end = cursor + 1
    while end < len(source) and source[end] in RUST_IDENTIFIER_BYTES:
        end += 1
    return source[cursor:end].decode("ascii"), end


def skip_outer_rust_attributes(source: bytes, cursor: int) -> int:
    """Skip attributes following the cfg(test) attribute on the same item."""
    while True:
        candidate = skip_rust_whitespace(source, cursor)
        if candidate >= len(source) or source[candidate] != ord("#"):
            return candidate
        bracket = skip_rust_whitespace(source, candidate + 1)
        if bracket >= len(source) or source[bracket] != ord("["):
            return candidate
        closing = matching_rust_delimiter(
            source, bracket, ord("["), ord("]")
        )
        if closing is None:
            raise ArchitectureError("unterminated Rust attribute in hotspot source")
        cursor = closing + 1


def cfg_test_item_kind(source: bytes, cursor: int) -> tuple[str, int]:
    """Classify the top-level Rust item attached to an exact cfg(test)."""
    cursor = skip_outer_rust_attributes(source, cursor)
    token, end = rust_identifier(source, cursor)
    if token == "pub":
        cursor = skip_rust_whitespace(source, end)
        if cursor < len(source) and source[cursor] == ord("("):
            closing = matching_rust_delimiter(
                source, cursor, ord("("), ord(")")
            )
            if closing is None:
                raise ArchitectureError("unterminated Rust visibility in hotspot source")
            cursor = closing + 1
        cursor = skip_rust_whitespace(source, cursor)

    modifiers: set[str] = set()
    while True:
        token, end = rust_identifier(source, cursor)
        if token not in {"async", "unsafe", "default", "auto", "extern"}:
            break
        modifiers.add(token)
        cursor = skip_rust_whitespace(source, end)

    token, end = rust_identifier(source, cursor)
    if token == "const":
        lookahead = skip_rust_whitespace(source, end)
        while True:
            following, following_end = rust_identifier(source, lookahead)
            if following not in {"async", "unsafe", "extern"}:
                break
            lookahead = skip_rust_whitespace(source, following_end)
        if following == "fn":
            return "body-or-semicolon", following_end
        return "semicolon", end
    if token in {"fn", "mod", "struct", "enum", "union", "trait", "impl"}:
        return "body-or-semicolon", end
    if token in {"use", "type", "static"}:
        return "semicolon", end
    if token in {"macro", "macro_rules"}:
        return "body-or-semicolon", end
    if "extern" in modifiers:
        if token in {"crate", "static", "type"}:
            return "semicolon", end
        if token == "fn" or token is None:
            return "body-or-semicolon", end
    raise ArchitectureError(
        "unsupported top-level Rust item following exact #[cfg(test)]"
    )


def cfg_test_item_end(source: bytes, cursor: int, kind: str) -> int:
    round_depth = 0
    square_depth = 0
    curly_depth = 0
    angle_depth = 0
    while cursor < len(source):
        byte = source[cursor]
        if byte == ord("("):
            round_depth += 1
        elif byte == ord(")"):
            round_depth -= 1
        elif byte == ord("["):
            square_depth += 1
        elif byte == ord("]"):
            square_depth -= 1
        elif (
            kind == "body-or-semicolon"
            and byte == ord("<")
            and not round_depth
            and not square_depth
        ):
            angle_depth += 1
        elif (
            kind == "body-or-semicolon"
            and byte == ord(">")
            and angle_depth
            and (cursor == 0 or source[cursor - 1] != ord("-"))
        ):
            angle_depth -= 1
        elif byte == ord("{"):
            if (
                kind == "body-or-semicolon"
                and not round_depth
                and not square_depth
                and not curly_depth
                and not angle_depth
            ):
                closing = matching_rust_delimiter(
                    source, cursor, ord("{"), ord("}")
                )
                if closing is None:
                    raise ArchitectureError(
                        "unterminated cfg(test) Rust item body in hotspot source"
                    )
                return closing + 1
            curly_depth += 1
        elif byte == ord("}"):
            curly_depth -= 1
        elif (
            byte == ord(";")
            and not round_depth
            and not square_depth
            and not curly_depth
            and not angle_depth
        ):
            return cursor + 1
        cursor += 1
    raise ArchitectureError("unterminated cfg(test) Rust item in hotspot source")


def top_level_cfg_test_item_spans(source: str) -> list[tuple[int, int]]:
    """Return exact byte spans of test-only top-level items in a Rust file."""
    masked = mask_rust_noncode(source)
    spans: list[tuple[int, int]] = []
    brace_depth = 0
    cursor = 0
    while cursor < len(masked):
        byte = masked[cursor]
        if byte == ord("{"):
            brace_depth += 1
        elif byte == ord("}"):
            brace_depth -= 1
        elif byte == ord("#") and brace_depth == 0:
            bracket = skip_rust_whitespace(masked, cursor + 1)
            if bracket < len(masked) and masked[bracket] == ord("["):
                closing = matching_rust_delimiter(
                    masked, bracket, ord("["), ord("]")
                )
                if closing is None:
                    raise ArchitectureError(
                        "unterminated top-level Rust attribute in hotspot source"
                    )
                if EXACT_CFG_TEST_ATTRIBUTE.fullmatch(
                    masked[bracket + 1 : closing]
                ):
                    item_start = closing + 1
                    kind, search_start = cfg_test_item_kind(masked, item_start)
                    item_end = cfg_test_item_end(masked, search_start, kind)
                    spans.append((cursor, item_end))
        cursor += 1
    return spans


def top_level_cfg_test_line_indexes(source: str) -> set[int]:
    """Return physical line indexes touched by top-level cfg(test) items."""
    test_line_indexes: set[int] = set()
    encoded = source.encode("utf-8")
    for start, end in top_level_cfg_test_item_spans(source):
        first_line = encoded.count(b"\n", 0, start)
        last_line = encoded.count(b"\n", 0, max(start, end - 1))
        test_line_indexes.update(range(first_line, last_line + 1))
    return test_line_indexes


def file_is_entirely_cfg_test(source: str) -> bool:
    """Whether an inner `#![cfg(test)]` makes the whole file test-only.

    A test module extracted to its own file carries its `#[cfg(test)]` on the
    `mod` declaration in the parent, not inside the file, so the extent scan
    below finds nothing and counts every line as production. That inverts the
    metric: moving 94k lines of tests out of a hotspot would report the hotspot
    unchanged and a new 94k production file appearing.

    Only inner attributes and inner doc comments may precede the first item, so
    reading until the first other line is enough to decide.
    """
    for line in source.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        if stripped == "#![cfg(test)]":
            return True
        if stripped.startswith("#!["):
            continue
        return False
    return False


def hotspot_line_counts(source: str) -> tuple[int, int, int]:
    """Partition physical lines by exact top-level cfg(test) item extents."""
    lines = source.splitlines()
    if file_is_entirely_cfg_test(source):
        return len(lines), 0, len(lines)
    test_lines = len(top_level_cfg_test_line_indexes(source))
    production_lines = len(lines) - test_lines
    return len(lines), production_lines, test_lines


def run_path_module_scanner_self_tests() -> int:
    """Prove the `#[path]` mapping covers the mounts this repository has.

    The mapping comes from `syn` by way of the Rust analyzer, so the thing worth
    checking here is not lexing -- it is that the guard is actually consuming it
    and that every mount it reports resolves to a file whose parent exists. A
    silent empty mapping would put every `#[path]` child back outside its
    parent's ceiling, which is the failure this aggregation exists to prevent.
    """
    mounts = path_module_mounts()
    if not mounts:
        raise ArchitectureError(
            "path module mapping is empty; this repository mounts #[path] children, "
            "so an empty result means the analyzer was not consulted"
        )
    for parent, children in mounts.items():
        if not parent.is_file():
            raise ArchitectureError(f"#[path] parent {parent} does not exist")
        for child, _ in children:
            if not child.is_file():
                raise ArchitectureError(f"#[path] child {child} does not exist")
            if child == parent:
                raise ArchitectureError(f"{parent} cannot mount itself")
    return len(mounts)


def run_hotspot_classifier_self_tests() -> int:
    """Prove cfg(test) extents survive lexical traps and stop at item ends."""
    lines = [
        "pub fn production_before() {}",
        "#[cfg(test)]",
        "mod first {",
        '    const COOKED: &str = "} // not syntax";',
        '    const RAW: &str = r###"} /* not syntax */"###;',
        "    const CLOSE: char = '}' ;",
        "    // } #[cfg(test)]",
        "    /* { outer /* } */ } */",
        "    fn still_inside<'a>(value: &'a str) -> &'a str { value }",
        "}",
        "pub fn production_between() {}",
        "#[cfg(test)]",
        "fn helper() {",
        "    let _ = '{';",
        "}",
        "#[cfg(test)]",
        "use crate::{",
        "    alpha,",
        "    beta,",
        "};",
        "#[cfg(test)]",
        "#[allow(dead_code)]",
        "pub(crate) mod second {",
        '    const NOTE: &str = "}";',
        "}",
        "#[cfg(not(test))]",
        "mod production_not_test {}",
        '#[cfg(any(test, feature = "fixture"))]',
        "fn production_possible() {}",
        'const FAKE: &str = "#[cfg(test)] mod fake { }";',
        "pub fn production_after() {}",
    ]
    source = "\n".join(lines) + "\n"
    expected = (
        set(range(1, 10))
        | set(range(11, 15))
        | set(range(15, 20))
        | set(range(20, 25))
    )
    actual = top_level_cfg_test_line_indexes(source)
    if actual != expected:
        raise ArchitectureError(
            "hotspot classifier adversarial self-test failed: "
            f"expected lines {sorted(expected)}, got {sorted(actual)}"
        )
    total, production, tests = hotspot_line_counts(source)
    expected_counts = (len(lines), len(lines) - len(expected), len(expected))
    if (total, production, tests) != expected_counts:
        raise ArchitectureError(
            "hotspot classifier line partition self-test failed: "
            f"got {(total, production, tests)}"
        )
    return 1


def run_hotspot_view_self_tests(runtime: dict[str, Any]) -> int:
    """Prove physical files and logical private descendants stay independent."""
    physical = {row[3]: row for row in physical_hotspot_rows(limit=None)}
    main_path = "crates/world-server/src/main.rs"
    if main_path not in physical or physical[main_path][0] >= 100:
        raise ArchitectureError(
            "physical hotspot view did not preserve the thin world-server main file"
        )

    runtime_root = REPO_ROOT / "crates/world-server/src/runtime/mod.rs"
    runtime_files = logical_hotspot_files(runtime_root, "module")
    expected_child = REPO_ROOT / "crates/world-server/src/runtime/delivery.rs"
    if expected_child not in runtime_files:
        raise ArchitectureError(
            "logical hotspot view did not include an ordinary adjacent Rust submodule"
        )

    logical = {row[3]: row for row in logical_hotspot_rows(runtime, limit=None)}
    composition_root = "crates/world-server/src/lib.rs"
    if composition_root not in logical:
        raise ArchitectureError("logical hotspot view omitted the composition owner")
    if logical[composition_root][0] <= physical[composition_root][0]:
        raise ArchitectureError(
            "logical composition owner did not include its private descendants"
        )
    return 3


HOTSPOT_ROW_CACHE: dict[tuple[pathlib.Path, str], tuple[int, int, int, str]] = {}


PATH_MODULE_MOUNTS_COMMAND = (
    "cargo",
    "run",
    "--quiet",
    "--release",
    "--locked",
    "--manifest-path",
    "tools/architecture/handler-contract-check/Cargo.toml",
    "--bin",
    "session-ownership-check",
    "--",
    "print-path-modules",
)

PATH_MODULE_MOUNTS_CACHE: dict[str, dict[pathlib.Path, list[tuple[pathlib.Path, bool]]]] = {}


def path_module_mounts() -> dict[pathlib.Path, list[tuple[pathlib.Path, bool]]]:
    """Which files each source mounts with `#[path]`, resolved by `syn`.

    Asked of the Rust analyzer rather than scanned here. Finding these in text
    means reimplementing a Rust lexer -- comments, escapes, raw strings, char
    literals versus lifetimes, macro bodies, trivia between attribute tokens --
    and every gap is a way to move a hotspot ceiling. That list does not end:
    an invoked `macro_rules!` can generate a mount no scanner can see without
    expanding it. One parser in this repository already resolves all of it, so
    this asks that one instead of maintaining a second.
    """
    if "mounts" in PATH_MODULE_MOUNTS_CACHE:
        return PATH_MODULE_MOUNTS_CACHE["mounts"]
    result = subprocess.run(
        PATH_MODULE_MOUNTS_COMMAND,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        raise ArchitectureError(
            "cannot resolve #[path] module mounts: "
            f"{result.stderr.strip() or result.stdout.strip()}"
        )
    try:
        rows = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ArchitectureError(f"#[path] mount output is not JSON: {exc}") from exc
    mounts: dict[pathlib.Path, list[tuple[pathlib.Path, bool]]] = {}
    for row in rows:
        parent = REPO_ROOT / row["parent"]
        mounts.setdefault(parent, []).append(
            (REPO_ROOT / row["child"], bool(row["test_only"]))
        )
    PATH_MODULE_MOUNTS_CACHE["mounts"] = mounts
    return mounts


def path_module_children(path: pathlib.Path, source: str) -> list[tuple[pathlib.Path, bool]]:
    """Files `path` mounts with `#[path]`, and whether each mount is test-only."""
    del source
    return path_module_mounts().get(path, [])


def physical_hotspot_row(path: pathlib.Path) -> tuple[int, int, int, str]:
    """Count one real source file without charging any child module to it."""
    try:
        source = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ArchitectureError(f"cannot read Rust source {path}: {exc}") from exc
    total, production, tests = hotspot_line_counts(source)
    return total, production, tests, path.relative_to(REPO_ROOT).as_posix()


def logical_hotspot_files(path: pathlib.Path, scope: str) -> dict[pathlib.Path, bool]:
    """Resolve the reviewed private descendants of one logical owner.

    A normal multi-file Rust module keeps descendants below the adjacent
    directory (`session.rs` + `session/` or `session/mod.rs`). Composition roots
    may explicitly own their full crate source tree. `#[path]` children are
    followed as well so the logical view remains compatible with transitional
    mounts while the physical view always reports each real file separately.
    """
    if scope not in {"module", "crate"}:
        raise ArchitectureError(
            f"logical hotspot {path} has unsupported logical_scope {scope!r}"
        )
    files = {path: False}
    if scope == "crate":
        candidates = path.parent.glob("**/*.rs")
    else:
        module_dir = path.parent if path.name == "mod.rs" else path.with_suffix("")
        candidates = module_dir.glob("**/*.rs") if module_dir.is_dir() else ()
    for candidate in candidates:
        relative_parts = candidate.relative_to(path.parent).parts
        files[candidate] = (
            candidate.stem == "tests"
            or candidate.stem.endswith("_tests")
            or "tests" in relative_parts
        )

    pending = list(files)
    while pending:
        parent = pending.pop()
        try:
            source = parent.read_text(encoding="utf-8")
        except OSError as exc:
            raise ArchitectureError(f"cannot read Rust source {parent}: {exc}") from exc
        for child, mounted_under_cfg_test in path_module_children(parent, source):
            test_only = files[parent] or mounted_under_cfg_test
            if child not in files or (test_only and not files[child]):
                files[child] = test_only
                pending.append(child)
    return files


def logical_hotspot_row(
    path: pathlib.Path, scope: str = "module"
) -> tuple[int, int, int, str]:
    cache_key = (path, scope)
    cached = HOTSPOT_ROW_CACHE.get(cache_key)
    if cached is not None:
        return cached
    total_lines = 0
    production_lines = 0
    test_lines = 0
    logical_files = logical_hotspot_files(path, scope)
    for source_path, mounted_under_cfg_test in logical_files.items():
        child_total, child_production, child_tests, _ = physical_hotspot_row(source_path)
        total_lines += child_total
        if mounted_under_cfg_test:
            test_lines += child_total
        else:
            production_lines += child_production
            test_lines += child_tests
    row = (
        total_lines,
        production_lines,
        test_lines,
        path.relative_to(REPO_ROOT).as_posix(),
    )
    HOTSPOT_ROW_CACHE[cache_key] = row
    return row


def physical_hotspot_rows(
    limit: int | None = 10,
) -> list[tuple[int, int, int, str]]:
    crates_root = REPO_ROOT / "crates"
    sources = sorted(crates_root.glob("*/src/**/*.rs"))
    rows = [physical_hotspot_row(path) for path in sources]
    rows.sort(key=lambda row: (-row[0], row[3]))
    return rows if limit is None else rows[:limit]


def logical_hotspot_rows(
    runtime: dict[str, Any], limit: int | None = 10
) -> list[tuple[int, int, int, str]]:
    rows = [
        logical_hotspot_row(
            REPO_ROOT / entry["path"], entry.get("logical_scope", "module")
        )
        for entry in runtime["inventories"]["hotspots"]["entries"]
    ]
    rows.sort(key=lambda row: (-row[0], row[3]))
    return rows if limit is None else rows[:limit]


def validate_hotspot_non_growth(
    runtime: dict[str, Any],
    live_rows: list[tuple[int, int, int, str]] | None = None,
) -> int:
    """Reject growth in any audited hotspot production/test/total metric.

    The checked-in ledger values are independent upper bounds, not exact
    snapshots: a hotspot may shrink without editing the baseline. An audited
    path must remain present until its ledger row is deliberately retired, so
    renaming a file cannot silently evade its ceiling. Reporting remains
    broader; only paths explicitly curated in the runtime ledger are ratcheted
    here.
    """
    if live_rows is None:
        live_rows = logical_hotspot_rows(runtime, limit=None)

    live_by_path: dict[str, tuple[int, int, int]] = {}
    for total, production, tests, path in live_rows:
        if (
            type(total) is not int
            or type(production) is not int
            or type(tests) is not int
            or not isinstance(path, str)
            or not path
            or min(total, production, tests) < 0
            or production + tests != total
        ):
            raise ArchitectureError(
                f"live hotspot metrics for {path!r} must be non-negative and add up"
            )
        if path in live_by_path:
            raise ArchitectureError(f"duplicate live hotspot metrics for {path}")
        live_by_path[path] = (production, tests, total)

    entries = runtime["inventories"]["hotspots"]["entries"]
    violations: list[str] = []
    for entry in entries:
        path = entry["path"]
        live = live_by_path.get(path)
        if live is None:
            violations.append(
                f"audited hotspot path {path} is missing; retire or replace "
                "its ledger row explicitly"
            )
            continue
        for index, key in enumerate(
            ("production_lines", "test_lines", "total_lines")
        ):
            baseline_value = entry[key]
            live_value = live[index]
            if live_value > baseline_value:
                violations.append(
                    f"{path} {key} grew from {baseline_value} to {live_value} "
                    f"(+{live_value - baseline_value})"
                )
    if violations:
        raise ArchitectureError(
            "runtime ownership hotspot LOC ratchet failed:\n- "
            + "\n- ".join(violations)
        )
    return len(entries)


def run_hotspot_ratchet_self_tests(runtime: dict[str, Any]) -> tuple[int, int]:
    """Prove independent growth rejection and below-baseline reduction acceptance."""
    baseline_rows = [
        (
            entry["total_lines"],
            entry["production_lines"],
            entry["test_lines"],
            entry["path"],
        )
        for entry in runtime["inventories"]["hotspots"]["entries"]
    ]
    validate_hotspot_non_growth(runtime, baseline_rows)

    total, production, tests, path = baseline_rows[0]
    growth_cases = [
        (
            "production-growth-with-stable-total",
            (total, production + 1, tests - 1, path),
            "production_lines grew",
        ),
        (
            "test-growth-with-stable-total",
            (total, production - 1, tests + 1, path),
            "test_lines grew",
        ),
        (
            "total-growth",
            (total + 1, production + 1, tests, path),
            "total_lines grew",
        ),
    ]
    for name, replacement, expected in growth_cases:
        candidate = [replacement, *baseline_rows[1:]]
        try:
            validate_hotspot_non_growth(runtime, candidate)
        except ArchitectureError as exc:
            if expected not in str(exc):
                raise ArchitectureError(
                    f"hotspot ratchet self-test {name} returned the wrong failure: {exc}"
                ) from exc
        else:
            raise ArchitectureError(
                f"hotspot ratchet self-test {name} was not rejected"
            )

    try:
        validate_hotspot_non_growth(runtime, baseline_rows[1:])
    except ArchitectureError as exc:
        if "audited hotspot path" not in str(exc) or "is missing" not in str(exc):
            raise ArchitectureError(
                f"hotspot ratchet self-test missing-path returned the wrong failure: {exc}"
            ) from exc
    else:
        raise ArchitectureError(
            "hotspot ratchet self-test missing-path was not rejected"
        )

    reduced = [
        (total - 2, production - 1, tests - 1, path),
        *baseline_rows[1:],
    ]
    validate_hotspot_non_growth(runtime, reduced)
    return len(growth_cases) + 1, 1


def print_hotspots(runtime: dict[str, Any], limit: int = 10) -> None:
    print(
        "Physical Rust files (reporting; each real file counted independently):"
    )
    print(f"{'total':>8} {'prod':>8} {'tests':>8}  path")
    for total, production, tests, path in physical_hotspot_rows(limit):
        print(f"{total:8d} {production:8d} {tests:8d}  {path}")
    print("Logical owners (curated roots including reviewed private descendants):")
    print(f"{'total':>8} {'prod':>8} {'tests':>8}  owner root")
    for total, production, tests, path in logical_hotspot_rows(runtime, limit):
        print(f"{total:8d} {production:8d} {tests:8d}  {path}")



