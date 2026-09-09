"""Issue state checks.

Separated from check_architecture.py under #664; behaviour is preserved.
"""

from __future__ import annotations

from check_shared import *  # noqa: F401,F403
from check_policy import *  # noqa: F401,F403

def github_issue_facts(repository: str | None = None) -> dict[int, tuple[str, str]]:
    """Live issue state and title. Explicitly networked; never called by `check`."""
    if repository is None:
        try:
            repository = subprocess.run(
                ["gh", "repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"],
                capture_output=True,
                text=True,
                check=True,
                timeout=60,
            ).stdout.strip()
        except (OSError, subprocess.SubprocessError) as error:
            raise ArchitectureError(f"cannot resolve the GitHub repository: {error}") from error
    try:
        listing = subprocess.run(
            [
                "gh", "api", "--paginate",
                f"repos/{repository}/issues?state=all&per_page=100",
                "--jq", r'.[] | "\(.number)\t\(.state)\t\(.title)"',
            ],
            capture_output=True,
            text=True,
            check=True,
            timeout=300,
        ).stdout
    except (OSError, subprocess.SubprocessError) as error:
        raise ArchitectureError(f"cannot read live issue state: {error}") from error
    facts: dict[int, tuple[str, str]] = {}
    for line in listing.splitlines():
        number, _, rest = line.partition("\t")
        state, _, title = rest.partition("\t")
        if number.isdigit() and state:
            facts[int(number)] = (state.lower(), title)
    if not facts:
        raise ArchitectureError("live issue listing returned nothing")
    return facts


def refresh_issue_state(
    issue_ledger_path: pathlib.Path,
    runtime_ledger_path: pathlib.Path,
    write: bool,
) -> int:
    """Derive the mirrored state/title fields instead of maintaining them by hand."""
    issue_ledger = load_json(issue_ledger_path)
    runtime_ledger = load_json(runtime_ledger_path)
    tracked: list[dict[str, Any]] = []
    tracked.extend(issue_ledger.get("issues", []))
    tracked.extend(issue_ledger.get("external_prerequisites", []))
    tracked.extend(runtime_ledger.get("external_tracking_issues", []))
    policy_path = DEFAULT_POLICY
    policy_document = load_json(policy_path)
    reserved = policy_document.get("reserved_packages", [])
    for entry in reserved:
        tracked.append({"number": entry["issue"], "state": entry["state"]})
    facts = github_issue_facts()
    unknown = sorted(
        entry["number"] for entry in tracked if entry.get("number") not in facts
    )
    if unknown:
        raise ArchitectureError(f"tracked issues absent from the repository: {unknown}")
    drift: list[str] = []
    for entry in tracked:
        state, title = facts[entry["number"]]
        for field, value in (("state", state), ("title", title)):
            if field in entry and entry[field] != value:
                drift.append(f"#{entry['number']} {field}: {entry[field]!r} -> {value!r}")
                entry[field] = value
    for line in drift:
        print(line)
    if not drift:
        print("issue state mirror is current")
        return 0
    if not write:
        print(f"{len(drift)} mirrored field(s) are stale", file=sys.stderr)
        return 1
    for entry in reserved:
        entry["state"] = facts[entry["issue"]][0]
    for path, document in (
        (issue_ledger_path, issue_ledger),
        (runtime_ledger_path, runtime_ledger),
        (policy_path, policy_document),
    ):
        path.write_text(
            json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    print(f"refreshed {len(drift)} mirrored field(s)")
    return 0
