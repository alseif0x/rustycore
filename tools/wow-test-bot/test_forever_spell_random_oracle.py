#!/usr/bin/env python3
"""Parent-exclusive source RNG QA, deferred to completed-delivery acceptance.

Generates dependency/fixture files only in a disposable temporary directory.
No DB, client assets, secrets, network or persistent production RNG reseeding.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "02245dcd245e7433e524577656177723d3e4992e"


def source_file(root: Path, path: str) -> bytes:
    return subprocess.run(["git", "-C", str(root), "show", f"{SOURCE}:{path}"],
                          capture_output=True, check=True).stdout


def section(source: bytes, start: bytes, next_section: bytes) -> bytes:
    position = source.index(start)
    return source[position:source.index(next_section, position)]


def run(args: argparse.Namespace) -> int:
    # Inspect the production input list without importing/running build code.
    helper = (ROOT / "crates/world-server/build/forever_spell_random.rs").read_text()
    files = re.findall(r'\(\s*"((?:dep/SFMT|src/common)/[^"\n]+)"\s*,\s*"([^"\n]+)"\s*,?\s*\)', helper)
    if len(files) != 13 or f'const SOURCE: &str = "{SOURCE}";' not in helper:
        raise ValueError("production source pin/input contract changed")
    random = source_file(args.reference_root, "src/common/Utilities/Random.cpp")
    header = source_file(args.reference_root, "src/common/Utilities/Random.h")
    frand = section(random, b"float frand(float min, float max)\n{", b"\n\nMilliseconds randtime")
    engine = section(header, b"class RandomEngine\n{", b"\n\nstruct PseudoRandomDistributionState")
    namespace = section(random, b"namespace\n{\nconstexpr RandomEngine engine;", b"\n\nint32 irand")
    rand32 = section(random, b"uint32 rand32()\n{", b"\n\nfloat rand_norm")
    urand = section(random, b"uint32 urand(uint32 min, uint32 max)\n{", b"\n\nuint32 urandms")
    weighted = section(random, b"uint32 urandweighted(size_t count, double const* chances)\n{", b"\n\nnamespace")
    prefix = random[:random.index(b"#include")]
    with tempfile.TemporaryDirectory(prefix="forever-spell-random-") as staging:
        root = Path(staging)
        for path, name in files:
            (root / name).write_bytes(source_file(args.reference_root, path))
        (root / "SourceRandom.cpp").write_bytes(prefix + b'''
#include "Random.h"
#include "SFMTRand.h"
#include <memory>
#include <random>
#include <cassert>
#define ASSERT(condition) assert(condition)
''' + namespace + b"\n" + frand + b"\n" + rand32 + b"\n" + urand + b"\n" + weighted + b"\n")
        (root / "SourceSeededDraw.hpp").write_bytes(prefix +
            b"#define ASSERT(condition) assert(condition)\n" + engine +
            b"\nconstexpr RandomEngine engine;\n" + frand + b"\n" + urand + b"\n" + weighted + b"\n#undef ASSERT\n")
        directory = ROOT / "crates/world-server/src/forever/spell_random"
        executable = root / "source-random-oracle"
        sfmt_object = root / "sfmt.o"
        result = subprocess.run([args.c_compiler, "-std=c11", "-O2", "-msse2",
            "-DSFMT_MEXP=19937", "-DHAVE_SSE2", f"-I{root}",
            "-c", str(root / "SFMT.c"), "-o", str(sfmt_object)],
            capture_output=True, text=True, timeout=120)
        if result.returncode != 0:
            sys.stderr.write(result.stderr)
            return result.returncode
        result = subprocess.run([args.compiler, "-std=c++20", "-O2", "-ffp-contract=off",
            "-msse2", "-DSFMT_MEXP=19937", "-DHAVE_SSE2", f"-I{root}",
            str(sfmt_object), str(root / "SFMTRand.cpp"), str(root / "SourceRandom.cpp"),
            str(directory / "bridge.cpp"), str(directory / "oracle.cpp"), "-o", str(executable)],
            capture_output=True, text=True, timeout=120)
        if result.returncode != 0:
            sys.stderr.write(result.stderr)  # public-source diagnostics only
            return result.returncode
        result = subprocess.run([str(executable)], capture_output=True, text=True, timeout=30)
        if result.returncode == 0 and result.stdout.startswith("PASS source-random-oracle "):
            print(result.stdout.strip())
            return 0
        print(f"FAIL source-random-oracle exit={result.returncode}", file=sys.stderr)
        return result.returncode or 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-root", type=Path, default=ROOT / "target/forever-cpp-reference")
    parser.add_argument("--compiler", default="g++")
    parser.add_argument("--c-compiler", default="gcc")
    args = parser.parse_args()
    try:
        return run(args)
    except (OSError, ValueError, subprocess.SubprocessError):
        print("FAIL source-random QA prerequisites or execution", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
