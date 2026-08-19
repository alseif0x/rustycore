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
    if result.returncode:
        error = result.stderr.strip() or result.stdout.strip()
        if (
            re.search(r"\(HTTP (?:404|5\d\d)\)", error)
            or "unexpected end of JSON input" in error
        ):
            raise TransientGitHubApiError(error)
        raise RuntimeError(f"gh api {path} failed: {error}")
    try:
        return [json.loads(line) for line in result.stdout.splitlines() if line]
    except json.JSONDecodeError as error:
        raise TransientGitHubApiError(
            f"gh api {path} returned invalid paginated JSON: {error}"
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

    failures = 0
    for name, kind, body, inline, expected in cases:
        got = classify_codex_item(kind, body, inline)
        if got != expected:
            failures += 1
            print(f"FAIL expected={expected!r} got={got!r}: {name}")
        else:
            print(f"ok   {expected!r:6} {name}")
    if failures:
        print(f"{failures} of {len(cases)} self-test cases failed")
        return 1
    print(f"codex_review_gate self-test: {len(cases)} cases OK")
    return 0


if __name__ == "__main__":
    if "--self-test" in sys.argv[1:]:
        sys.exit(self_test())
    sys.exit(main())
