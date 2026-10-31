#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: offline extraction of pinned native selector operations.
"""Build a bounded numeric oracle, not a native TerrainInfo/runtime harness.

Original files/notices remain in the new external output directory. The wrapper
changes only dependency injection: grid/VMAP heights are input scalars; the
original selection, eps and fuzzyGe bodies are retained verbatim. No client data,
network, phase resolution, GridMap or VMapManager execution occurs here.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

PIN = "02245dcd245e7433e524577656177723d3e4992e"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def function(text, signature):
    start = text.index(signature)
    body = text.index("{", start)
    depth = 1
    end = body + 1
    while depth:
        depth += (text[end] == "{") - (text[end] == "}")
        end += 1
    return text[start:end]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.absolute()
    if output.exists() or any((p / ".git").exists() for p in output.parents):
        parser.error("output must be new and outside Git")
    started = time.monotonic()
    paths = ["src/server/game/Maps/TerrainMgr.cpp", "dep/g3dlite/include/G3D/g3dmath.h",
             "src/common/Collision/Management/VMapManager.h", "src/server/game/Grids/GridDefines.h",
             "src/server/game/Miscellaneous/SharedDefines.h"]
    # Partial clones must fail on missing blobs instead of fetching implicitly.
    git_env = {**os.environ, "GIT_NO_LAZY_FETCH": "1"}
    sources = {p: subprocess.check_output(["git", "-C", str(args.reference), "show", f"{PIN}:{p}"],
                                        env=git_env) for p in paths}
    decoded = {p: b.decode() for p, b in sources.items()}
    terrain, math = decoded[paths[0]], decoded[paths[1]]
    original = function(terrain, "float TerrainInfo::GetStaticHeight(")
    tail = original[original.index("    // mapHeight set"):]
    gate_start = original.index("    if (G3D::fuzzyGe")
    gate = original[gate_start:original.index("\n\n", gate_start)]
    constants = []
    for text, name in [(math, "fuzzyEpsilon64"), (decoded[paths[2]], "VMAP_INVALID_HEIGHT_VALUE"),
                       (decoded[paths[3]], "INVALID_HEIGHT"), (decoded[paths[4]], "GROUND_HEIGHT_TOLERANCE")]:
        match = re.search(r"^\s*#define\s+" + name + r"\s+(\S+)", text, re.M)
        if match is None and name == "GROUND_HEIGHT_TOLERANCE":
            match = re.search(r"float const GROUND_HEIGHT_TOLERANCE = ([^;]+);", text)
        if match is None:
            raise RuntimeError(f"pinned constant not found: {name}")
        constants.append(f"#define {name} {match[1]}")
    wrapper = """// Modification notice 2026-10-11: scalar injection wrapper.
// Derived operations from TrinityCore/G3D; full original notices in sources/.
#include <cmath>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <limits>
""" + "\n".join(constants) + "\nnamespace G3D {\nusing std::abs;\ninline double inf() { return std::numeric_limits<double>::infinity(); }\n"
    wrapper += function(math, "inline double eps(double a, double b)") + "\n"
    wrapper += function(math, "inline bool fuzzyGe(double a, double b)") + "\n}\n"
    wrapper += "float select_height(float gridHeight, float vmapHeight, float z) {\n    float mapHeight = VMAP_INVALID_HEIGHT_VALUE;\n" + gate + "\n" + tail
    wrapper += """
int main() {
    static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);
    std::uint32_t input[3];
    while (std::cin.read(reinterpret_cast<char*>(input), sizeof(input))) {
        float p[3]; std::memcpy(p, input, sizeof(p));
        for (float v : p) if (!std::isfinite(v)) return 4;
        float result = select_height(p[0], p[1], p[2]);
        std::uint32_t bits; std::memcpy(&bits, &result, 4);
        std::cout.write(reinterpret_cast<char*>(&bits), 4);
    }
    return std::cin.gcount() == 0 && std::cout.good() ? 0 : 4;
}
"""
    output.mkdir(parents=True)
    for path, data in sources.items():
        dest = output / "sources" / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
        dest.chmod(0o400)
    source = output / "oracle.cpp"
    source.write_text(wrapper)
    binary = output / "height-oracle"
    command = ["c++", "-std=c++17", "-O2", "-fno-fast-math", "-ffp-contract=off", str(source), "-o", str(binary)]
    result = subprocess.run(command, capture_output=True)
    (output / "build.log").write_bytes(result.stdout + result.stderr)
    report = {"schema": "rustycore.forever_height_oracle.v1", "pin": PIN,
              "source_sha256": {p: sha(b) for p, b in sources.items()}, "wrapper_sha256": sha(source.read_bytes()),
              "driver_sha256": sha(Path(__file__).read_bytes()), "command": command, "exit_code": result.returncode,
              "elapsed_seconds": time.monotonic() - started, "whole_terrain_runtime": False,
              "binary_sha256": sha(binary.read_bytes()) if result.returncode == 0 else None}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"report": str(output / "report.json"), "exit_code": result.returncode}))
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
