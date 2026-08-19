#!/usr/bin/env python3
"""Require a clean Codex review on a pull request's current HEAD.

Extracted from `.github/workflows/codex-review-gate.yml` so the verdict logic can
be exercised directly: run `python3 tools/ci/codex_review_gate.py --self-test`.
The gate had been failing clean reviews, which is unmergeable-with-nothing-to-fix,
so the classification rules are now covered by cases rather than only by the live
run. Mirrors the repo convention of tools carrying their own self-test subcommand.
"""

import json
import os
import re
import subprocess
import sys
import time

repo = os.environ.get("REPOSITORY", "")
pr_number = os.environ.get("PR_NUMBER", "")
head_sha = os.environ.get("HEAD_SHA", "").lower()
timeout_seconds = int(os.environ.get("CODEX_REVIEW_TIMEOUT_SECONDS", "1800"))
poll_seconds = int(os.environ.get("CODEX_REVIEW_POLL_SECONDS", "30"))

codex_logins = {"chatgpt-codex-connector", "chatgpt-codex-connector[bot]"}
positive_markers = (
    "didn't find any major issues",
    "did not find any major issues",
    "no major issues",
)
# A finding is a P-badge in the text. These are the only reliable
# in-body evidence that Codex actually reported something.
finding_markers = (
    "p0 badge",
    "p1 badge",
    "p2 badge",
    "p3 badge",
)
# Codex prints this header whenever it posts a body at all, including
# on reviews that carry no findings, so it cannot be read as feedback
# on its own. For issue comments there is nothing else to go on, so it
# still counts there; for reviews the inline comment count decides.
comment_only_finding_marker = "automated review suggestions"

class TransientGitHubApiError(RuntimeError):
    pass

def gh_json_pages(path):
    separator = "&" if "?" in path else "?"
    result = subprocess.run(
        [
            "gh",
            "api",
            "--paginate",
            f"{path}{separator}per_page=100",
            "--jq",
            ".[]",
        ],
        text=True,
        capture_output=True,
    )
    raise_for_gh_failure(result, path)
    try:
        return [json.loads(line) for line in result.stdout.splitlines() if line]
    except json.JSONDecodeError as error:
        raise TransientGitHubApiError(
            f"gh api {path} returned invalid paginated JSON: {error}"
        ) from error


def raise_for_gh_failure(result, path):
    if not result.returncode:
        return
    error = result.stderr.strip() or result.stdout.strip()
    if (
        re.search(r"\(HTTP (?:404|5\d\d)\)", error)
        or "unexpected end of JSON input" in error
    ):
        raise TransientGitHubApiError(error)
    raise RuntimeError(f"gh api {path} failed: {error}")


def gh_json_object(path):
    """Fetch one JSON object.

    Not gh_json_pages: that passes `--jq .[]`, which on an object iterates its
    values instead of yielding the object, so a single-resource endpoint such as
    a commit would come back shredded into unrelated fragments.
    """
    result = subprocess.run(
        ["gh", "api", path], text=True, capture_output=True
    )
    raise_for_gh_failure(result, path)
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise TransientGitHubApiError(
            f"gh api {path} returned invalid JSON: {error}"
        ) from error

def review_inline_comment_count(review_id):
    return len(
        gh_json_pages(
            f"repos/{repo}/pulls/{pr_number}/reviews/{review_id}/comments"
        )
    )

def reviewed_commits(body, fallback_commit=None):
    commits = [
        commit.lower()
        for commit in re.findall(
            r"Reviewed commit:[*\s]*`?([0-9a-fA-F]{7,40})`?",
            body or "",
        )
    ]
    if fallback_commit:
        commits.append(fallback_commit.lower())
    return commits

def is_current_head(commit):
    return bool(commit) and (head_sha.startswith(commit) or commit.startswith(head_sha))

def codex_items_for_current_head():
    comments = gh_json_pages(f"repos/{repo}/issues/{pr_number}/comments")
    reviews = gh_json_pages(f"repos/{repo}/pulls/{pr_number}/reviews")

    items = []
    for item in comments:
        if item.get("user", {}).get("login") not in codex_logins:
            continue
        body = item.get("body") or ""
        if any(is_current_head(commit) for commit in reviewed_commits(body)):
            items.append((
                "comment",
                body,
                item.get("html_url", ""),
                item.get("created_at", ""),
                None,
            ))

    for item in reviews:
        if item.get("user", {}).get("login") not in codex_logins:
            continue
        body = item.get("body") or ""
        fallback_commit = item.get("commit_id")
        if any(
            is_current_head(commit)
            for commit in reviewed_commits(body, fallback_commit)
        ):
            url = item.get("html_url") or item.get("_links", {}).get("html", {}).get("href", "")
            items.append((
                "review",
                body,
                url,
                item.get("submitted_at") or item.get("updated_at") or "",
                review_inline_comment_count(item["id"]),
            ))

    return items

def classify_codex_item(kind, body, inline_comments):
    """Decide pass/fail/unknown for one Codex item on the current HEAD.

    A P-badge anywhere in the text is a finding and always fails. Beyond
    that, a *review* is judged by whether it carries inline comments,
    because Codex emits its "automated review suggestions" header even
    when it found nothing: reading that header as feedback fails a clean
    review and leaves the PR unmergeable with nothing to address. An
    *issue comment* has no inline comments to count, so there the header
    remains the only available signal and still counts as feedback.
    """
    lower_body = body.lower()
    if any(marker in lower_body for marker in finding_markers):
        return "fail"
    if kind == "review":
        return "fail" if inline_comments else "pass"
    if comment_only_finding_marker in lower_body:
        return "fail"
    if any(marker in lower_body for marker in positive_markers):
        return "pass"
    return None

def head_commit_timestamp():
    """When the current HEAD commit was committed, as an ISO-8601 string."""
    commit = gh_json_object(f"repos/{repo}/commits/{head_sha}")
    return (commit.get("commit") or {}).get("committer", {}).get("date") or ""


def reaction_is_current(reaction_created_at, head_committed_at):
    """Whether a thumbs-up can be about the current HEAD rather than an older one.

    A reaction carries no commit reference, so the only thing tying it to a
    revision is that Codex reacts *after* reviewing. A thumbs-up older than the
    HEAD commit therefore cannot be about it and must not pass the gate; without
    this the check would go green on unreviewed code every time someone pushed a
    fix on top of an approved commit.
    """
    if not reaction_created_at or not head_committed_at:
        return False
    return reaction_created_at > head_committed_at


def codex_thumbs_up_for_current_head():
    """Codex's other way of saying it found nothing: a thumbs-up, with no review.

    Its own note reads "If Codex has suggestions, it will comment; otherwise it
    will react with a thumbs up", so a gate that only reads comments and reviews
    times out on exactly the clean case it is meant to let through.
    """
    head_committed_at = head_commit_timestamp()
    for reaction in gh_json_pages(f"repos/{repo}/issues/{pr_number}/reactions"):
        if reaction.get("user", {}).get("login") not in codex_logins:
            continue
        if reaction.get("content") != "+1":
            continue
        if reaction_is_current(reaction.get("created_at", ""), head_committed_at):
            return reaction.get("created_at", "")
    return None


def verdict():
    items = codex_items_for_current_head()
    classified = []

    for kind, body, url, timestamp, inline_comments in items:
        state = classify_codex_item(kind, body, inline_comments)
        if state:
            classified.append((timestamp, state, kind, url))

    if classified:
        timestamp, state, kind, url = max(classified, key=lambda item: item[0])
        return state, [(kind, url)]

    # Checked only when nothing was written about this HEAD, so a real finding
    # always outranks a thumbs-up.
    reacted_at = codex_thumbs_up_for_current_head()
    if reacted_at:
        return "pass", [("thumbs-up reaction", f"reacted at {reacted_at}")]

    return "wait", items



def main():
    if not repo or not pr_number or not head_sha:
        print("REPOSITORY, PR_NUMBER and HEAD_SHA must be set.")
        return 1

    print(f"Waiting for clean Codex review on PR #{pr_number} at {head_sha[:10]}")
    print("Comment '@codex review' on the PR if the review has not started.")
    deadline = time.time() + timeout_seconds

    while True:
        try:
            state, items = verdict()
        except TransientGitHubApiError as error:
            remaining = int(deadline - time.time())
            if remaining <= 0:
                print(
                    "Timed out waiting for GitHub API recovery while checking "
                    f"the Codex verdict: {error}"
                )
                sys.exit(1)

            retry_seconds = min(poll_seconds, remaining)
            print(
                "Transient GitHub API failure while checking the Codex verdict; "
                f"retrying in {retry_seconds}s ({remaining}s left): {error}"
            )
            time.sleep(retry_seconds)
            continue

        if state == "pass":
            print("Found clean Codex review for current HEAD:")
            for kind, url in items:
                print(f"- {kind}: {url}")
            sys.exit(0)

        if state == "fail":
            print("Codex left review feedback for current HEAD; address it before merge:")
            for kind, url in items:
                print(f"- {kind}: {url}")
            sys.exit(1)

        remaining = int(deadline - time.time())
        if remaining <= 0:
            print(
                "Timed out waiting for a clean Codex review on the current HEAD. "
                "Comment '@codex review' on the PR, then rerun this job after Codex responds."
            )
            sys.exit(1)

        print(
            f"No clean Codex verdict for {head_sha[:10]} yet; "
            f"checking again in {poll_seconds}s ({remaining}s left)."
        )
        time.sleep(poll_seconds)

def self_test():
    """Cases the live gate got wrong, plus the ones it must keep getting right."""
    codex_header = (
        "### Codex Review\n\nHere are some automated review suggestions for this "
        "pull request.\n\n**Reviewed commit:** `7e32308df3`\n\n<details>"
        "<summary>About Codex in GitHub</summary>If Codex has suggestions, it will "
        "comment; otherwise it will react with a thumbs up.</details>"
    )
    cases = [
        # (name, kind, body, inline_comments, expected)
        (
            "review carrying only the boilerplate header and no inline comments "
            "is clean -- this is the case that wrongly blocked PR #212",
            "review", codex_header, 0, "pass",
        ),
        (
            "review with the same header but inline comments is feedback",
            "review", codex_header, 3, "fail",
        ),
        (
            "a P-badge outranks a zero inline count, since the finding is in the body",
            "review", codex_header + "\n\nP1 Badge: broken rollback path", 0, "fail",
        ),
        (
            "issue comment keeps the header as its only signal: no inline comments exist",
            "comment", codex_header, None, "fail",
        ),
        (
            "issue comment stating no major issues passes",
            "comment", "I didn't find any major issues.\n**Reviewed commit:** `abc1234`",
            None, "pass",
        ),
        (
            "unrelated chatter yields no verdict, so the gate keeps waiting",
            "comment", "Reviewing now.\n**Reviewed commit:** `abc1234`", None, None,
        ),
        (
            "empty review body with no inline comments is still clean",
            "review", "", 0, "pass",
        ),
    ]

    reaction_cases = [
        (
            "a thumbs-up added after the HEAD commit is about that commit",
            "2026-08-19T11:20:00Z", "2026-08-19T11:00:00Z", True,
        ),
        (
            "a thumbs-up predating the HEAD commit cannot be about it, so pushing "
            "a fix on top of an approved commit must not stay green",
            "2026-08-19T10:00:00Z", "2026-08-19T11:00:00Z", False,
        ),
        (
            "a missing reaction timestamp is not evidence of anything",
            "", "2026-08-19T11:00:00Z", False,
        ),
        (
            "an unknown HEAD commit date fails closed rather than passing",
            "2026-08-19T11:20:00Z", "", False,
        ),
    ]

    failures = 0
    for name, kind, body, inline, expected in cases:
        got = classify_codex_item(kind, body, inline)
        if got != expected:
            failures += 1
            print(f"FAIL expected={expected!r} got={got!r}: {name}")
        else:
            print(f"ok   {expected!r:6} {name}")

    for name, reacted_at, committed_at, expected in reaction_cases:
        got = reaction_is_current(reacted_at, committed_at)
        if got != expected:
            failures += 1
            print(f"FAIL expected={expected!r} got={got!r}: {name}")
        else:
            print(f"ok   {expected!r:6} {name}")

    total = len(cases) + len(reaction_cases)
    if failures:
        print(f"{failures} of {total} self-test cases failed")
        return 1
    print(f"codex_review_gate self-test: {total} cases OK")
    return 0


if __name__ == "__main__":
    if "--self-test" in sys.argv[1:]:
        sys.exit(self_test())
    sys.exit(main())
