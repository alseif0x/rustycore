# RustyCore local review policy

Review only the changes in the selected diff. Read and follow `AGENTS.md`; do not report
pre-existing problems unless the patch makes them materially worse. Prefer actionable
correctness findings over style suggestions.

For gameplay, protocol, database, map/runtime, persistence, and packet behavior, contrast the
change with the exact C++ implementation under `/home/server/woltk-trinity-legacy`. Existing
Rust code, Rust comments, C# notes, migration percentages, and passing tests are not correctness
proof. If the legacy source is incomplete or ambiguous, require packet-capture or client-build
evidence rather than guessing.

Check every relevant change for:

- missing C++ branches, early returns, side effects, ordering, cleanup, and failure paths;
- wrong units or clock domains, persistence timing, restart windows, retries, and idempotency;
- confusion between database spawn IDs, runtime GUIDs, entries, map IDs, instance IDs, and map
  kinds;
- concurrency, lock ordering, ownership, duplicate processing, and per-session versus global
  runtime state;
- packet field order, bit counts, opcodes, registration/dispatch metadata, and recipient fanout;
- fallbacks that override canonical empty/`None` state or make represented-only behavior appear
  live;
- tests that do not exercise both positive and negative behavior, silently run zero cases, rely
  on timing, or assert implementation details instead of observable behavior;
- documentation or migration claims that exceed what code, capture-diff, runtime QA, or manual
  client evidence actually proves;
- unrelated changes, credentials, generated captures, local configs, binaries, or destructive
  automation entering the patch.

For build and workflow changes, also verify exact command equivalence between local and GitHub
execution, Rust 1.88.0 and locked dependency use, protoc 28.3 handling, shell quoting and exit-code
propagation, shallow-checkout behavior, and that safe default modes never start services or mutate
databases. Local review must not bypass the required GitHub `Codex reviewer verdict` for the PR's
current HEAD.

Report each real defect with the narrowest useful file and line range and a P0-P3 priority. Do not
invent speculative findings. Treat the patch as correct only when there are no actionable
findings in its stated scope.
