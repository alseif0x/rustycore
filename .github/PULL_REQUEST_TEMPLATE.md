## What changed?

<!-- Summarize the change in a few sentences. -->

## C++ reference

<!-- Required for port work. Link exact upstream C++ files/functions/lines from the canonical source you used. -->

- C++:
- Rust:

## Type of change

- [ ] C++ port / parity fix
- [ ] Runtime behaviour
- [ ] Packet / protocol
- [ ] Database / SQL
- [ ] Architecture / mechanical module split
- [ ] Documentation
- [ ] Tests only

## Verification

<!-- Only PRs authored by exactly alseif0x use the trusted local path; every other author
retains remote validation/reviewer checks. Final publication evidence names the clean committed
HEAD. Add focused evidence for changed behavior AND explicit issue acceptance; internal macro
checkpoints are not separate PRs. A skipped hosted job is not a test result. -->

- [ ] `./tools/validation-v2 final --base origin/3.4.3`
- [ ] Focused tests:
- [ ] Capture-diff (only packet/metadata/connection/order changes): not needed / result
- [ ] Runtime QA (only lifecycle/runtime changes): not needed / result
- Validated HEAD / manifest:
- Explicit issue acceptance completed / remaining:

## Migration notes

<!-- Mention roadmap/inventory rows touched, remaining gaps, and whether this is represented-partial or runtime/live-client-ready. -->

- Inventory / roadmap row:
- Remaining gaps:
- Manual client/bot tested: yes / no

## Risk

<!-- Describe compatibility, DB migration, performance, locking, packet, or gameplay risks. -->
