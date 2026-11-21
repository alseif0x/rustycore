# Contributing to RustyCore

Thanks for helping bring Azeroth to Rust. RustyCore is an active port of a TrinityCore-style
WotLK Classic 3.4.3 server, and contributions are most useful when they leave a clear trail
from the observed C++ behavior to the Rust change.

## Start here

Read the [documentation map](docs/README.md) and the [current state](docs/migration/STATE.md)
before picking work. The state document is dated; use it to understand what is evidenced and
what remains open. For project-wide rules, see [AGENTS.md](AGENTS.md).

Choose an existing issue when one matches your idea. For a new problem or bounded improvement,
open an issue before starting substantial work so the scope and ownership can be coordinated.
Create one feature branch from `3.4.3` for the issue and keep one pull request for its agreed scope.

## Ways to help

- Improve guides, examples, links, and explanations when the maintained source is unclear.
- Report reproducible login, protocol, database, or gameplay problems.
- Add focused Rust fixes and the positive, negative, or regression tests they need.
- Compare a behavior with the versioned C++ source or a target-build capture and record the result.
- Help review an existing change, reproduce it, or discuss the tradeoffs on Discord.

## Report an issue

Include the client build, RustyCore commit, operating system/architecture when relevant,
database/data setup, exact reproduction steps, expected behavior, and observed behavior.
Attach small sanitized logs or packet evidence when they clarify the report. Do not include
passwords, connection strings, private keys, certificates, account data, or unredacted runtime
configuration; use a private channel for sensitive material.

## Work on code

For protocol, gameplay, persistence, database, or runtime work:

1. Find the complete operation and its current callers and owner.
2. Locate the exact 3.4.3 C++ classes/functions, or capture the target behavior when source is ambiguous.
3. Implement the complete agreed change, preserving the established phase/order contract.
4. Keep one canonical authority per concept; do not introduce a detached mirror just to move code.
5. Add focused tests and update the owning checkpoint or documentation when the evidence changes.

Behavior changes and structural refactors should be described separately. A parser or represented
unit does not prove that the live server path is integrated; say which level the change reaches.

## Work on documentation

Update the document that already owns the information. Keep current status dated and evidence-based,
and link to the canonical source instead of creating a second ledger or repeating a full runbook.
Check relative links and code snippets against the current repository. English is the maintained
language for project documentation.

## Local validation

While developing, run the test that exercises your change. For a library regression, use
the package and test filter that own it. Confirm that the intended test ran.

```bash
PROTOC=/path/to/protoc cargo test --locked -j 1 -p <package> --lib <test-filter>
```

Use `--bin <name>` or `--test <name>` for a binary or integration target. For a type or interface
question, a scoped `cargo check` can give the answer before linking. Keep the same checkout
and target directory across edits. Use the pinned [protoc version](.protoc-version).

On the completed candidate, run the path-scoped final once and add the checks the issue
requires. It includes formatting and whitespace checks. The optional quick profile gives
hygiene feedback while editing. [Validation V2](docs/operations/validation-v2.md) ·
[Local-first development](docs/operations/local-first-development.md)

```bash
VALIDATION_V2_CARGO_JOBS=1 ./tools/validation-v2 final --base origin/3.4.3
```

The final profile belongs on the committed candidate before publication; follow the runner's
documented rules for unrelated documentation and evidence reuse. Prepare offline dependencies
using its [fresh-clone instructions](docs/operations/validation-v2.md#fresh-clone).
Do not claim a test,
capture, build, or live scenario that was not actually run. Runtime and durability claims need the
appropriate authorized integration or live evidence; library tests alone are not enough.

## Pull requests

Select `3.4.3` explicitly as the base for this client version. GitHub defaults to `forever`.
External PRs need `Full validation` and `Codex reviewer verdict`. For PRs authored by
`alseif0x`, both jobs skip and the maintainer supplies local acceptance evidence.
[Local and hosted checks](docs/operations/local-first-development.md)

Open or update one PR for the issue-linked branch. A useful PR description includes:

- the user-visible or maintainer-visible problem and resulting behavior;
- the relevant C++ anchor, capture identity, or reason a source comparison is not applicable;
- changed paths and focused tests/checks that were actually run;
- remaining boundaries, known parity differences, and any follow-up work.

Use `Closes #<issue>` when the PR fulfills the issue's acceptance criteria.
Keep unrelated cleanup out of the change. Reviewers should be able to trace the change, its owner,
and its evidence without reconstructing the whole migration history. Publication, merge, deployment,
and live server changes retain their separate authorization gates.

## License

RustyCore uses **GPL-3.0-or-later**, as declared in [Cargo.toml](Cargo.toml).
Read the [project notice](NOTICE) and [full license terms](LICENSE). Contributions must be
compatible with those terms. Preserve upstream authorship, license notices and provenance
when adapting code or data.

## Community

Ask in the [RustyCore Discord](https://discord.gg/mH6ACpGPb2), comment on the relevant issue, or
start a discussion on [GitHub Discussions](https://github.com/alseif0x/rustycore/discussions).
