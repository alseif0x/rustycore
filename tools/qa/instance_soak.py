"""Run the #45 teleport/reconnect and two-client reset scenarios on one Rust process."""

from __future__ import annotations

from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import threading
import time

import death_scenario


SCENARIO = "instance-stability-soak"
GROUP_SECONDS = 2 * 60 * 60
SAMPLE_SECONDS = 15
QUEUE_MESSAGE = "Instance soak command queue totals"
QUEUE_FIELDS = ("command_full", "command_disconnected", "command_stale", "retry_full")


def read_memory(pid: int, starttime: int, proc: Path = Path("/proc")) -> dict:
    process = proc / str(pid)

    def check_process():
        # Field 22 follows the parenthesized comm, which can contain spaces and ')'.
        fields = (process / "stat").read_text().rsplit(") ", 1)[1].split()
        if fields[0] in {"Z", "X"} or int(fields[19]) != starttime:
            raise death_scenario.ScenarioFailure("the captured server process ended or changed")

    check_process()
    values = {}
    for line in (process / "status").read_text().splitlines():
        name, _, value = line.partition(":")
        if name in {"VmRSS", "VmHWM"}:
            count, unit = value.split()
            if unit != "kB" or not count.isdecimal():
                raise death_scenario.ScenarioFailure("unrecognized procfs memory units")
            values[name] = int(count)
    check_process()
    return {"rss_kib": values["VmRSS"], "peak_rss_kib": values["VmHWM"]}


def queue_sample(pid, starttime):
    # A journal entry older than this call could miss the last in-flight reset or disconnect.
    since = time.monotonic_ns() // 1000
    deadline = time.monotonic() + 75
    while time.monotonic() < deadline:
        read_memory(pid, starttime)
        result = subprocess.run(
            ["journalctl", "--boot=0", f"_PID={pid}", "--grep", QUEUE_MESSAGE,
             "--lines=1", "--reverse", "--no-pager", "--quiet",
             "--output=json", "--output-fields=MESSAGE"],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode not in {0, 1}:
            raise death_scenario.ScenarioFailure("journal queue observation failed")
        if result.stdout.strip():
            entry = json.loads(result.stdout)
            stamp = int(entry["__MONOTONIC_TIMESTAMP"])
            if stamp >= since:
                message = re.sub(r"\x1b\[[0-9;]*m", "", entry["MESSAGE"])
                fields = dict(re.findall(r"\b([a-z_]+)=([0-9]+)\b", message))
                sample = {name: int(fields[name]) for name in QUEUE_FIELDS}
                return {**sample, "monotonic_us": stamp}
        time.sleep(2)
    raise death_scenario.ScenarioFailure("no fresh queue counters from the captured server")


@contextmanager
def memory_samples(output: Path, pid: int, starttime: int):
    stop = threading.Event()
    errors = []
    started = time.monotonic()
    with (output / "memory.jsonl").open("x", encoding="utf-8") as stream:
        def sample():
            row = {"elapsed_s": round(time.monotonic() - started, 3),
                   "monotonic_us": time.monotonic_ns() // 1000, **read_memory(pid, starttime)}
            stream.write(json.dumps(row) + "\n")
            stream.flush()

        def poll():
            try:
                while not stop.wait(SAMPLE_SECONDS):
                    sample()
            except Exception as error:
                errors.append(error)

        sample()
        thread = threading.Thread(target=poll, name="soak-memory")
        thread.start()
        try:
            def check():
                if errors:
                    raise death_scenario.ScenarioFailure("server memory sampling stopped") from errors[0]

            yield check
        finally:
            stop.set()
            thread.join()
        check()
        sample()


def run_workload(identity, journal, realm_id, db, run_case):
    output = identity["capture"]
    started = time.monotonic()
    completed = []
    stage = "startup"

    def run_one(name, label):
        nonlocal stage
        stage = label
        directory = output / label
        directory.mkdir(mode=0o700)
        before = time.monotonic()
        result = run_case({**identity, "scenario": name, "capture": directory},
                          journal, realm_id, db)
        if result.get("scenario") != name or result.get("scenario_observation") != "passed":
            raise death_scenario.ScenarioFailure("incomplete instance workload")
        report = directory / "scenario-report.json"
        death_scenario._write_private(report, result)
        completed.append({"scenario": name, "directory": label,
                          "start_monotonic_us": round(before * 1_000_000),
                          "elapsed_s": round(time.monotonic() - before, 3),
                          "report_sha256": death_scenario._sha256(report)})

    def interrupted(signum, frame):
        raise InterruptedError("soak interrupted")

    pid = int(os.environ["RUSTYCORE_CAPTURE_PID"])
    starttime = int(os.environ["RUSTYCORE_CAPTURE_STARTTIME"])
    previous = signal.signal(signal.SIGTERM, interrupted)
    try:
        with memory_samples(output, pid, starttime) as check_samples:
            stage = "queue-before"
            baseline = queue_sample(pid, starttime)
            death_scenario._write_private(output / "queue-before.json", baseline)
            # Hearthstone has a real cooldown. Exercise the teleport once per fixture.
            run_one("instance-solo-reset", "solo")
            check_samples()
            group_start = time.monotonic()
            cycles = 0
            while cycles < 2 or time.monotonic() - group_start < GROUP_SECONDS:
                run_one("instance-group-reset", f"group-{cycles + 1:03d}")
                cycles += 1
                check_samples()
            group_elapsed = time.monotonic() - group_start
            stage = "queue-after"
            end = queue_sample(pid, starttime)
            death_scenario._write_private(output / "queue-after.json", end)
            deltas = {name: end[name] - baseline[name] for name in QUEUE_FIELDS}
            if any(delta < 0 for delta in deltas.values()):
                raise death_scenario.ScenarioFailure("process queue counters moved backwards")
    except BaseException as error:
        death_scenario._write_private(output / "soak-failure.json", {
            "stage": stage, "completed_cases": len(completed), "type": type(error).__name__,
            "elapsed_s": round(time.monotonic() - started, 3),
        })
        raise
    finally:
        signal.signal(signal.SIGTERM, previous)

    return {
        "scenario": SCENARIO, "epoch": 1, "scenario_observation": "passed",
        "workload": completed, "group_cycles": cycles,
        "group_elapsed_s": round(group_elapsed, 3), "elapsed_s": round(time.monotonic() - started, 3),
        "server_pid": pid, "server_starttime_ticks": starttime,
        "command_queue_deltas": deltas,
        "memory_samples": "memory.jsonl", "parity_assessed": False,
    }
