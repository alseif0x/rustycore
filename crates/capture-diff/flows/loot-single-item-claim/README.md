# Required capture: single-session loot item claim

This directory is the required real-capture gate for issue #106. Its existing
isolated one-client C++/Rust action windows contain six packets each and report
`6 matched, 0 value-diffs, 0 routing-diffs, 0 missing, 0 extra`, with an empty
accepted-divergence baseline. They predate the mandatory completed RAW
manifests, however, so their executable/process/config lineage cannot be
verified. They are useful semantic regression fixtures but are not accredited
completion evidence. `requirement.json` must stay `awaiting-real-captures` and
`verify-required` must fail until this runbook produces and imports a fresh
manifest-backed pair.

## C++-anchored wire contract

Use exactly one logged-in character and no party. Kill and open one disposable,
stationary creature whose loot table contains exactly one unconditional
quantity-1 item. Drain the open response before the action window, then claim
that item and finish with a fixed-serial, zero-latency `CMSG_PING` on the
instance socket. The imported window must begin and end at those two CMSGs.

After the three reviewed symmetric filters below, the required window is
exactly these six packets in this order:

1. instance `CMSG_LOOT_ITEM` (`c2s`, connection 1, `0x3211`);
2. instance `SMSG_UPDATE_OBJECT` Item `CreateObject` (`s2c`, connection 1,
   `0x27CB`);
3. instance `SMSG_LOOT_REMOVED` (`s2c`, connection 1, `0x2615`);
4. realm `SMSG_ITEM_PUSH_RESULT` (`s2c`, connection 0, `0x2623`);
5. instance `SMSG_UPDATE_OBJECT` one-slot `InvSlots` VALUES update (`s2c`,
   connection 1, `0x27CB`);
6. instance `CMSG_PING` fence (`c2s`, connection 1, `0x3768`).

This is grounded in the legacy source, not inferred from Rust:

- `LootHandler.cpp::HandleAutostoreLootItemOpcode` delegates to
  `Player::StoreLootItem`;
- `Player.cpp::StoreLootItem` calls `NotifyItemRemoved` before `SendNewItem`;
- `Opcodes.cpp` routes `SMSG_LOOT_REMOVED` to `CONNECTION_TYPE_INSTANCE` and
  `SMSG_ITEM_PUSH_RESULT` to `CONNECTION_TYPE_REALM`.

No extra, replacement, reordered, wrong-socket, wrong-direction, or duplicate
packet is accepted inside that post-filter window. The semantic contract also
decodes each side independently: the request is one non-soft claim for the
canonical type-15/map-530 LootObj at list `0`; the created Item is the exact
owner-visible quantity-1 entry `30712`; removal is owned by Doctor Maleficus
entry `21779` on map `530` and correlates the request LootObj/list; the push is
for character `15`, quantity `1`, the same canonical Item GUID and deterministic
slot/flags/bonus/modification shape; the one-child `InvSlots` VALUES update
correlates that player, item, and `SlotInBag`; and the fence body is fixed
`TOOL` serial plus zero latency. The bot sends the fence as soon as it has
received the one removal and one item-push response; it does not add a fixed
settle delay. Periodic time-sync request/response pairs and independent creature
movement are the only reviewed symmetric import filters.

## Reproducibility and safety

- The bot requires zero globally-online characters before setup, after logout
  verification, and after cleanup. It snapshots both disposable characters'
  position/money plus level, XP, health, powers, rest, exploration and title
  state; it also snapshots all personal achievement/criteria/quest rows and
  reputation. Cleanup restores these rows transactionally and reloads them for
  exact verification. The characters must not belong to a guild, because C++
  fans kill/item criteria into guild-wide state.
- The versioned defaults are character GUIDs `15` and `16`; setup verifies
  their configured account owners and every cleanup delete/update remains
  bounded to those two GUIDs (plus the newly awarded item GUID discovered
  under those owners).
- Item `30712` is The Doctor's Key. Character `15` must have exact top-level
  keyring destination `bag=0, slot=106` empty before either run. Wire evidence,
  persistence, and post-cleanup verification all pin that same slot; a grant
  displaced into a backpack slot is a failure even if the item entry matches.
- Snapshot the inventory rows, creature respawn row, and current maximum
  item-instance GUID before either run as an operator-visible cross-check.
- Use the same clean DB snapshot and restart each world server before its run.
  This is necessary because both servers allocate the awarded item at the
  process-global `MAX(item_instance.guid) + 1` frontier.
- Restore the snapshot before switching C++ → Rust. Do not run another item
  creator between the two captures.
- Keep money outside this window. Doctor Maleficus (entry `21779`, database
  spawn id `1117`) has exactly one guaranteed item (`30712`) and
  `GoldMin=GoldMax=0`; a money claim is a separate capture contract.
- Do not use the two-client loot-race dump as this golden. Global C++/Rust logs
  merge both clients, so concurrent claims cannot be attributed or ordered as a
  reproducible single-session flow.
- The paired issue-#106 captures proved one unavoidable identity difference:
  the same Doctor owner was Creature runtime counter `268` in C++ and `1` in
  Rust. `capture-diff` therefore normalizes only that Owner GUID's lower 40-bit
  map-runtime counter in `SMSG_LOOT_REMOVED`. Both counters must be nonzero;
  the complete stable Owner identity, complete LootObj GUID (including its own
  counter), LootListID, canonical packed shape, body boundaries, and instance
  routing remain strict. Do not add any broader GUID normalization.
- Two independent C++ runs also emitted the same second
  `SMSG_UPDATE_OBJECT` (`0x27CB`) body byte-for-byte. It was a 51-byte,
  single-player VALUES update whose `UnitData` contribution was exactly
  `08 00 10 00 00`: parent bit 116 for the Power arrays, with no child and no
  payload. The rest was one `ActivePlayerData::InvSlots` update and matched the
  46-byte Rust body exactly after removing those five C++ bytes. The legacy
  anchor is `Player.cpp::Regenerate` at the throttled
  `DoWithSuppressingObjectUpdates` + `UnitData::Power::ClearChanged` path;
  `UpdateFields.h` places the Power arrays under parent 116 and
  `UpdateFields.cpp::UnitData::WriteUpdate` emits that parent mask before any
  child payload. This is reproducible accumulated-change-mask/capture-cadence
  noise, not proof that Rust power regeneration or gameplay timing is correct.
- The corresponding semantic exception is intentionally narrower than the
  opcode. It requires S2C `0x27CB`, one VALUES block, the same map and canonical
  Player GUID, no destroy/out-of-range data, and the exact same one-slot
  canonical non-empty Item GUID update after deleting only the C++ parent-116
  mask. A Unit child or payload, any other Unit or ActivePlayer bit, a different
  GUID/slot/value, malformed or non-canonical bytes, reversed C++/Rust
  orientation, different direction/opcode/socket route, CreateObject, or any
  other UpdateObject shape still fails.
- For the complete C++→Rust pair, use the host's external firewall/control plane
  to allow world ports `8085` and `8086` only from loopback (`127.0.0.1` and
  `::1`). Apply the block before Terminal A starts C++, verify it from a remote
  host, keep it in place through Rust capture and strict import, then restore the
  normal policy. The database online-count guards detect persisted sessions but
  cannot close the race of an external client connecting after preflight.

Record each side with the existing service-safe wrappers while performing the
same one-client action when they pause. There are two separate acknowledgements
and neither substitutes for the other:

1. answer `y` to each capture script's service swap/restart prompt (or pass its
   explicit `--yes` only in controlled automation);
2. set `WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1` for the bot's destructive
   fixture guard.

Pin the exact feature binary and bot bytes before touching services. The Rust
capture wrapper checks the world executable both at its source path and through
`/proc/<pid>/exe`; the bot wrapper likewise refuses a supplied executable whose
SHA-256 does not match.

The RustyCore worktree must be committed and completely clean (including
non-ignored untracked files) for both recordings. The legacy C++ checkout is
known to contain local port/test changes, so its HEAD is informational rather
than a build attestation: the wrapper records `source_worktree_dirty` and a
stable path/mode/content-hash state digest. The pinned live `worldserver`
path/SHA-256 is the primary C++ executable evidence. RAW manifest schema v3
records this explicitly. Its PM2 profile runs
`worldserver-wrapper.sh` without `exec`; the manifest therefore records that
PM2 entrypoint separately and proves that the one PID owning both listeners is
its descendant.

```bash
# Preparation (no service mutation).
REPO_ROOT=/absolute/path/to/your/rustycore-worktree
cd "$REPO_ROOT"
test -z "$(git status --porcelain=v1 --untracked-files=normal)"
PROTOC=/home/ubuntu/.local/protoc/bin/protoc \
  cargo build --locked -p world-server
cargo build --locked \
  --manifest-path tools/wow-test-bot/Cargo.toml

RUST_EXEC="$(realpath target/debug/world-server)"
BOT_EXEC="$(realpath tools/wow-test-bot/target/debug/wow-test-bot)"
git rev-parse HEAD
sha256sum /home/server/trinity-legacy-install/bin/worldserver \
  "$RUST_EXEC" "$BOT_EXEC"
```

The wrappers refuse to overwrite an existing RAW generation. Before starting
a replacement run, archive the old gitignored directory under another name (or
remove it only after separately preserving anything needed for investigation):

```bash
test -d target/captures/loot-single-item-claim && \
  mv target/captures/loot-single-item-claim \
     "target/captures/loot-single-item-claim.pre-lineage-$(date -u +%Y%m%dT%H%M%SZ)"
```

Then use two terminals. For C++:

```bash
# Terminal A: answer y, then leave this waiting at "Perform the flow".
REPO_ROOT=/absolute/path/to/your/rustycore-worktree
cd "$REPO_ROOT"
DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf
CPP_EXEC="$(realpath /home/server/trinity-legacy-install/bin/worldserver)"
CPP_SHA="$(sha256sum "$CPP_EXEC" | awk '{print $1}')"
BOT_EXEC="$(realpath tools/wow-test-bot/target/debug/wow-test-bot)"
BOT_SHA="$(sha256sum "$BOT_EXEC" | awk '{print $1}')"
FIXTURE_DIR="${TMPDIR:-/tmp}/rustycore-loot-item-cpp-${USER:-$(id -u)}"
install -d -m 700 "$FIXTURE_DIR"
FIXTURE_JOURNAL="$FIXTURE_DIR/fixture.journal"
BOT_REPORT="$FIXTURE_DIR/bot-report.json"
BOT_LOG="$FIXTURE_DIR/bot.log"
test ! -e "$FIXTURE_JOURNAL" \
  && test ! -L "$FIXTURE_JOURNAL" \
  && test ! -e "${FIXTURE_JOURNAL}.cleanup-complete" \
  && test ! -L "${FIXTURE_JOURNAL}.cleanup-complete" \
  && test ! -e "$BOT_REPORT" \
  && test ! -L "$BOT_REPORT" \
  && test ! -e "$BOT_LOG" \
  && test ! -L "$BOT_LOG"
CPP_CAPTURE_LOOT_FIXTURE_GUARD=1 \
CPP_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1 \
CPP_CAPTURE_DB_CONF="$DB_CONF" \
CPP_CAPTURE_EXEC="$CPP_EXEC" \
CPP_CAPTURE_EXEC_SHA256="$CPP_SHA" \
WOW_BOT_EXEC="$BOT_EXEC" \
WOW_BOT_EXEC_SHA256="$BOT_SHA" \
WOW_BOT_REPORT="$BOT_REPORT" \
WOW_BOT_FIXTURE_JOURNAL="$FIXTURE_JOURNAL" \
crates/capture-diff/scripts/capture-cpp.sh loot-single-item-claim

# Terminal B: only after Terminal A is waiting.
REPO_ROOT=/absolute/path/to/your/rustycore-worktree
cd "$REPO_ROOT"
DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf
FIXTURE_DIR="${TMPDIR:-/tmp}/rustycore-loot-item-cpp-${USER:-$(id -u)}"
FIXTURE_JOURNAL="$FIXTURE_DIR/fixture.journal"
BOT_EXEC="$(realpath tools/wow-test-bot/target/debug/wow-test-bot)"
BOT_SHA="$(sha256sum "$BOT_EXEC" | awk '{print $1}')"
BOT_REPORT="$FIXTURE_DIR/bot-report.json"
BOT_LOG="$FIXTURE_DIR/bot.log"
WOW_BOT_DB_CONF="$DB_CONF" \
WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
WOW_BOT_FIXTURE_JOURNAL="$FIXTURE_JOURNAL" \
WOW_BOT_LOOT_ITEM_CAPTURE=1 \
WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1 \
WOW_BOT_EXEC="$BOT_EXEC" \
WOW_BOT_EXEC_SHA256="$BOT_SHA" \
WOW_BOT_REPORT="$BOT_REPORT" \
WOW_BOT_LOG="$BOT_LOG" \
tools/wow-test-bot/run_rustycore_login_smoke.sh
```

The C++ wrapper uses the same shared guard as the Rust wrapper: after Rust is
stopped, it requires every character offline, atomically changes only Doctor
Maleficus entry `21779` difficulty-0 `HealthModifier` from `1` to `0.0001`, and
arms exact CAS restoration before C++ starts. Wait for Terminal B to exit `0`;
the pinned report and bot log remain under the private `FIXTURE_DIR` for the
operator and RAW-manifest validation.
Only then press
ENTER in Terminal A; that copies `cpp.pkt`, stops C++, restores its config, and
starts the normal Rust PM2 process only after the Doctor row is restored and the
bot journal is gone with a valid mode-0600 cleanup marker. The marker is consumed
only after PM2 starts successfully.

If the bot fails or receives TERM/INT, do not collect the run. Interrupt
Terminal A normally; its trap stops C++, restores the outer Doctor guard, and
leaves the normal world stopped while the journal is pending. Then use the exact
retained path for explicit bot recovery:

```bash
WOW_BOT_DB_CONF="$DB_CONF" \
WOW_BOT_FIXTURE_JOURNAL="$FIXTURE_JOURNAL" \
"$BOT_EXEC" --recover-loot-fixture
```

Only after recovery removes the journal and writes a valid marker may the
operator restart the normal PM2 world and remove that marker. A `SIGKILL` or
host death of Terminal A crosses the outer process boundary, so no shell trap runs.
In that case keep both worlds stopped, retain the `.capture-diff.bak`, journal,
and marker evidence, recover the bot journal explicitly, and inspect/restore the
Doctor CAS plus C++ config before any restart. Use a clean disposable DB snapshot
only if that outer state can no longer be proven; a normal bot failure is not a
reason to bypass journal recovery.

For Rust, recalculate the hashes from the unchanged files and pin the feature
world executable in the capture wrapper:

```bash
# Terminal A: answer y, then leave this waiting at "Perform the flow".
REPO_ROOT=/absolute/path/to/your/rustycore-worktree
cd "$REPO_ROOT"
DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf
RUST_CONFIG=/home/server/trinity-legacy-install/etc/worldserver.conf
FIXTURE_DIR="${TMPDIR:-/tmp}/rustycore-loot-item-rust-${USER:-$(id -u)}"
install -d -m 700 "$FIXTURE_DIR"
FIXTURE_JOURNAL="$FIXTURE_DIR/fixture.journal"
BOT_REPORT="$FIXTURE_DIR/bot-report.json"
BOT_LOG="$FIXTURE_DIR/bot.log"
test ! -e "$FIXTURE_JOURNAL" \
  && test ! -L "$FIXTURE_JOURNAL" \
  && test ! -e "${FIXTURE_JOURNAL}.cleanup-complete" \
  && test ! -L "${FIXTURE_JOURNAL}.cleanup-complete" \
  && test ! -e "$BOT_REPORT" \
  && test ! -L "$BOT_REPORT" \
  && test ! -e "$BOT_LOG" \
  && test ! -L "$BOT_LOG"
RUST_EXEC="$(realpath target/debug/world-server)"
RUST_SHA="$(sha256sum "$RUST_EXEC" | awk '{print $1}')"
BOT_EXEC="$(realpath tools/wow-test-bot/target/debug/wow-test-bot)"
BOT_SHA="$(sha256sum "$BOT_EXEC" | awk '{print $1}')"
RUST_CONFIG="$(realpath "$RUST_CONFIG")"
RUST_CAPTURE_EXEC="$RUST_EXEC" \
RUST_CAPTURE_EXEC_SHA256="$RUST_SHA" \
RUST_CAPTURE_EFFECTIVE_CONFIG="$RUST_CONFIG" \
RUST_CAPTURE_LOOT_FIXTURE_GUARD=1 \
RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1 \
RUST_CAPTURE_DB_CONF="$DB_CONF" \
WOW_BOT_EXEC="$BOT_EXEC" \
WOW_BOT_EXEC_SHA256="$BOT_SHA" \
WOW_BOT_REPORT="$BOT_REPORT" \
WOW_BOT_FIXTURE_JOURNAL="$FIXTURE_JOURNAL" \
crates/capture-diff/scripts/capture-rust.sh loot-single-item-claim

# Terminal B: run the exact same pinned bot mode used for C++.
REPO_ROOT=/absolute/path/to/your/rustycore-worktree
cd "$REPO_ROOT"
DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf
FIXTURE_DIR="${TMPDIR:-/tmp}/rustycore-loot-item-rust-${USER:-$(id -u)}"
FIXTURE_JOURNAL="$FIXTURE_DIR/fixture.journal"
BOT_EXEC="$(realpath tools/wow-test-bot/target/debug/wow-test-bot)"
BOT_SHA="$(sha256sum "$BOT_EXEC" | awk '{print $1}')"
BOT_REPORT="$FIXTURE_DIR/bot-report.json"
BOT_LOG="$FIXTURE_DIR/bot.log"
WOW_BOT_DB_CONF="$DB_CONF" \
WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
WOW_BOT_FIXTURE_JOURNAL="$FIXTURE_JOURNAL" \
WOW_BOT_LOOT_ITEM_CAPTURE=1 \
WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1 \
WOW_BOT_EXEC="$BOT_EXEC" \
WOW_BOT_EXEC_SHA256="$BOT_SHA" \
WOW_BOT_REPORT="$BOT_REPORT" \
WOW_BOT_LOG="$BOT_LOG" \
tools/wow-test-bot/run_rustycore_login_smoke.sh
```

Again, press ENTER in Terminal A only after Terminal B exits `0`. The Rust
wrapper then verifies that the live process still matches the pinned hash,
collects the dump, and restores the exact original PM2 profile. Do not import a
capture from a failed bot run or a process-provenance warning. A normal failed
run uses the same explicit `--recover-loot-fixture` procedure above after the
capture trap has stopped the QA world; do not delete a pending journal or invent
a cleanup marker.

Finally, without either capture script running:

```bash
REPO_ROOT=/absolute/path/to/your/rustycore-worktree
cd "$REPO_ROOT"
cargo run -p capture-diff -- import loot-single-item-claim \
  --cpp target/captures/loot-single-item-claim/cpp.pkt \
  --rust target/captures/loot-single-item-claim/rust \
  --cpp-manifest target/captures/loot-single-item-claim/cpp.capture-manifest.json \
  --rust-manifest target/captures/loot-single-item-claim/rust/rust.capture-manifest.json \
  --from-opcode c2s:0x3211 \
  --until-opcode c2s:0x3768 \
  --ignore-opcode s2c:0x2DD2 \
  --ignore-opcode c2s:0x3A3D \
  --ignore-opcode s2c:0x2DD4 \
  --direction both \
  --strict
```

After that import reports `CLEAN`, maintainers must:

1. inspect both artifacts with `capture-diff show`, both retained files under
   `capture-provenance/`, and `capture-lineage.json`; confirm the one-client
   provenance and all repository/executable/PM2/config identities;
2. set `requirement.json` to `"status": "ready"` with no stale
   `blocked_reason`;
3. run `cargo run -p capture-diff -- verify-required
   loot-single-item-claim` and the local `capture` preflight.

The pre-lineage issue-#106 import on 2026-07-18 did not produce the RAW
manifests now required by this gate, so it does not satisfy those steps. A new
capture must repeat the complete provenance, fixture restoration, strict
import, inspection, and verification sequence; `ready` is never permission to
replace the golden with uninspected input.

`verify-required` rejects missing artifacts or manifests, lineage/output hash
drift, any import selection other than the exact declared boundaries and three
ordered ignores, a non-empty accepted-divergence baseline, routing/order drift,
extra packets inside the selected action window, or any byte/opcode diff.
