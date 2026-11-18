"""CLI regressions for strict validation manifest input."""

from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path


def test_manifest_input_cli(
    tools: Path,
    environment: dict[str, str],
    directory: Path,
    green_manifest: Path,
    runner,
) -> None:
    verdict_error = runner.VERDICT_ERROR
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

    reused = fixtures / "reused.json"
    reused.write_bytes(green_manifest.read_bytes())
    assert run_manifest_cli("verify", reused).returncode == 0
    validate = [str(tools / "validation-v2"), "quick", "--base", "HEAD", "--require-changes"]
    reused_environment = {**environment, "VALIDATION_V2_MANIFEST": str(reused)}
    rejected = subprocess.run(
        validate, cwd=tools.parent,
        env={**reused_environment, "VALIDATION_V2_CARGO_JOBS": "0"},
        text=True, capture_output=True,
    )
    assert rejected.returncode == runner.USAGE_ERROR, rejected.stderr
    failed = json.loads(reused.read_text())
    assert failed["status"] == "failed" and failed["exit_code"] == runner.USAGE_ERROR
    assert "VALIDATION_V2_CARGO_JOBS" in failed["runner_error"]
    assert run_manifest_cli("verify", reused).returncode == verdict_error

    reused.write_bytes(green_manifest.read_bytes())
    ready = fixtures / "preflight-ready"
    release = fixtures / "preflight-release"
    probe_bin = fixtures / "probe-bin"
    probe_bin.mkdir()
    rustc = probe_bin / "rustc"
    rustc.write_text(
        f"#!{sys.executable}\n"
        "from pathlib import Path\n"
        "import time\n"
        f"Path({str(ready)!r}).touch()\n"
        "deadline = time.monotonic() + 8\n"
        f"while not Path({str(release)!r}).exists() and time.monotonic() < deadline:\n"
        "    time.sleep(0.02)\n"
        "print('rustc 0.0.0 (preflight fixture)')\n"
    )
    rustc.chmod(0o755)
    blocked_environment = {**reused_environment, "PATH": f"{probe_bin}:{environment['PATH']}"}
    with subprocess.Popen(
        validate, cwd=tools.parent, env=blocked_environment, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ) as active:
        try:
            deadline = time.monotonic() + 5
            while not ready.exists() and active.poll() is None and time.monotonic() < deadline:
                time.sleep(0.02)
            assert ready.exists(), "the first validation did not enter preflight"
            pending = reused.read_bytes()
            assert json.loads(pending)["status"] == "running"
            assert run_manifest_cli("verify", reused).returncode == verdict_error
            contender = subprocess.run(
                validate, cwd=tools.parent, env=reused_environment,
                text=True, capture_output=True, timeout=5,
            )
            assert contender.returncode == runner.LOCKED_ERROR, contender.stderr
            assert reused.read_bytes() == pending
        finally:
            release.touch()
        _, stderr = active.communicate(timeout=5)
        assert active.returncode == runner.USAGE_ERROR, stderr
    failed = json.loads(reused.read_text())
    assert failed["status"] == "failed" and failed["exit_code"] == runner.USAGE_ERROR
    assert "differs from pinned Rust" in failed["runner_error"]
    assert run_manifest_cli("verify", reused).returncode == verdict_error
    recovered = subprocess.run(
        validate, cwd=tools.parent, env=reused_environment,
        text=True, capture_output=True, timeout=5,
    )
    assert recovered.returncode == 0, recovered.stderr
    assert run_manifest_cli("verify", reused).returncode == 0
