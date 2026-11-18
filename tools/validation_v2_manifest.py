"""Pure manifest validation and deterministic comparison helpers."""

from __future__ import annotations


MANIFEST_SCHEMA = 4

# The determinism contract. Every key a manifest may carry is named here, so a
# field added later fails the comparison instead of escaping a denylist.
DROPPED_FIELDS = frozenset(
    {"run_id", "started_at", "ended_at", "duration_seconds", "peak_child_rss_kib"}
)
DROPPED_COMMAND_FIELDS = frozenset({"started_at", "ended_at", "duration_seconds"})
NORMALISED_FIELDS = {
    ("provenance", "repository_root"),
    ("provenance", "kernel"),
    ("resources", "memory_limit_kib"),
    ("locks", "repository"),
    ("locks", "heavy"),
}
COMPARED_SHAPE: dict[str, frozenset[str]] = {
    "": frozenset(
        {
            "schema", "runner", "shadow_only", "profile", "status", "exit_code",
            "runner_signal", "runner_error", "provenance", "resources", "locks",
            "plan", "commands",
        }
    ),
    "provenance": frozenset({"head", "dirty", "rust", "repository_root", "kernel"}),
    "provenance.rust": frozenset({"pinned", "active"}),
    "resources": frozenset({"cargo_jobs", "command_timeout_seconds", "memory_limit_kib"}),
    "locks": frozenset({"repository", "heavy"}),
    "plan": frozenset(
        {
            "base_input", "base_commit", "changed_paths", "path_classes", "workspace",
            "metadata", "optional_skips", "planned_steps", "planned_commands",
            "failure_policy", "require_changes",
        }
    ),
    "command": frozenset(
        {
            "argv", "section", "exit_code", "signal", "timed_out", "oom_kills",
            "child_signal_reports", "failure_kind", "status",
        }
    ),
}
PLACEHOLDER = "<normalised>"


def normalise_command(command: object, where: str) -> dict[str, object]:
    if not isinstance(command, dict):
        raise ValueError(f"{where} is not an object")
    unknown = sorted(set(command) - COMPARED_SHAPE["command"] - DROPPED_COMMAND_FIELDS)
    if unknown:
        raise ValueError(f"{where} carries unknown field(s): {', '.join(unknown)}")
    return {
        key: value for key, value in command.items() if key not in DROPPED_COMMAND_FIELDS
    }


def normalise_manifest(document: object) -> dict[str, object]:
    """The exact comparison form: volatile dropped, host-shaped placeheld, rest exact."""
    if not isinstance(document, dict):
        raise ValueError("manifest is not an object")
    unknown = sorted(set(document) - COMPARED_SHAPE[""] - DROPPED_FIELDS)
    if unknown:
        raise ValueError(f"manifest carries unknown field(s): {', '.join(unknown)}")
    result: dict[str, object] = {}
    for key, value in document.items():
        if key in DROPPED_FIELDS:
            continue
        if key in {"provenance", "resources", "locks"}:
            if not isinstance(value, dict):
                raise ValueError(f"{key} is not an object")
            section_unknown = sorted(set(value) - COMPARED_SHAPE[key])
            if section_unknown:
                raise ValueError(
                    f"{key} carries unknown field(s): {', '.join(section_unknown)}"
                )
            section = {}
            for name, item in value.items():
                if (key, name) in NORMALISED_FIELDS:
                    section[name] = None if item is None else PLACEHOLDER
                elif key == "provenance" and name == "rust":
                    if not isinstance(item, dict) or set(item) != COMPARED_SHAPE[
                        "provenance.rust"
                    ]:
                        raise ValueError("provenance.rust does not match its shape")
                    section[name] = item
                else:
                    section[name] = item
            result[key] = section
        elif key == "plan":
            if value is None:
                result[key] = None
                continue
            if not isinstance(value, dict):
                raise ValueError("plan is not an object")
            plan_unknown = sorted(set(value) - COMPARED_SHAPE["plan"])
            if plan_unknown:
                raise ValueError(f"plan carries unknown field(s): {', '.join(plan_unknown)}")
            plan = dict(value)
            if plan.get("metadata") is not None:
                plan["metadata"] = normalise_command(plan["metadata"], "plan.metadata")
            result[key] = plan
        elif key == "commands":
            if not isinstance(value, list):
                raise ValueError("commands is not a list")
            result[key] = [
                normalise_command(command, f"command {index}")
                for index, command in enumerate(value)
            ]
        else:
            result[key] = value
    return result


def _outcome_problems(
    outcome: object, label: str, *, include_section: bool = False
) -> list[str]:
    """Validate one recorded command-like outcome as a successful result."""
    if not isinstance(outcome, dict):
        return [f"{label} is not an object"]

    problems: list[str] = []
    status_label = f"{label} ({outcome.get('section')})" if include_section else label
    if outcome.get("status") != "passed":
        problems.append(
            f"{status_label} is {outcome.get('status')!r}"
            f" [{outcome.get('failure_kind')}]"
        )
    exit_code = outcome.get("exit_code")
    if type(exit_code) is not int or exit_code != 0:
        problems.append(f"{label} exit code is {exit_code!r}; expected integer 0")
    if "failure_kind" not in outcome or outcome["failure_kind"] is not None:
        problems.append(f"{label} failure kind is {outcome.get('failure_kind')!r}")
    if "signal" not in outcome or outcome["signal"] is not None:
        problems.append(f"{label} signal is {outcome.get('signal')!r}")
    if outcome.get("timed_out") is not False:
        problems.append(f"{label} timed_out is {outcome.get('timed_out')!r}")
    oom_kills = outcome.get("oom_kills")
    valid_oom_kills = oom_kills is None or (type(oom_kills) is int and oom_kills == 0)
    if "oom_kills" not in outcome or not valid_oom_kills:
        problems.append(f"{label} oom_kills is {oom_kills!r}; expected 0 or None")
    return problems


def manifest_problems(document: object, *, require_changes: bool = False) -> list[str]:
    """Every reason this manifest may not be read as a green run."""
    if not isinstance(document, dict):
        return ["manifest is not a JSON object"]
    problems: list[str] = []
    schema = document.get("schema")
    if type(schema) is not int or schema != MANIFEST_SCHEMA:
        problems.append(f"schema is {schema!r}, expected {MANIFEST_SCHEMA}")
    if document.get("runner") != "rustycore-validation-v2":
        problems.append(f"runner is {document.get('runner')!r}")
    if document.get("status") != "passed":
        problems.append(f"status is {document.get('status')!r}")
    exit_code = document.get("exit_code")
    if type(exit_code) is not int or exit_code != 0:
        problems.append(f"exit code is {exit_code!r}")
    runner_error = document.get("runner_error")
    if runner_error is not None:
        problems.append(f"runner error {runner_error!r}")
    if document.get("runner_signal") is not None:
        problems.append(f"runner died by signal {document.get('runner_signal')!r}")
    profile = document.get("profile")
    if profile not in ("self-test", "quick", "final", "audit"):
        problems.append(f"profile is {profile!r}; expected self-test, quick, final or audit")
    if require_changes and profile not in ("quick", "final"):
        problems.append(f"profile is {profile!r}; --require-changes requires quick or final")
    plan = document.get("plan")
    if isinstance(plan, dict):
        if "require_changes" in plan and type(plan["require_changes"]) is not bool:
            problems.append(
                f"plan.require_changes is {plan['require_changes']!r}; expected a boolean"
            )
        metadata = plan.get("metadata")
        if metadata is not None:
            problems.extend(_outcome_problems(metadata, "plan.metadata"))
    commands = document.get("commands")
    if not isinstance(commands, list):
        problems.append("commands are missing")
        return problems
    for index, command in enumerate(commands):
        problems.extend(
            _outcome_problems(command, f"command {index}", include_section=True)
        )
    if profile == "self-test":
        if "plan" not in document or plan is not None:
            problems.append("self-test plan must be None")
        if len(commands) != 1:
            problems.append(f"self-test recorded {len(commands)} commands; expected exactly one")
    elif profile in ("quick", "final", "audit"):
        if not isinstance(plan, dict):
            problems.append(f"{profile} plan is missing or is not an object")
            return problems
        if require_changes or plan.get("require_changes") is True:
            changed_paths = plan.get("changed_paths")
            if not isinstance(changed_paths, list) or not changed_paths or not all(
                isinstance(path, str) and bool(path) for path in changed_paths
            ):
                problems.append("changed paths are missing or invalid; expected a nonempty list of nonempty strings")
            if not commands:
                problems.append("no commands were recorded for the required nonempty validation scope")
        planned = plan.get("planned_steps")
        planned_commands = plan.get("planned_commands")
        if not isinstance(planned, list):
            problems.append("planned steps are missing or are not a list")
            return problems
        if len(planned) != len(commands):
            problems.append(f"{len(commands)} of {len(planned)} planned steps were executed")
        if not isinstance(planned_commands, list):
            problems.append("planned commands are missing or are not a list")

        ordered_argv: list[list[str]] = []
        valid_steps = True
        for index, step in enumerate(planned):
            if not isinstance(step, dict):
                problems.append(f"planned step {index} is not an object")
                valid_steps = False
                continue
            section = step.get("section")
            argv = step.get("argv")
            valid_section = isinstance(section, str) and bool(section)
            valid_argv = isinstance(argv, list) and bool(argv) and all(
                isinstance(argument, str) for argument in argv
            )
            if not valid_section:
                problems.append(f"planned step {index} has an invalid section")
            if not valid_argv:
                problems.append(f"planned step {index} has an invalid argv")
            if not valid_section or not valid_argv:
                valid_steps = False
                continue
            ordered_argv.append(argv)
            if index < len(commands) and isinstance(commands[index], dict):
                outcome = commands[index]
                if outcome.get("section") != section:
                    problems.append(
                        f"command {index} section {outcome.get('section')!r} does not match "
                        f"planned section {section!r}"
                    )
                if outcome.get("argv") != argv:
                    problems.append(f"command {index} argv does not match its planned step")
        if isinstance(planned_commands, list) and valid_steps and planned_commands != ordered_argv:
            problems.append("planned commands do not match planned step argv")
    return problems
