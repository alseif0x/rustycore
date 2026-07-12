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

To isolate one action from the surrounding login/session traffic, give the
import command directional inclusive boundaries. The stand-state bot requires
`SMSG_CONNECT_TO` plus distinct realm/instance sockets, waits for the realm ACK,
drains both sockets through a post-action quiet period, then sends a
deterministic `CMSG_PING` fence. Trimming through
that fence includes deferred `SMSG_UPDATE_OBJECT`/aura fanout instead of
stopping at the earlier `SMSG_STAND_STATE_UPDATE` ACK:

```bash
# In the second terminal, while each capture script waits for the flow:
cd tools/wow-test-bot
WOW_BOT_STAND_STATE_SMOKE=1 WOW_BOT_STAND_STATE=1 \
  ./run_rustycore_login_smoke.sh

# After recording both sides, install only a byte-clean isolated flow:
cd ../..
cargo run -p capture-diff -- import stand-state \
  --cpp target/captures/stand-state/cpp.pkt \
  --rust target/captures/stand-state/rust \
  --from-opcode c2s:0x318C \
  --until-opcode c2s:0x3768 \
  --ignore-opcode s2c:0x2DD4 \
  --direction both \
  --strict
cargo run -p capture-diff -- diff stand-state --strict
```

The fence uses a fixed ping serial and zero latency; its Pong is verified live
by the bot but intentionally falls after the inclusive import boundary. The
import fails if either capture lacks either boundary. With `--strict`, it
also refuses to write fixtures or a baseline unless the isolated flow is fully
clean. This prevents a missing or byte-different Rust response from being
hidden inside an accepted baseline.

`--ignore-opcode` is repeatable, direction-required, and applied symmetrically
after boundary selection. It is fail-closed to the reviewed periodic allowlist
(`s2c:0x2DD2 SMSG_TIME_SYNC_REQUEST` and `s2c:0x2DD4
SMSG_ON_MONSTER_MOVE`) and cannot overlap either action boundary. The
stand-state flow excludes only `SMSG_ON_MONSTER_MOVE` produced by the
independent global creature clock; the stand ACK, VALUES/aura side effects,
connection ids, request, and ping fence remain strict. `update-baseline`
rejects filters so only `import` can install a consistently selected fixture
pair.

Other subcommands: `show <PKT|DUMPDIR>` (list a capture), `list` (known flows),
`update-baseline <flow>` (re-pin the accepted divergences after a real fix).

## What it reports

The engine aligns both captures by opcode order **per direction** (LCS) and
reports four divergence classes:

- **count / presence** — `MISS` (in C++, not Rust) and `EXTRA` (in Rust, not C++);
- **order** — a moved packet drops out of the common subsequence and shows up as
  a `MISS` + `EXTRA` pair of the same opcode;
- **value** — an aligned packet whose body bytes differ (`VALUE`, with the first
  differing offset and a hex preview);
- **routing** — an aligned packet used a different C++ `ConnectionId`
  (`ROUTE`; realm `0`, instance `1`).

`c2s` should always diff clean (the same client drives both servers); divergences
live in the `s2c` server output.

## Capture formats (both native, no patching)

| Side | Mechanism | On disk |
|------|-----------|---------|
| C++  | `PacketLogFile` in worldserver.conf | one **PKT 3.1** binary (`PacketLog.cpp`) |
| Rust | `RUSTYCORE_PACKET_DUMP_DIR` env | one `.bin`+`.meta` pair per packet (`world_socket.rs`) |

Both log the **decrypted, uncompressed** opcode + body and preserve the socket
role, so they normalize to the same
`(direction, connection_id, opcode, body)` model. A routing mismatch makes a
strict diff fail even when opcode and body are byte-identical.

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
vars (server paths, pm2 process names) it honors. The Rust script recreates the
world process from a mode-0600 snapshot whose only capture-time difference is
`RUSTYCORE_PACKET_DUMP_DIR`, verifies the exact PM2 profile and listener ports,
and rejects a capture if the process PID/restart counter changes. The dump
parser also rejects duplicate global sequence numbers, which would otherwise
make a capture spanning an autorestart ambiguous.

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

### The committed `login` fixtures are a real capture

`login/cpp.pkt` and `login/rust/` are a **real capture** (2026-06-28): the same
character logging in against C++ TrinityCore (via `PacketLogFile`) and against
RustyCore (via `RUSTYCORE_PACKET_DUMP_DIR`), trimmed to the login flow (first
`CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE`, `0x3A46`). The flow diffs **s2c** only —
c2s carries per-session crypto/timestamps that change every capture. The
committed baseline is therefore the *current* real C++-vs-Rust login divergence
set (the live equivalent of `docs/migration/world-load-audit.md`); it shrinks as
Rust login parity improves.

To re-pin after a Rust login change (records into `target/`, which is gitignored,
then installs + re-baselines in one step):

```bash
crates/capture-diff/scripts/capture-cpp.sh  login   # -> target/captures/login/cpp.pkt
crates/capture-diff/scripts/capture-rust.sh login   # -> target/captures/login/rust/
cargo run -p capture-diff -- import login \
  --cpp target/captures/login/cpp.pkt \
  --rust target/captures/login/rust \
  --until-opcode 0x3A46 --direction s2c
```

`import` trims both captures at the boundary opcode, writes `cpp.pkt` + `rust/`,
and rewrites `expected-divergences.json`.

## Adding a flow

1. Record a `cpp.pkt` (C++ `PacketLogFile`) and a `rust/` dump (scripts above).
2. `cargo run -p capture-diff -- import <name> --cpp <pkt> --rust <dir> [--from-opcode c2s:0xNNNN] [--until-opcode s2c:0xNNNN] [--ignore-opcode s2c:0xNNNN] [--direction s2c]`
   — installs the fixtures under `flows/<name>/` and pins the baseline.
3. Optionally edit `flows/<name>/flow.json` (`description` / `directions`).
4. `cargo test -p capture-diff` — the new flow is now gated.
