# Player cast and death lifecycle QA (#43, #589)

The integrated bot uses the same ordinary login, packet capture, encryption and
two-socket drain as other scenarios. This mode performs no account provisioning,
fixture SQL, service restart or data overlay installation. Login, scripted
gameplay and normal logout still mutate live server state: use only authorized
accounts and runtime targets.

After building the bot at the candidate source, run against either Classic or
Rust with its existing private configuration and an explicit action plan:

```sh
tools/wow-test-bot/target/debug/wow-test-bot \
  --config /absolute/private/bot-config.json --single TESTBOT1@bot.local \
  --cast-lifecycle-plan /absolute/private/cast-plan.json \
  --report /absolute/private/cast-report.json
```

Use the actual binary location selected by `CARGO_TARGET_DIR`. Do not combine this
mode with `--login-only`, other post-login modes, or `--ensure-test-accounts`.
`WOW_BOT_CAST_LIFECYCLE_PLAN` also selects the mode. Retain the existing capture
configuration and a distinct report/capture destination for each paired run.

`cast-lifecycle-plan.example.json` describes the explicitly synthetic #587
30798→6197 fixture, not stock gameplay and not evidence of a completed #589 run.
It requires the reviewed effective data, known source spell and matching server
visual. Do not run it blindly against an unrelated character or data set.

## Plan and acceptance

Actions include the existing `cast`, `cancel` and `wait`, plus typed death
requests:

- `release_spirit` takes `check_instance` and sends CMSG_REPOP_REQUEST 0x3526
  as its one MSB-first bit (`true` is `80`).
- `reclaim_corpse` takes `corpse: {"low": ..., "high": ...}` and sends its
  packed GUID. The target handler currently ignores this field, so empty and
  arbitrary GUIDs remain available for negative/ownership cases.
- `spirit_healer_activate` takes a packed `healer` GUID.
- `resurrect_response` takes `offerer` and the exact `response` u32. Response
  `0` accepts; any nonzero value is preserved and declines/times out per C++.
- `self_res` takes a positive int32 `spell_id` and sends exactly four
  little-endian bytes. It must name a spell the logged-in player was offered;
  the legal Shaman Reincarnation request is 21169, not passive 20608.
- `attack_swing` takes a packed `target` GUID and sends CMSG_ATTACK_SWING 0x3255.

The opt-in movement controller owns one transient player mover, the latest
server-seeded position, movement flags, and any explicit fall data. It observes
the authenticated login burst before the post-login drain, so LOGIN_VERIFY_WORLD
and movement controls in that burst are recorded and acknowledged by the same
owner. It retains safe decoded packet observations in the lifecycle report.

Supported server controls are root/unroot, water-walk/land-walk, run/swim speed
changes, near teleport, TransferPending, SuspendToken, LOGIN_VERIFY_WORLD and
NewWorld. Movement flag ACKs carry MovementInfo followed by the server sequence;
speed ACKs append the speed float. Near-teleport ACK carries packed mover,
sequence and client time. WorldPortResponse is empty. TransferPending arrives
on REALM, SuspendToken on INSTANCE, and NewWorld on REALM. The client answers
SuspendToken on INSTANCE with its same little-endian sequence before NewWorld
exists. ResumeToken is validated on INSTANCE and has no client ACK. After
LOGIN_VERIFY_WORLD, the controller sends CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE
once on INSTANCE; after NewWorld it sends the empty world-port response and
then active-mover completion on the same INSTANCE stream because Player
teleport resets the visibility override. The C++ client opcode handler has no
socket restriction, and WorldSocket queues packets from either connection.
Using one TCP stream preserves the required response order across the server's
two-socket arrival interleaving. Time-sync responses also have one owner while
this controller is active.

Only run and swim speed changes are acknowledged. Known flight, hover, walk,
backward-speed, pitch-rate and turn-rate controls fail explicitly until their
exact client ACK behavior is implemented. Near-teleport movement with transport
or vehicle extensions also fails explicitly because this controller has no
contract for those MovementInfo variants.

Plans may use only the closed movement actions `heartbeat`, `start_forward`,
`stop` and `fall_land`. Each supplies finite `x`, `y`, `z` and `orientation`,
with optional `time_ms`. `fall_land` requires `fall` data; any action can provide
fall data to describe a seeded airborne position before a timed landing. The
controller retains root and water-walking flags, rejects forward movement while
rooted, and requires explicit fall data whenever the outgoing MovementInfo has
a falling flag. For example, these typed actions describe an airborne heartbeat
and subsequent landing; the fixture must already place the character at the
matching airborne start position and the coordinates/time must match its
authorized movement path:

```json
[
  {"action":"movement","kind":"heartbeat","time_ms":100,"x":10.0,"y":20.0,"z":8.0,"orientation":0.0,
   "fall":{"fall_time_ms":800,"z_speed":-2.0,"sin_angle":0.0,"cos_angle":1.0,"xy_speed":0.0}},
  {"action":"movement","kind":"fall_land","time_ms":900,"x":10.0,"y":20.0,"z":0.0,"orientation":0.0,
   "fall":{"fall_time_ms":800,"z_speed":-2.0,"sin_angle":0.0,"cos_angle":1.0,"xy_speed":0.0}}
]
```

The movement contract is pinned to `woltk-trinity-legacy` HEAD
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `MovementPackets.cpp:104–179`
defines MovementInfo read order and conditional fall fields; `MovementPackets.h:37–41`
and `.cpp:800–823` define MovementAck; `.cpp:622–648` defines the server
MoveSetSpeed/MoveSetFlag fields; `.cpp:685–702` writes NewWorld's map and
TeleportLocation, with `MovementPackets.h:268–272` defaulting the two unused
TeleportLocation fields to `-1`; and
`.cpp:1005–1017` defines SuspendToken and its response; `.cpp:1019–1026`
defines ResumeToken. `MovementHandler.cpp:238–257`
places NewWorld after the suspend response. `Player.cpp:1434–1474` sends
TransferPending then SuspendToken and resets the visibility override during
teleport. `CharacterHandler.cpp:1087–1090,1229–1240` establishes login position
before death controls. `Unit.cpp:8460–8504,10893–10925,12718–12743` emits the
relevant speed, root and water-walk changes; `UnitDefines.h:384–386` defines
the movement mask cleared before root. `Opcodes.cpp:1663,1711,1727–1747,
1776–1777,1811,2023,2150,2173` routes LOGIN_VERIFY_WORLD to INSTANCE; the
supported root, land-walk, speed, water-walk and near-teleport controls to
INSTANCE; flight, hover, walk, backward-speed, pitch-rate and turn-rate controls
to INSTANCE; NewWorld to REALM; ResumeToken and SuspendToken to INSTANCE; and
TransferPending to REALM. `Opcodes.cpp:1019` registers the empty WorldPortResponse
handler without a socket constraint; `WorldSocket.cpp:363,488–499` tags each
incoming packet with its socket then queues it to the session. The tests check
response routes and keep the ordered world-port and active-mover writes on one
INSTANCE stream.

This is finite client protocol support, not a replicated world or general
physics implementation. The current plan still accepts literal GUIDs for
healers; an observed entry-plus-near-position creature selector is deferred.
Lifecycle reports and bot packet traces do not replace the parent campaign's
required real server captures or live acceptance evidence.

For example, an explicit release followed by a reclaim attempt can be written
as:

```json
{
  "actions": [
    {"action":"release_spirit","check_instance":true},
    {"action":"reclaim_corpse","corpse":{"low":0,"high":0}}
  ],
  "expect": [
    {"event":"packet","connection":"realm","opcode":9860,"body_hex":""}
  ]
}
```

The client-opcode source does not assign these requests to a server connection
type; the scripted requests use the existing instance socket. GUID fields are
wire inputs, not fixture admission checks. Omitted/null cast target preserves
the empty wire target; a unit target is `{"low": ..., "high": ...}`. Active
cancellation sends 0x329F with packed CastID and spell ID. Pending cancellation
sends 0x3182 with an empty body; its IDs document the intended pending request.

The wire contract is pinned to `woltk-trinity-legacy` at
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Opcodes.h:72,545,550,581,590,637`
defines the request IDs; `MiscPackets.cpp:274,279,296`
defines corpse reclaim, one-bit release and response decoding;
`MiscHandler.cpp:416,447-462` deliberately ignores the reclaim GUID and treats
response `0` as acceptance; `NPCPackets.cpp:234` reads the healer GUID;
`SpellPackets.cpp:873` reads the four-byte self-res spell ID and
`SpellHandler.cpp:416-432` applies admission; `CombatPackets.h:35` defines the
packed attack target.

The ordered `expect` list names `prepare`, `start`, `go`, `cast_failed`,
`spell_failure` or `spell_failed_other`. Start/Go require a spell ID and target
(`"empty"` or `{"unit": {"low": ..., "high": ...}}`). Use client IDs for the
caster: preceding SpellPrepare must establish the server ID. An observer with a
known server ID can use `server_cast_id` explicitly instead. Failure before
preparation can reference the original client request directly.

Start/Go can require flags and `metadata` (visual ID, OriginalCastID, cast time and
the exact `remaining_power` rows). Go also supports hit/miss counts and the
full-combat-log bit; failures support their reason. All named cast events must
arrive on INSTANCE, as registered by Classic `Opcodes.cpp`; the same packet seen
on the realm socket fails.

Each cast action needs a distinct client CastID: reusing one is rejected at load
because it would make the SpellPrepare mapping ambiguous. Every observed
SpellPrepare/SpellStart/SpellGo/CastFailed/SpellFailure/SpellFailedOther that
resolves — directly or through an observed SpellPrepare mapping — to an identity
named anywhere in `expect` must be consumed by one of those rules. An unconsumed
plan-bound packet fails the run, so a Go after cancellation, a duplicated
Start/Go, a second SpellPrepare for the same client CastID or an unlisted
SpellPrepare cannot be hidden by an otherwise matched failure. List every
plan-bound packet you expect, including SpellPrepare. Ambient cast traffic for
an unrelated identity is still recorded but does not fail acceptance.

An exact raw packet expectation uses this form:

```json
{"event":"packet","connection":"realm","opcode":9860,"body_hex":""}
```

The supported exact-packet opcode set is limited to cast results
(`SMSG_SPELL_PREPARE`, `SMSG_SPELL_START`, `SMSG_SPELL_GO`, `SMSG_CAST_FAILED`,
`SMSG_SPELL_FAILURE`, `SMSG_SPELL_FAILED_OTHER`), death and corpse packets
(`SMSG_PRE_RESSURECT`, `SMSG_DEATH_RELEASE_LOC`, `SMSG_RESURRECT_REQUEST`,
`SMSG_REATTACH_RESURRECT`, `SMSG_CORPSE_LOCATION`,
`SMSG_CORPSE_RECLAIM_DELAY`, `SMSG_DURABILITY_DAMAGE_DEATH`,
`SMSG_FORCED_DEATH_UPDATE`), healer timing, aura/update, attack and logout
packets needed by this driver. Authentication/control opcodes such as SMSG_AUTH_CHALLENGE,
SMSG_ENTER_ENCRYPTED_MODE and SMSG_CONNECT_TO are rejected as expectations
before login. `body_hex` must be even-length hexadecimal and at most 64 KiB;
the empty string expresses an empty body. Matching checks the explicit
connection, opcode and complete body. Expectations preserve order separately
on each connection. The merged trace records client poll/arrival order across
the two sockets; that interleaving does not claim server enqueue order.

The lifecycle trace begins during the authenticated login burst and continues
through its bounded post-login drain. It stores logical decrypted,
uncompressed opcode/body bytes, route, per-connection sequence and merged
observation sequence. Action boundaries, send attempts and successful local
writes are distinct. A failed write keeps its attempted body and error without
claiming completion. Authentication, encrypted-mode and ConnectTo bodies are
omitted even if encountered later; the trace retains only their opcode, route,
byte count and order. Raw packet equality is packet-level evidence and does not
by itself certify the full death lifecycle or database state.

The default `finish_policy` is `logout`: the bot sends its normal logout request
and still requires an empty realm SMSG_LOGOUT_COMPLETE. Set
`"finish_policy":"disconnect"` only for an intentional disconnect/relogin
scenario. That policy records successful local socket closure and never claims
logout completion or database persistence.

Any packet the spell decoder cannot read exactly — truncated, over-long or with a
structurally wrong optional section — is retained with its `error` and fails the
run; a wrong payload can never be matched. Reports retain ordered decoded events,
raw body hex, hashes, identities, power/target sections and connection,
including unmatched ambient events. Exact raw expectations also fail on an
unmatched duplicate or same-opcode packet on the wrong connection. A process
exit, a decode error or an empty expectation list does not establish a pass.

Use separate plans for instant/timed casts, the 400 ms queue boundary, replacement,
active/pending cancellation and late failure. Script timings alone do not prove
the exact boundary: retain observed preparation and cast duration, and use the
deterministic application tests for 400/401 ms admission. A late-power scenario
needs an independently prepared, authorized source of the resource change.

## Paired caster/observer correlation

A nearby observer does not receive SpellPrepare, so it never learns the client
CastID. `observe_only` collects facts and can never pass: the loader forbids
actions and expectations in that mode, and the evaluator additionally forces
`passed: false` for any `observe_only` evidence. The bot exits unsuccessfully by
design. Collection is not QA acceptance, and an observed server CastID must never
be relabelled as a client CastID.

Reproducible procedure for paired acceptance, using two pre-existing authorized
accounts and a separate report path per session:

1. **Discovery pair.** Start the observer with
   `{"observe_only": true, "timeout_ms": 10000, "actions": [], "expect": []}`,
   then run the caster with its full plan. Both reports are kept; both runs are
   expected to be unsuccessful for the observer.
2. **Read the mapping from the caster only.** Take `client_cast_id` →
   `server_cast_id` from the caster report's `spell_prepare` event. Do not infer
   it from a failure, from packet order or from the observer capture.
3. **Author the observer acceptance plan.** Write the observer's `expect` rules
   with that explicit `server_cast_id` (no `client_cast_id`), the same spell ID,
   target shape, flags and metadata the caster proved.
4. **Acceptance pair.** Re-run the identical caster plan together with the
   observer acceptance plan. Both reports must show `passed: true`, and the
   observer's `event_sequence` values must be present for every rule.
5. **Compare the pair.** Diff the caster and observer `spell_start`/`spell_go`
   `body_sha256`, identities, targets and connection before claiming scoped live
   acceptance.

The server CastID is per cast attempt: step 3's plan is only valid for the run in
step 4, and step 4 must be re-done whenever the caster plan changes. If step 1
produced no SpellPrepare, if a report is truncated, or if any rule is unmatched,
the pair FAILS — there is no partial credit and no manual annotation of a pass.

## Standalone tests and completion limits

The bot is outside the root workspace and has its own manifest:

```sh
CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/home/server/rustycore/target \
  cargo check --manifest-path tools/wow-test-bot/Cargo.toml --locked --bin wow-test-bot
CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/home/server/rustycore/target \
  cargo test --manifest-path tools/wow-test-bot/Cargo.toml \
  --locked --bin wow-test-bot cast_lifecycle
```

Those tests cover the request builders, the 3.4.3 decoders including every
optional SpellCastData section, plan validation, identity correlation, ordering,
socket selection, the unexpected-event rejection and the run-result gate. They
are deterministic byte-level tests: they do not prove any live server behavior.

The default scenario requests normal logout and requires an empty LogoutComplete
on REALM; a logout on the instance socket, one carrying a payload, a missing
confirmation or a transport error fails the run. Intentional disconnect mode
records only local closure and requires a later separately orchestrated login
to establish relog behavior. The driver does not query saved database state,
prove durability, or establish full spell/effect parity. Pair runtime/data
identities and compare the relevant packets before claiming scoped live
acceptance; #587 save/relogin retains its separate persistence checks.
