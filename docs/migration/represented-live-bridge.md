# Represented -> Live Bridge Convention

Issue #19 defines the minimum convention for converting represented handler
work into live runtime mutations without hiding ownership or recording false
successes.

## Contract

1. The packet handler owns decoding and the exact C++ validation order.
2. After validation it constructs one typed intent. An invalid packet never
   reaches the bridge.
3. The bridge resolves the authoritative live owner and applies the mutation
   and its immediate packet effects there.
4. The bridge returns an explicit applied/rejected outcome.
5. Structured telemetry and test evidence are emitted only after an applied
   outcome. Missing owners, failed locks, and other rejections must not look
   like completed represented work. Production must not retain an unbounded
   client-controlled intent history.
6. The handler must not duplicate the same mutation or packet send.

“One intent per accepted handler event” is an invocation rule, not an
idempotence promise. If C++ repeats a response for repeated valid input, the
Rust bridge repeats it too.

The intent enum is deliberately `Clone`, not `Copy`, so later session-owned
intents may carry strings or vectors. It is not a global queue and does not
make `WorldSession` the owner of unrelated runtime state.

| Effect | Authoritative owner | Bridge location |
|---|---|---|
| Session/player request with synchronous player mutation | canonical `Player`/`Unit` | `WorldSession::apply_represented_live_intent_like_cpp` |
| Global creature/map tick | canonical or legacy map selected by the runtime ADR | map-owned bridge, not `WorldSession` |
| Persistence | database transaction/save owner | DB writer after successful live mutation |
| Remote-session delivery | receiving session through `SessionCommand`/visibility gate | sender queues; receiver validates visibility |

Each new variant must cite its C++ function, name the live owner, enumerate
packet routing (realm/instance), cover positive and negative outcomes, and
have capture-diff evidence before being called done.

## First converted example: stand state

`RepresentedLiveIntentLikeCpp::StandStateChanged` converts
`CMSG_STAND_STATE_CHANGE`.

- C++ validation: `src/server/game/Handlers/MiscHandler.cpp:406-420`.
- C++ live mutation: `src/server/game/Entities/Unit/Unit.cpp:9966-9977`.
- C++ packet layout: `src/server/game/Server/Packets/MiscPackets.cpp:328-340`
  and `MiscPackets.h:434-453`.
- C++ connection routing: `src/server/game/Server/Protocol/Opcodes.cpp:2131`.
- Rust handler: `crates/wow-world/src/handlers/misc.rs::handle_stand_state_change`.
- Rust bridge: `crates/wow-world/src/session.rs::apply_represented_stand_state_changed_live_like_cpp`.

The handler accepts only Stand, Sit, Sleep, and Kneel. The bridge mutates
canonical `UnitData::StandState`,
removes `SpellAuraInterruptFlags::Standing` auras only when the resulting state
is standing, and refreshes the canonical visible-aura slot and a clean
canonical-state `ObjectAccessor` snapshot. Aura decisions use effective masks composed in C++
load order from `SpellInterrupts.db2`, official/custom SQL rows that replace an
exact DB2 record ID and then reindex its spell/difficulty relationship, world
`serverside_spell` rows, and the five interrupt-mask mutations ported from
`LoadSpellInfoCorrections`. These masks only hydrate older
missing/zero snapshots. The lookup starts at the difficulty of the canonical
`ManagedMap` that actually owns the Player and follows
`DifficultyEntry::FallbackDifficultyID` like `SpellMgr::GetSpellInfo`; aura and
channel words always come from the same selected row. A present canonical mask
(or nonzero represented mask) remains authoritative, including a known
non-`Standing` value, because it may already contain cast-difficulty-resolved
metadata. It then updates temporary session mirrors, sends
`SMSG_STAND_STATE_UPDATE` on the realm connection (`u32 AnimKitID = 0`, then
`u8 State`), and sends the changed StandState VALUES delta on the instance
connection to self and visible observers. Repeating the current state still
runs supported standing side effects and sends the direct packet, while
omitting an unchanged VALUES delta.

The server-side mask import uses the effective file plus official/custom
`SpellName` base-table collision gate. Locale hydration, full server-side
`SpellInfo`, and wider spell correction parity remain outside this bounded
composition; locale rows do not change the collision ID set.

After the canonical stand mutation and `Standing`-aura removal, the bridge also
classifies canonical `CURRENT_CHANNELED_SPELL` in the same position as C++. If
that slot is exactly `Casting` and its channel mask has `Standing`, C++ would
enter `InterruptNonMeleeSpells(false)`. Rust invokes the canonical equivalent,
interrupting eligible Generic, Autorepeat, and Channeled slots in C++ order,
clears a matching session cast mirror, refreshes `ObjectAccessor`, and records
the interrupted counts in the applied outcome.
Unknown Casting-channel metadata records an `UnknownInterruptMetadata` boundary
instead when the active-map DB2 difficulty/fallback chain cannot resolve a row;
the thin `CurrentSpellRef` does not retain interrupt masks. This avoids both
silently dropping a valid stand request and falsely reporting that a partial
cancellation stopped the spell.

The explicit visibility-registry VALUES fanout is transitional until canonical
`Map::SendObjectUpdates` owns real per-viewer fanout. It mirrors the delta while
the generated-field equivalent queues the in-world Player and preserves the
canonical StandState dirty bit; the represented canonical object-update seam
then captures the complete Player VALUES update before consuming its masks.
Missing session routing therefore does not lose the update or leave a stale
unqueued delta.
Other callers that change stand state (movement, chairs, death, creature addon
loading) are not silently routed through this client-handler intent.

Full `Spell::cancel()` cleanup remains an explicit represented-partial boundary
and is not part of the clean golden below. The represented interruption stops
the server-side current-spell/session-cast continuation, but production player
casts do not yet populate canonical `CurrentSpellSlot`, and the thin
`CurrentSpellRef` does not own the target list, channel update/interrupted/
cast-result packet metadata, remote owned auras, dynamic objects, or
gameobjects needed to reproduce every cancellation side effect. `SpellStore`
now retains every DB2 difficulty, composes the bounded effective interrupt masks
listed above, and implements exact-to-fallback lookup. However, transitional
aura/current-spell records do not yet retain their original cast difficulty
when their resolved masks are absent; that case uses the current canonical map
difficulty. Once full cancellation exists, the typed boundary must become the
C++ aura-then-cancel path.

For interrupted auras, the bridge now removes a matching locally owned Aura
base after unapplying it, matching the local-owner branch of C++
`Unit::RemoveAura`. The thin aura model has no cross-Unit application registry,
so removal of that base from remote targets and their effect/script/proc
lifecycle remains an explicit full-Aura-runtime boundary.

## Validation flow

The local bot supports `--stand-state <0|1|3|8>`, validates the five-byte realm
response, requires a changed state to produce `SMSG_UPDATE_OBJECT` on the active
instance connection, and rejects stand side effects on the separate realm
connection. For the golden action use one request (`--stand-state 1`). After
the realm ACK, the bot drains both sockets to a quiet period and sends a
deterministic `CMSG_PING` fence, so capture import includes deferred VALUES/aura
fanout rather than ending at the ACK:

```bash
cargo run -p capture-diff -- import stand-state \
  --cpp target/captures/stand-state/cpp.pkt \
  --rust target/captures/stand-state/rust \
  --from-opcode c2s:0x318C \
  --until-opcode c2s:0x3768 \
  --ignore-opcode s2c:0x2DD4 \
  --direction both \
  --strict
```

`--strict` refuses to write a fixture or accepted-divergence baseline unless
the isolated C++ and Rust actions are clean. The sole filter removes ambient
`SMSG_ON_MONSTER_MOVE` from the independent global creature clock on both
sides. Opcode filters are fail-closed to the reviewed periodic allowlist and
cannot overlap an action boundary, so this filter cannot hide the stand
request, realm ACK, instance VALUES/aura fanout, connection routing, or ping
fence.

### Captured result (2026-07-12)

The installed Rust candidate passed the bot smoke with requested and confirmed
stand states both equal to `[1]`, distinct realm/instance sockets, the expected
VALUES update, and a successful ping fence. The imported fixture under
`crates/capture-diff/flows/stand-state` contains this exact bounded sequence:

| Order | Direction | Connection | Opcode | Role |
|---:|---|---:|---|---|
| 1 | C2S | 1 | `0x318C CMSG_STAND_STATE_CHANGE` | Sit request |
| 2 | S2C | 0 | `0x271C SMSG_STAND_STATE_UPDATE` | realm ACK |
| 3 | S2C | 1 | `0x27CB SMSG_UPDATE_OBJECT` | instance StandState VALUES delta |
| 4 | C2S | 1 | `0x3768 CMSG_PING` | instance post-action fence |

Strict C++/Rust comparison reported `4 matched`, zero value or routing
differences, zero missing packets, and zero extras: **CLEAN**. The
`s2c:0x2DD4` exclusion was applied symmetrically before comparison and removes
only unrelated global-creature movement.
