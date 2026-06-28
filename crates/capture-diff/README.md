# capture-diff — C++(PKT) vs Rust packet capture diff harness

**Issue [01] / #66 — the acceptance gate for every milestone.**

"Done" across the whole port plan means the RustyCore wire output is byte/opcode
clean versus a C++ TrinityCore capture of the same action (STATE.md §5). This
crate automates that comparison — the same diff `docs/migration/world-load-audit.md`
did by hand.

## The one command

```bash
# Diff the committed login flow (golden C++ capture vs reference Rust capture):
cargo run -p capture-diff -- diff login

# Use a fresh Rust capture you just recorded:
cargo run -p capture-diff -- diff login --rust target/captures/login/rust

# Ad-hoc, no flow:
cargo run -p capture-diff -- diff --cpp some.pkt --rust some/dump/dir

# Regression gate (exit non-zero if the diff drifts from the accepted baseline):
cargo run -p capture-diff -- diff login --strict
```

Other subcommands: `show <PKT|DUMPDIR>` (list a capture), `list` (known flows),
`update-baseline <flow>` (re-pin the accepted divergences after a real fix).

## What it reports

The engine aligns both captures by opcode order **per direction** (LCS) and
reports the three divergence classes the audit tracked by hand:

- **count / presence** — `MISS` (in C++, not Rust) and `EXTRA` (in Rust, not C++);
- **order** — a moved packet drops out of the common subsequence and shows up as
  a `MISS` + `EXTRA` pair of the same opcode;
- **value** — an aligned packet whose body bytes differ (`VALUE`, with the first
  differing offset and a hex preview).

`c2s` should always diff clean (the same client drives both servers); divergences
live in the `s2c` server output.

## Capture formats (both native, no patching)

| Side | Mechanism | On disk |
|------|-----------|---------|
| C++  | `PacketLogFile` in worldserver.conf | one **PKT 3.1** binary (`PacketLog.cpp`) |
| Rust | `RUSTYCORE_PACKET_DUMP_DIR` env | one `.bin`+`.meta` pair per packet (`world_socket.rs`) |

Both log the **decrypted, uncompressed** opcode + body, so they normalize to the
same `(direction, opcode, body)` model.

## Recording a capture

Capture artifacts are large/PII-bearing — keep them out of git (the scripts
default to `target/captures/`, which is gitignored).

```bash
# C++ golden — sets PacketLogFile, restarts the legacy server, collects World.pkt
crates/capture-diff/scripts/capture-cpp.sh login

# Rust — runs the world server with RUSTYCORE_PACKET_DUMP_DIR, collects the dump
crates/capture-diff/scripts/capture-rust.sh login
```

Both scripts pause for you to perform the flow with a client, then collect the
artifact into `target/captures/<flow>/`. See each script's header for the env
vars (server paths, pm2 process names) it honors.

## Flows and the golden fixtures

A *flow* pins a golden capture so a milestone gets a regression gate. Layout
(committed under `flows/<name>/`):

```text
flows/login/cpp.pkt                    # C++ PKT 3.1 golden
flows/login/rust/                      # reference Rust dump (.bin/.meta)
flows/login/expected-divergences.json  # accepted-divergence baseline
flows/login/flow.json                  # description + directions
```

`cargo test -p capture-diff` runs the gate: it parses the committed pair, diffs
them, and asserts the result equals `expected-divergences.json`. When you fix a
divergence, the test fails until you re-pin with `update-baseline`.

### ⚠ The committed `login` fixtures are synthetic

To avoid committing real (PII-bearing, session-specific) captures, the committed
`login/cpp.pkt` and `login/rust/` are **synthetic** — authored by
`cargo run -p capture-diff --bin gen-fixtures` to model the divergences
catalogued in `docs/migration/world-load-audit.md`. They exercise and
regression-lock the harness end to end, but **the login flow is not "capture
clean" per STATE.md §5 until they are replaced with a live capture pair**:

```bash
crates/capture-diff/scripts/capture-cpp.sh  login          # -> target/captures/login/cpp.pkt
crates/capture-diff/scripts/capture-rust.sh login          # -> target/captures/login/rust/
cp target/captures/login/cpp.pkt   crates/capture-diff/flows/login/cpp.pkt
rm -rf crates/capture-diff/flows/login/rust && \
  cp -r target/captures/login/rust crates/capture-diff/flows/login/rust
cargo run -p capture-diff -- update-baseline login         # re-pin accepted divergences
```

## Adding a flow

1. Record a `cpp.pkt` and `rust/` pair (scripts above).
2. `mkdir crates/capture-diff/flows/<name>` and drop both in (+ optional
   `flow.json` with `description`/`directions`).
3. `cargo run -p capture-diff -- update-baseline <name>` to pin the baseline.
4. `cargo test -p capture-diff` — the new flow is now gated.
