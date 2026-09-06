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

# Milestone gate (operator-attested ready state, empty baseline and pinned shape):
cargo run -p capture-diff -- verify-required loot-single-item-claim
cargo run -p capture-diff -- verify-required creature-spell-casting
```

`verify-required` is intentionally stricter than `diff --strict`: it refuses an
`awaiting-real-captures` contract, missing artifacts, accepted divergences, an
incorrect exact packet count/boundary/socket/order shape, an invalid correlated
payload, a selection that differs from the exact reviewed boundaries/ignores,
a missing or malformed RAW-to-derived lineage, any retained manifest/output
hash mismatch, or any C++↔Rust difference. This lets a PR record exactly which
capture it still owes without manufacturing a golden. Both required contracts
shown above are enforced `ready` gates backed by accredited schema-v3 RAW
manifests and completed RAW-to-derived lineage: issue #106's
`loot-single-item-claim` permits exactly six packets per side and issue #26's
final `creature-spell-casting` generation exactly two. Both compare
strict-CLEAN with empty accepted-divergence baselines; issue #26's current
generation also passes `verify-required` from clean harness HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`.

The `ready` status is an operator attestation made only after inspecting the
capture run and its provenance. Each wrapper publishes a completed RAW
manifest only after cleanup succeeds. `import` validates that manifest against
the RAW bytes, preserves an exact copy, and hashes the filtered C++ packet,
normalized Rust dump, and baseline into `capture-lineage.json`. The manifest
also pins the harness/source commits and worktree-state digests,
expected/source/live executable paths and hashes, PM2 entrypoint path/hash,
entry/listener identity and restart count, plus a canonical
redacted capture-config hash. This is an integrity and review chain, not a
cryptographic signature: repository write access can still replace evidence
and all of its hashes together, which normal review must catch.

For a required flow, `required_order` is the complete post-filter packet
sequence, not a subsequence. The issue-#106 contract therefore permits exactly
the six declared packets; an extra or duplicate packet fails even when it uses
the wrong direction or socket. Its versioned semantic contract independently
requires one loot request and correlates the LootObj/list id, Doctor removal,
created item, recipient, quantity, item GUID/entry, deterministic keyring slot
and inventory update before accepting either side of the pair.

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

The default RAW manifests are the wrapper outputs next to those captures:
`target/captures/<flow>/cpp.capture-manifest.json` and
`target/captures/<flow>/rust/rust.capture-manifest.json`. Use
`--cpp-manifest` or `--rust-manifest` only if those same completed manifests
were deliberately moved. Import rejects a missing/unknown field, wrong
flow/side, incomplete run, RAW size/count/hash mismatch, malformed UTC time,
process/executable inconsistency, a dirty RustyCore harness/source, or an
unpinned executable for a required flow. A legacy C++ source checkout may be
dirty: its HEAD is then informational, `source_worktree_dirty` is explicit,
and a deterministic changed/untracked path+mode+content-hash state digest is
retained. The pinned listener-binary path/SHA remains the primary C++ runtime
identity; the manifest does not claim that the recorded source HEAD built it.

The fence uses a fixed ping serial and zero latency; its Pong is verified live
by the bot but intentionally falls after the inclusive import boundary. The
import fails if either capture lacks either boundary. With `--strict`, it
also refuses to write fixtures or a baseline unless the isolated flow is fully
clean. This prevents a missing or byte-different Rust response from being
hidden inside an accepted baseline.

The rested-XP gate isolates the first kill reward packet from the complete bot
round-trip:

```bash
cargo run -q -p capture-diff -- import rested-xp-kill \
  --cpp target/captures/rested-xp-kill/cpp.pkt \
  --rust target/captures/rested-xp-kill/rust \
  --from-opcode s2c:0x26E5 \
  --until-opcode s2c:0x26E5 \
  --direction s2c \
  --strict
```

`SMSG_LOG_XP_GAIN` uses one narrow semantic comparator only for Creature Kill
XP with nonzero runtime counters. It normalizes the lower 40-bit runtime
counter of the victim `ObjectGuid`; socket routing, high type, realm, map,
entry, subtype, server id, `Original`, `Reason`, `Amount`, and the exact IEEE
bits of `GroupBonus` remain strict. Malformed bodies and zero counters can
never compare clean. If a semantic mismatch is ever recorded in a non-clean
baseline, its signature includes the comparator plus the exact mismatched
stable values and decode errors from both sides. Invalid bodies additionally
carry a SHA-256 identity, as does any valid non-kill side that enters a semantic
mismatch because its peer is malformed or is creature-kill XP. Raw lengths are
omitted because the normalized packed GUID counter can change them between
equivalent runs. A regression in a different field—or different bytes producing
the same decode error—therefore cannot silently reuse the accepted divergence.
The one-packet fixture proves the reward wire shape; the bot workflow separately
proves offline accrual, DB XP/rest consumption, relog persistence, fixture
restoration, and the natural respawn timer.

Issue #62 uses a similarly narrow `SMSG_SEND_KNOWN_SPELLS` comparator for one
live Blood Elf Hunter login. C++ builds `KnownSpells` and `FavoriteSpells` by
iterating `PlayerSpellMap` (`std::unordered_map`), so their vector order is not
a stable protocol value. The comparator decodes the complete body, requires
canonical bit padding and exact lengths/counts, rejects zero/duplicate spell
IDs and favorites absent from the known set, then sorts each list before
comparison. `InitialLogin`, exact membership/cardinality, favorites, opcode,
direction, and connection routing remain strict. The focused regression embeds
the exact C++ and Rust wire orders plus source capture hashes and proves both
43-spell sets are identical. The bot's
`WOW_BOT_LOGIN_EXPECT_KNOWN_SPELLS` gate independently validates that exact
set against a live Rust login packet.

The installed legacy C++ runtime could complete this login only on its direct
world socket, while Rust completed the normal `SMSG_CONNECT_TO` split flow.
Consequently the issue-#62 evidence claims capture-clean packet semantics, not
numeric capture-local connection identity across those two different
topologies. C++ registers `SMSG_SEND_KNOWN_SPELLS` as
`CONNECTION_TYPE_INSTANCE`, Rust emits it on its authenticated instance
socket, and the comparator itself does not normalize routing; a same-topology
connection regression remains a separate hard failure.

The versioned `creature-spell-casting-v1` contract applies fail-closed semantic
comparisons to the mandatory adjacent `SMSG_SPELL_START` (`0x2C37`) then
`SMSG_SPELL_GO` (`0x2C36`) pair. Its reviewed live C++/Rust generation is
strict-CLEAN (2/2 packets) with an empty accepted-divergence baseline. The
required-flow validator pins Cabal Interrogator entry `22378` on map `530`,
Eviscerate `15691`, SpellXSpellVisual `244493`, exact
START/GO flags `0x2`/`0x100` with CastFlagsEx `0`, and one unit-only Player
target (the guarded character GUID `15`) that appears exactly once in GO's hit
list with no miss or optional
power/rune/target-point/ammo state. Both packets decode the complete 3.4.3
`SpellCastData`, including
canonical packed GUIDs and bit padding, both caster fields, spell visual and
cast flags, missile/immunity/heal-prediction fields, complete
`SpellTargetData`, hit/miss/status topology, and power/rune/target-point/ammo
optionals; GO additionally validates the trailing basic combat-log bit. The
contract correlates exact same-side caster, CastID, spell, visual and target
values across START→GO. For an ordinary Creature AI cast, only the lower
40-bit runtime counters of the exactly correlated
`CasterGUID == CasterUnit` and normal-source `HighGuid::Cast` ID are normalized
on both opcodes. GO's wrapping timestamp `CastTime` is also normalized, while
START's cast-duration `CastTime` remains exact. Creature/cast type, realm, map,
entry (spell ID), subtype/source and server ID remain strict.
The guarded fixture pins the Creature, Player victim, and Cast GUID to local
realm `1`; no realm bits are normalized (the legacy GUID factory expands its
zero realm argument to that local realm before serialization).
`OriginalCastID` must remain exact EMPTY. A self target or hit is normalized
only by proving exact same-side equality with the caster; unrelated Creature
GUIDs remain exact. Player/item casts retain raw byte identity. Missing or
reordered START/GO, full combat-log payloads, noncanonical padding/GUIDs,
mismatched miss/status counts, trailing bytes and even identical malformed
bodies all fail closed.

The current `15691` pair is the final issue-#26 P1 recapture and specifically
an observed **HIT** sample; it does not establish deterministic hit behavior.
Both C++ and Rust wrappers ran from clean harness HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2` under
`creature-spell-casting-shell-fixture-v2`. Before capture, that guard verifies
the installed stock `AIName=SmartAI` / difficulty-0 `StaticFlags1=0` snapshot,
CAS-switches it to `CombatAI` / `0x00100000`
(`CREATURE_STATIC_FLAG_NO_MELEE`), and after capture CAS-restores the exact
`SmartAI` / `0` pair. `NO_MELEE` suppresses the unrelated automatic melee path
without disabling CombatAI's EventMap cast or consuming its melee damage RNG
before the due spell.

The final P1 hardening removes the unconditional-hit assumption: atomic
START/GO publication is limited to physical `DmgClass=MELEE` Creature spells
against a Player attacked from behind, with zero spell/effect mechanics and
complete Creature/Player source authority that proves every omitted source is
hit-inert for this bounded result. Completeness is not an emptiness shortcut:
the reduced runtime's canonical local aura application/modifier/visible
containers must still be empty, while loaded persistence/login sources may be
nonempty only when their exact effects are proven unable to change the result.

Player authority fails closed across persistence and login/zone reconciliation,
map and area ancestry, guild, skills, active/rewarded and auto-push quests,
glyphs, active-spec traits, pets and battle-pet slots, FFA/PvP/war-mode rules,
SpellArea/outdoor/battlefield sources, and script, legacy/all-rank and
SpellLinked hooks. A valid SpellLinked hook blocks the candidate; so does the
absolute trigger spell ID retained from a rejected SpellLinked loader row,
because a malformed row cannot prove that the hook is absent.

Once accredited, the Creature-owned `0..=9_999` hit roll resolves the base 5%
`MISS` branch below `500`, otherwise `HIT`. The local C++ order is retained:
cast before repeat scheduling, and hit roll before the cooldown draw;
`NO_ATTACK_MISS` still consumes exactly one hit roll before forcing `HIT`.
An accepted `HIT` is published, then tombstones the local stream before repeat
scheduling because C++ next consumes an unconditional launch critical roll and
potential effect-value draws that this wire-only slice does not represent. A
`MISS` reaches none of those target-effect draws and may retain authority for
its repeat-delay draw.
Spell, melee and movement randomness share one fail-closed runtime tombstone:
an unrepresentable random branch disables subsequent random-dependent work for
that Creature. A valid melee swing that reaches the still-unrepresented C++
damage/outcome/proc calculation sets that tombstone and publishes no invented
damage or melee wire. C++ owns one process-global random engine whereas Rust
owns one RNG per Creature, so this slice claims the same distributions and
local causal draw order, not an exact global sequence or cross-Creature
interleaving. Damage and effects remain outside this wire-only slice; the final
issue-#26 acceptance closes only this bounded P1 wire/lifecycle contract, not
the full Spell effect or combat pipeline.

The C++ RAW PKT
`b52cc8ba962160be63286e72eb7611c6282b0cdc3a1cee0082fc6d6d7bf2c7b9`
comes from an accredited source derivation: base HEAD
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, one-file patch SHA-256
`ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262`,
and resulting HEAD `8cfed90bf1720dbf8b9dc109113c8d7d9173ff6c`. The patch only corrects the
`ChrSpecialization` index-container bound required by the installed DB2 data;
it changes no AI or spell path. The Rust RAW tree
`9aee309d9ffb2e2e1e5a33167c228ccaa8d1634d917efd026e1a525f2a5db94a`
comes from clean source HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`. The retained C++ and Rust
manifest hashes are respectively
`d40e3615b3337a26a3c4d4e380dc665c23719133ec0b3c7a05febdfd640e849d`
and `3c942209db52f9f36b3d661477f0cad766e7e2ee49cf1fc97d68a74d996f0da0`.
Strict import produced filtered C++ PKT
`a6b32206e3277e455e25f6aa8e491606aa5cd9449e2bf24245ea9dd5db79d932`,
normalized Rust tree
`cc8d53b06c2727c95990eda80fd095a1a7e390da0af16981429d07176e0c003b`,
and lineage file
`f443539e7857ac27dfb2029012f1e889d92ed27a224f89f7a6247f9510f0479d`.
`diff --strict` reports 2 matched packets and zero differences, and
`verify-required creature-spell-casting` reports CLEAN with exact
topology/order and correlated payload semantics.
This wire window proves only the bounded START/GO publication contract; it does
not claim spell-effect, damage, power, aura, or health-state mutation parity.

The issue-#108 vendor fixture isolates the exact post-COMMIT realm response:
`SMSG_BUY_SUCCEEDED` followed by `SMSG_ITEM_PUSH_RESULT`. Paired C++ and Rust
bot runs bought item `30183` from G'eras (entry `18525`, spawn `96654`) for
extended cost `1642`, proved currency `42` changed `30→15`, verified the item
after a fresh authentication, and restored the fixture. The strict two-packet
flow is CLEAN with an empty divergence baseline. Both schema-v3 manifests bind
the pinned bot executable and exact validated report, and import retains both
reports under `capture-provenance/` for re-verification. Its narrow
`SMSG_BUY_SUCCEEDED` comparator omits only G'eras' nonzero lower 40-bit
map-runtime GUID counter while pinning Creature/realm 1/map 530/entry 18525/
subtype 0/server 0, MUID 59, `NewQuantity = -1`, `QuantityBought = 1`, S2C
direction, realm routing, and canonical decoding. `SMSG_ITEM_PUSH_RESULT`
remains byte-exact. The complete raw action also exposed the independently
missing Rust `SMSG_CRITERIA_UPDATE`; that achievement-subsystem gap is neither
ignored nor accepted by this vendor fixture. See the flow's README for the
precise scope and reproduction command.

The issue-#112 equipment-set fixture isolates one new-set
`SMSG_EQUIPMENT_SET_ID`. Paired real C++/Rust captures both produced the exact
16-byte `guid=1`, transmog `type=1`, `set_id=8` body on the instance connection,
so the one-packet committed flow is CLEAN with an empty divergence baseline and
no semantic normalization. The separate two-client Rust bot run concurrently
saved an ordinary equipment set and a transmog outfit, required distinct GUIDs
from their shared process-wide namespace, verified both CharacterDB tables after
logout and again through `SMSG_LOAD_EQUIPMENT_SET` after fresh authentication,
and proved cleanup returned both fixture owners to zero rows. See the flow README
for the narrower capture claim and the installed C++ runtime boundary.

The issue-#106 loot gate has an equally narrow comparator for
`SMSG_LOOT_REMOVED`: paired real captures assigned its one reviewed Doctor
Maleficus identity (Creature, realm 1, map 530, entry 21779, subtype/server 0)
different nonzero map-runtime counters. The bot preflight proves that this
entry has exactly one SQL spawn on that map. Only the lower 40 bits of that
Owner counter are omitted. Owner type/realm/map/entry/subtype/server id, the
complete map-530 LootObj GUID including its counter, LootListID 0, canonical
packed encoding, packet direction, and instance-socket routing remain exact.
Zero counters, malformed or non-canonical bodies, every other Creature
identity, and non-Creature owners are never normalized.

The required-flow validator separately decodes the preceding item
`CreateObject`. It requires exactly one map-530 `UpdateType::CreateObject`
block with `TypeID::Item`, all item movement flags clear, the owner-visible
item-30712 value shape, owner and contained-in GUID equal to character 15,
StackCount 1, and the reviewed deterministic zero tail. Its Item GUID must be
the same canonical realm-1/server-0 GUID reported by `ItemPushResult` and the
one `InvSlots` child; the push must report quantity 1 and the observed keyring
slot 106.

The same gate has one separate, fail-closed `SMSG_UPDATE_OBJECT` comparator for
the one-player `UpdateType::Values` inventory delta observed after the claim.
Two independent real C++ captures produced the same 51-byte body: before the
otherwise identical `ActivePlayerData::InvSlots` update, `UnitData` contained
only its five-byte mask fragment `08 00 10 00 00` (parent bit 116 for the Power
arrays), with no child bit and no payload. Rust produced the corresponding
46-byte body without that empty parent. This follows the throttled
`Player::Regenerate` path in `Player.cpp` (`DoWithSuppressingObjectUpdates`,
then `UnitData::Power::ClearChanged`): the parent can remain accumulated until
the next object-update flush even though its child was cleared.

Normalization is allowed only for S2C opcode `0x27CB` when both bodies contain
exactly one VALUES block for the same canonical Player GUID and map, no
destroy/out-of-range section, and exactly the same single canonical non-empty
Item GUID at the same `InvSlots` slot after removing that C++-only fragment.
Every child or payload under Unit parent 116, every other Unit/ActivePlayer
mask, GUID, slot, value, malformed or non-canonical encoding, opcode,
direction, and socket route remains strict; CreateObject and every other
UpdateObject shape receive no normalization. This exception records proven
accumulated-change-mask/capture-cadence noise only. It is not evidence of power
regeneration—or any other gameplay—parity.

`--ignore-opcode` is repeatable, direction-required, and applied symmetrically
after boundary selection. It is fail-closed to the reviewed periodic allowlist
(`s2c:0x2DD2 SMSG_TIME_SYNC_REQUEST`, its causally paired
`c2s:0x3A3D CMSG_TIME_SYNC_RESPONSE`, and `s2c:0x2DD4
SMSG_ON_MONSTER_MOVE`) and cannot overlap either action boundary. Filtering
only the request would leave its client-time-bearing response in the strict
window, so a flow that answers time sync must exclude that reviewed pair. The
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

The recording wrappers below still target **PM2**; the current local deployment
uses systemd. These remain reproduction procedures for their matching PM2 fixture
environment, not commands to run unchanged against an unrelated live service.
Confirm the approved service identities, isolation and fixture scope before use.
The systemd `tools/qa-runtime.sh` provides bounded runtime smoke, not equivalent
fresh-capture provenance. `pr-preflight.sh` and its `capture`/`qa-loot-race`
subcommands were retired; do not wait for or invoke that former gate.

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
and rejects a capture if the process PID/start-time/restart counter or redacted
PM2 launch-profile hash changes. Set
`RUST_CAPTURE_EXEC` to an absolute canonical path when a feature-branch binary
must be captured and set `RUST_CAPTURE_EXEC_SHA256` to that file's 64-hex
SHA-256. The script verifies the source immediately before launch and checks
that both `/proc/<pid>/exe` and its bytes match the pinned path and digest before
and after the interactive capture; a provenance mismatch fails closed. The
snapshot's original executable is still restored on normal exit or a caught
termination signal, but readiness requires both distinct world/instance
listeners to belong to one accredited PID that is the PM2 entry itself or its
verified descendant. The stopped-world accreditation used during C++ swaps
waits at most 30 seconds and stable startup waits at most 180 seconds by
default; operators may override the bounded
`CAPTURE_WORLD_STOP_TIMEOUT_SECONDS` and
`CAPTURE_WORLD_READY_TIMEOUT_SECONDS` values from 1 and 3, respectively,
through 3600 seconds. The automated two-session QA gives the wrapper both
budgets plus a separate 120-second pre-readiness margin for offline-character,
tree-termination, SQL, PM2, and accreditation work.
Before any fixture restoration, cleanup removes the PM2
entry and verifies the recorded entry, listener, and descendant process-tree
identities plus both listeners are absent; a stubborn tree is terminated and
rechecked. The dump parser accepts only a flat set of regular, non-symlink
`.meta`/`.bin` pairs with exact metadata keys, canonical filenames/opcodes and
contiguous global sequence numbers. Extras, orphans, subdirectories, and
symlinks fail closed.

Each successful wrapper also publishes one completed RAW manifest (schema v3): C++ writes
`cpp.capture-manifest.json` next to `cpp.pkt`, while Rust writes
`rust.capture-manifest.json` inside the dump directory. Publication happens
without overwriting an existing generation and only after service/fixture
cleanup succeeds and the normal runtime is healthy again. The manifest records
repository HEADs and worktree state, expected/source/live executable identity,
PM2 entrypoint plus listener PID/start-time identity and restart count, the
redacted PM2 profile hash, and a capture-config digest. Direct Rust profiles
may have the same PM2-entry and listener PID;
legacy C++ uses a non-`exec` shell entrypoint, so its unique listener PID must
be a live descendant. The capture-config digest is calculated from an ordered
allowlist of capture-relevant settings; credential-like values are
replaced with a fixed presence marker before hashing, so no password, token, or
complete secret-bearing configuration is hashed or stored. C++ additionally
derives the effective config selected by PM2 argv or its pinned wrapper and
requires it to be the exact canonical `CPP_CONF`; caller declaration alone is
not evidence.

`loot-single-item-claim` cannot opt out of its versioned fixture guard: both
C++ and Rust wrappers require guard/acknowledgement, a pinned
`WOW_BOT_EXEC`/SHA-256, and a fresh absolute `WOW_BOT_REPORT`. The report must
prove the exact successful TESTBOT2/character-15 Doctor-21779/spawn-1117/item-
30712 single-item flow before schema-v3 evidence can publish. Rust guards use
both `RUST_CAPTURE_LOOT_FIXTURE_GUARD=1` and
`RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1`, plus an unused absolute
`WOW_BOT_FIXTURE_JOURNAL` path in a private directory. `loot-single-item-claim` retains its
temporary Doctor-health fixture. `loot-two-session-atomic-race` instead checks
the pinned entry-2846/loot-2278 prerequisites, temporarily installs shared QA
chest spawn `9106001` with item 38 and 10 copper, and removes/restores only
unchanged fixture-owned values—including its exact generated respawn row—before
restarting the original PM2 profile. Any external drift fails visibly and
leaves the normal world stopped for manual inspection.

Both guarded Rust flows also require a pinned `WOW_BOT_EXEC`/SHA-256 and a
fresh absolute `WOW_BOT_REPORT` path before any service mutation. For
`loot-two-session-atomic-race`, the operator runs the two clients and
produces that report while the wrapper is waiting at its prompt (the old automated
preflight is no longer available). Publication is
fail-closed: only the exact TESTBOT2/TESTBOT3 success contract (same live target
counter/list ID, one item winner, `10`/`0` money notifications, exact database
deltas, and relog proof) can produce a completed race dump. Cleanup and normal
PM2 restoration still run when the report is absent or invalid. A direct/manual
invocation must arrange the same pinned bot report; pressing ENTER without that
evidence intentionally restores the runtime without publishing.
A successful race generation retains `race.bot-report.json` inside the atomic
Rust artifact and records its final path/SHA-256 plus non-null fixture/bot
contracts in the manifest; the operator may then retire the private source
copy without orphaning the provenance record.

The bot writes a mode-0600 recovery journal before its first character/fixture
mutation, removes it only after verified restoration, and atomically creates
`${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete`. The capture wrapper removes that
marker only after the original PM2 profile is healthy again. If the wrapper
fails before it exposes the flow prompt, both journal and marker must still be
absent and the original world can be restored without inventing a cleanup
receipt. Once the prompt is exposed, a pending journal, missing marker, caught
TERM, state/metadata drift, or cleanup error leaves the normal world stopped
and retains recovery evidence; cleanup failure is the reported exit status even
when the capture already failed. `SIGKILL` cannot be trapped, so an operator
must recover a retained journal before restarting the normal world.

The shared-chest capture guard claims no spawn that has pool, event,
linked-respawn, `gameobject_addon`, `gameobject_overrides`, or `spawn_group`
metadata. It pins every `gameobject_template_addon` field used by the fixture
and validates the complete spawn—including `state`—before any cleanup write.
Neither the script nor the bot may normalize that state merely to hide drift.

## Flows and the golden fixtures

A *flow* pins a golden capture so a milestone gets a regression gate. Layout
(committed under `flows/<name>/`):

```text
flows/login/cpp.pkt                    # C++ PKT 3.1 golden
flows/login/rust/                      # reference Rust dump (.bin/.meta)
flows/login/expected-divergences.json  # accepted-divergence baseline
flows/login/flow.json                  # description + directions
flows/<required>/requirement.json      # attestation + exact wire/payload contract
flows/<required>/capture-lineage.json  # RAW/import/output hashes + exact selection
flows/<required>/capture-provenance/   # exact reviewed C++ and Rust RAW manifests
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

The issue-#7 `cuf-login-order` flow is a newer focused live pair with one
non-empty profile. It pins C++'s exact post-add sequence
`InitWorldStates → LoadCufProfiles → AuraUpdate → PhaseShiftChange`, requires
the CUF and final phase-shift bodies to remain byte-identical, and retains the
separately tracked dynamic world-state/aura value differences in its explicit
baseline. See `flows/cuf-login-order/README.md` for provenance and scope.

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

`import` trims both captures at the boundary opcode and prepares `cpp.pkt`,
`rust/`, the baseline, retained RAW manifests, and lineage as one complete
generation. It publishes that generation with one atomic directory rename (or
an atomic directory exchange when replacing a flow), so an interrupted import
leaves the previously installed generation intact. Existing metadata is copied
only from the explicit `README.md`, `flow.json`, and `requirement.json`
allowlist; unknown entries or symlinks fail closed.

## Adding a flow

1. Record a `cpp.pkt` (C++ `PacketLogFile`) and a `rust/` dump with their
   completed RAW manifests (scripts above).
2. `cargo run -p capture-diff -- import <name> --cpp <pkt> --rust <dir> [--cpp-manifest <json>] [--rust-manifest <json>] [--from-opcode c2s:0xNNNN] [--until-opcode s2c:0xNNNN] [--ignore-opcode s2c:0xNNNN] [--direction s2c]`
   — validates the RAW artifacts and process provenance, installs the whole
   fixture generation atomically, and pins the baseline plus lineage.
3. Optionally edit `flows/<name>/flow.json` (`description` / `directions`).
4. Inspect `capture-provenance/*.capture-manifest.json` and
   `capture-lineage.json`, then run `cargo test -p capture-diff`.

For a flow required by an issue before its real pair exists, add only
`requirement.json`, `flow.json`, and a capture runbook. Keep its status
`awaiting-real-captures`; do not add placeholder `cpp.pkt`, `rust/`, or an
accepted divergence. After a strict real import succeeds, inspect provenance,
set the requirement to `ready` as an explicit operator attestation, remove its
stale `blocked_reason`, and run `verify-required <name>`.
