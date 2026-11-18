"""CLI regressions for strict validation manifest input."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path


def test_manifest_input_cli(
    tools: Path,
    environment: dict[str, str],
    directory: Path,
    green_manifest: Path,
    verdict_error: int,
) -> None:
    fixtures = directory / "manifest-input-contract"
    fixtures.mkdir()
    (fixtures / "unreadable.json").mkdir()
    green = json.loads(green_manifest.read_text(encoding="utf-8"))
    green_json = json.dumps(green, separators=(",", ":"))

    nested_marker = '"provenance":{'
    assert nested_marker in green_json
    nested_duplicate = green_json.replace(
        nested_marker, '"provenance":{"head":"duplicate",', 1
    )
    cases = [
        ("invalid-utf8", b"\xff", "not valid UTF-8"),
        (
            "duplicate-top-level-status",
            ('{"status":"failed",' + green_json[1:]).encode(),
            "duplicate object key 'status'",
        ),
        (
            "duplicate-nested-head",
            nested_duplicate.encode(),
            "duplicate object key 'head'",
        ),
        ("invalid-json", b"{not json", "not valid JSON"),
        ("missing", None, "is missing"),
        ("unreadable", None, "is unreadable"),
    ]
    for constant in ("NaN", "Infinity", "-Infinity"):
        cases.append(
            (
                f"constant-{constant}",
                ('{"extra":' + constant + "," + green_json[1:]).encode(),
                f"non-standard JSON constant '{constant}'",
            )
        )

    for name, payload, diagnostic in cases:
        target = fixtures / f"{name}.json"
        if payload is not None:
            target.write_bytes(payload)
        for operation in ("verify", "normalize"):
            result = subprocess.run(
                [str(tools / "validation-v2"), operation, "--manifest", str(target)],
                cwd=tools.parent,
                env=environment,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            assert result.returncode == verdict_error, (operation, name, result)
            assert str(target) in result.stderr, (operation, name, result.stderr)
            assert diagnostic in result.stderr, (operation, name, result.stderr)
            assert result.stdout == "", (operation, name, result.stdout)
            assert "Traceback" not in result.stderr, (operation, name, result.stderr)
