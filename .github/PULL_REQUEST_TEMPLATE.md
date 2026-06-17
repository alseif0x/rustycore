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
- [ ] Documentation
- [ ] Tests only

## Verification

<!-- Check every command that was run. Add extra focused tests when relevant. -->

- [ ] `cargo fmt --all --check`
- [ ] `cargo check -p wow-data -p wow-database -p wow-network -p wow-world -p world-server`
- [ ] Focused tests:
- [ ] `git diff --check`

## Migration notes

<!-- Mention roadmap/inventory rows touched, remaining gaps, and whether this is represented-partial or runtime/live-client-ready. -->

- Inventory / roadmap row:
- Remaining gaps:
- Manual client/bot tested: yes / no

## Risk

<!-- Describe compatibility, DB migration, performance, locking, packet, or gameplay risks. -->
