#!/usr/bin/env python3
"""Token comparison of moved function bodies (#1363, #1404 review).

A move-only slice moves `WorldSession` methods into a domain crate as free
functions over a host trait, changing only the access path (`self` becomes
`host`, a field becomes a host method, ...). This tool shows that nothing else
changed: it extracts each moved function from the base revision, applies the
slice's documented rewrite rules, extracts the same function from the working
tree, and compares the two token streams.

    move_token_compare.py --base REV --pairs pairs.tsv --rules rules.json
    move_token_compare.py self-test

`pairs.tsv` has one moved function per line: `base_path name head_path`
(paths relative to the repository root). `rules.json` is
`{"moved": [names], "rules": [[pattern, replacement], ...]}`: Python regular
expressions applied in order to the base body; a rule containing `{name}` is
expanded once per moved name (the name is regex-escaped in the pattern).
Rules apply only to code, never inside string literals or comments.
Tokens keep comments (whitespace-normalised) and string literals verbatim,
and a trailing comma before a closing delimiter is dropped (rustfmt). The
output lists the functions that are identical and, for the others, the token
difference, so every remaining change can be reviewed by name. It exits 1 if
a function cannot be found.
"""
from __future__ import annotations

import argparse
import difflib
import json
import re
import subprocess
import sys

TOKEN = re.compile(
    r'r#*"(?:.|\n)*?"#*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\'|//[^\n]*|/\*(?:.|\n)*?\*/'
    r"|[A-Za-z_][A-Za-z0-9_]*|\d[\w.]*|::|->|=>|==|!=|<=|>=|&&|\|\||\.\.=?|\S"
)


def tokens(text):
    out = []
    for match in TOKEN.finditer(text):
        tok = match.group(0)
        out.append(" ".join(tok.split()) if tok.startswith(("//", "/*")) else tok)
    return [t for i, t in enumerate(out) if not (t == "," and i + 1 < len(out) and out[i + 1] in ")]}")]


def extract(src, name):
    """The function `name` with its doc comments and attributes, or None."""
    found = re.search(r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+" + re.escape(name) + r"\b", src)
    if not found:
        return None
    lines = src[: found.start()].split("\n")
    j = len(lines) - 2
    while j >= 0 and lines[j].strip().startswith(("///", "#[")):
        j -= 1
    start = len("\n".join(lines[: j + 1])) + 1 if j >= 0 else 0
    depth = 0
    for match in TOKEN.finditer(src, src.index("{", found.end())):
        if match.group(0) == "{":
            depth += 1
        elif match.group(0) == "}":
            depth -= 1
            if depth == 0:
                return src[start : match.end()]
    return None


LITERAL = re.compile(r'r#*"(?:.|\n)*?"#*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\'|//[^\n]*|/\*(?:.|\n)*?\*/')


def _rewrite_code(text, spec):
    for pattern, replacement in spec.get("rules", []):
        if "{name}" in pattern:
            for name in spec.get("moved", []):
                text = re.sub(pattern.replace("{name}", re.escape(name)), replacement.replace("{name}", name), text)
        else:
            text = re.sub(pattern, replacement, text)
    return text


def rewrite(text, spec):
    """Apply the rules to the code between string literals and comments only,
    so a rule can never change a literal: a changed literal stays a difference."""
    out, last = [], 0
    for match in LITERAL.finditer(text):
        out.append(_rewrite_code(text[last : match.start()], spec))
        out.append(match.group(0))
        last = match.end()
    out.append(_rewrite_code(text[last:], spec))
    return "".join(out)


def compare_pair(base_src, head_src, name, spec):
    base_fn, head_fn = extract(base_src, name), extract(head_src, name)
    if base_fn is None or head_fn is None:
        return None
    before, after = tokens(rewrite(base_fn, spec)), tokens(head_fn)
    if before == after:
        return []
    return [d for d in difflib.unified_diff(before, after, lineterm="", n=0) if not d.startswith(("---", "+++"))]


def run(base, pairs, rules):
    spec = json.load(open(rules, encoding="utf-8"))
    identical, differing, missing = [], [], []
    for row in open(pairs, encoding="utf-8"):
        if not row.strip() or row.startswith("#"):
            continue
        base_path, name, head_path = row.split()
        base_src = subprocess.check_output(["git", "show", f"{base}:{base_path}"], text=True)
        head_src = open(head_path, encoding="utf-8").read()
        diff = compare_pair(base_src, head_src, name, spec)
        if diff is None:
            missing.append(name)
        elif diff:
            differing.append((name, diff))
        else:
            identical.append(name)
    total = len(identical) + len(differing) + len(missing)
    print(f"identical after the rewrite: {len(identical)} of {total}")
    for name, diff in differing:
        print(f"== {name}\n   " + " | ".join(diff))
    for name in missing:
        print(f"!! not found: {name}")
    return 1 if missing else 0


def self_test():
    base = 'impl S {\n    /// Doc.\n    pub(crate) fn moved(&mut self, x: u32) -> u32 {\n        warn!("self.keep {}", x);\n        self.other(x,)\n    }\n}\n'
    head = '/// Doc.\npub fn moved<H: Host>(host: &mut H, x: u32) -> u32 {\n    warn!("self.keep {}", x);\n    other(host, x)\n}\n'
    spec = {"moved": ["moved", "other"], "rules": [
        [r"pub\(crate\) fn {name}\(&mut self", "pub fn {name}<H: Host>(host: &mut H"],
        [r"\bself\.{name}\(", "{name}(host, "],
        [r"\bself\.", "host."],
    ]}
    assert compare_pair(base, head, "moved", spec) == [], "a mechanical move must compare identical"
    changed = head.replace("x)\n}", "x + 1)\n}")
    assert compare_pair(base, changed, "moved", spec), "a changed body must differ"
    renamed = head.replace('"self.keep {}"', '"host.keep {}"')
    assert compare_pair(base, renamed, "moved", spec), "a string literal must not be rewritten"
    assert compare_pair(base, head, "absent", spec) is None
    print("move_token_compare self-test: PASS")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", nargs="?", choices=["self-test"])
    parser.add_argument("--base")
    parser.add_argument("--pairs")
    parser.add_argument("--rules")
    args = parser.parse_args()
    if args.command == "self-test":
        return self_test()
    if not (args.base and args.pairs and args.rules):
        parser.error("--base, --pairs and --rules are required")
    return run(args.base, args.pairs, args.rules)


if __name__ == "__main__":
    sys.exit(main())
