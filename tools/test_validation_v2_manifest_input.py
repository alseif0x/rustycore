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

    def run_manifest_cli(operation: str, target: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(tools / "validation-v2"), operation, "--manifest", str(target)],
            cwd=tools.parent,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

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
            result = run_manifest_cli(operation, target)
            assert result.returncode == verdict_error, (operation, name, result)
            assert str(target) in result.stderr, (operation, name, result.stderr)
            assert diagnostic in result.stderr, (operation, name, result.stderr)
            assert result.stdout == "", (operation, name, result.stdout)
            assert "Traceback" not in result.stderr, (operation, name, result.stderr)

    verdict_cases = (
        (
            "float-schema",
            lambda document: document.update(schema=float(green["schema"])),
            (f"schema is {float(green['schema'])!r}",),
        ),
        ("boolean-root-exit", lambda document: document.update(exit_code=False), ("exit code is False",)),
        ("float-root-exit", lambda document: document.update(exit_code=0.0), ("exit code is 0.0",)),
        ("empty-runner-error", lambda document: document.update(runner_error=""), ("runner error ''",)),
        (
            "string-require-changes",
            lambda document: document["plan"].update(require_changes="true"),
            ("plan.require_changes is 'true'; expected a boolean",),
        ),
        (
            "non-object-metadata",
            lambda document: document["plan"].update(metadata="not an object"),
            ("plan.metadata is not an object",),
        ),
        (
            "failed-metadata",
            lambda document: document["plan"].update(
                metadata={
                    "status": "failed",
                    "exit_code": 1,
                    "failure_kind": "exit",
                    "signal": 9,
                    "timed_out": True,
                    "oom_kills": 1,
                }
            ),
            (
                "plan.metadata is 'failed' [exit]",
                "plan.metadata exit code is 1; expected integer 0",
                "plan.metadata failure kind is 'exit'",
                "plan.metadata signal is 9",
                "plan.metadata timed_out is True",
                "plan.metadata oom_kills is 1; expected 0 or None",
            ),
        ),
    )
    for name, mutate, diagnostics in verdict_cases:
        document = json.loads(green_json)
        mutate(document)
        target = fixtures / f"{name}.json"
        target.write_text(json.dumps(document), encoding="utf-8")
        result = run_manifest_cli("verify", target)
        assert result.returncode == verdict_error, (name, result)
        assert str(target) in result.stderr, (name, result.stderr)
        for diagnostic in diagnostics:
            assert diagnostic in result.stderr, (name, diagnostic, result.stderr)
        assert result.stdout == "", (name, result.stdout)
        assert "Traceback" not in result.stderr, (name, result.stderr)

    compatible_green_cases = (
        (
            "valid-metadata",
            lambda document: document["plan"].update(
                metadata={
                    "status": "passed",
                    "exit_code": 0,
                    "failure_kind": None,
                    "signal": None,
                    "timed_out": False,
                    "oom_kills": 0,
                }
            ),
        ),
        ("metadata-absent", lambda document: document["plan"].pop("metadata", None)),
        ("metadata-null", lambda document: document["plan"].update(metadata=None)),
        (
            "require-changes-false",
            lambda document: document["plan"].update(require_changes=False),
        ),
        (
            "require-changes-true",
            lambda document: document["plan"].update(require_changes=True),
        ),
        (
            "require-changes-absent",
            lambda document: document["plan"].pop("require_changes", None),
        ),
    )
    for name, mutate in compatible_green_cases:
        document = json.loads(green_json)
        mutate(document)
        target = fixtures / f"{name}.json"
        target.write_text(json.dumps(document), encoding="utf-8")
        result = run_manifest_cli("verify", target)
        assert result.returncode == 0, (name, result.stdout, result.stderr)
        assert "verified green" in result.stdout, (name, result.stdout)
        assert result.stderr == "", (name, result.stderr)
