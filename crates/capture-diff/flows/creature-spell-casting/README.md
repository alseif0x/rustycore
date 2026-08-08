# Required capture: creature spell casting

This directory is the fail-closed live acceptance contract for issue #26. It
does not contain a packet capture and its requirement remains
`awaiting-real-captures`. Synthetic semantic tests are parser/comparator
coverage only; they are not evidence that either world server cast a spell.

## Guarded fixture

The shared capture guard verifies the installed world snapshot before either
world starts:

- `creature_template` entry `22378` is the one `Cabal Interrogator`, with
  `AIName=SmartAI` and an empty `ScriptName`;
- exactly one spawn exists for the entry: guid `78686`, map `530`, at the
  coordinates pinned in `fixture/fixture.json`, with no pool, event, linked
  respawn, addon, or spawn-group augmentation;
- exactly one `creature_template_spell` row exists: slot `0`, spell `15691`
  (`Eviscerate`);
- the original SmartAI rows and the absence of a persisted creature respawn
  are included in the database snapshot hash;
- character `15` has neither ghost aura `8326` in `character_aura` nor any
  associated `character_aura_effect` rows. A stale ghost aura changes C++
  server-side visibility and can hide the living Cabal despite an otherwise
  clean character row, corpse state, and creature respawn state.

With both PM2 worlds stopped, both listener ports absent, and every character
offline, the wrapper validates the exact 87-column `characters` schema and
writes a private mode-0600 recovery journal before either mutation. It
CAS-updates `creature_template.AIName` from `SmartAI` to `CombatAI`, and moves
character `15` (`TESTBOT2@bot.local`) to the pinned safe login position with
health `50000`. Both character transitions use a hash of the complete
87-column row. `CombatAI` reads the already-installed spell slot; no spawn or
spell row is created.

Cleanup restores the exact 73-field `CHAR_UPD_CHARACTER` projection used by
the C++ and Rust persistence paths. Its atomic predicate contains both the
durably recorded complete-row hash and a hash of the other 14 columns, then it
proves the original 87-column row was reproduced. It also CAS-restores
`SmartAI`, proves the original database snapshot, and replaces the journal
with a hash-bound cleanup marker before normal Rust may start. Any external
drift leaves both worlds stopped and retains recovery evidence.

An abrupt authenticated-socket shutdown can leave stock C++'s
`characters.online=1` marker behind while its `WorldSession` waits to expire.
The normal wrapper handles only that marker after both PM2 world entries are
stopped with pid `0` and both listeners are absent, but before the global
all-characters-offline gate and post-login snapshot. If every character is
already offline it performs no write. Otherwise it requires the sole online
row to be character `15`, account `9`, with `online=1`; the hypothetical full
87-column row obtained by changing only `online` to `0` must equal the
journaled pre-login hash, the 14-column immutable hash must still match, and
there must be no corpse or persisted spell `8326` aura/effect. One atomic CAS
then changes only `online` from `1` to `0` and must affect exactly one row.
Every other online shape or failed predicate leaves both worlds stopped and
the journal retained. The ordinary global offline gate still runs afterward;
this is not permission to repair health, position, death, aura, or any other
gameplay state, and explicit recovery remains fail-closed.

Both the shell guard before mutation and the bot immediately before login
independently reject persisted spell `8326` aura/effect state. After the
capture world stops, the shell durably records the post-login core row and
checks the same narrow ghost state again. Publication requires both checks;
if the live session persisted ghost state, the capture is not accredited and
the captured journal is retained for explicit recovery and manual review.

Spell `15691` is an instant, zero-speed, zero-cost victim-targeted school
damage spell. The installed DB2 rows and legacy C++ producer imply exact START
flags `0x00000002`, GO flags `0x00000100`, and CastFlagsEx `0`, with no
projectile/ammo, pending/triggered, immunity, trajectory, power, rune, heal
prediction, or full combat-log optional. `StartRecoveryTime=1000` prevents the
GO `NO_GCD` flag. The bot sends one heartbeat to the pinned pull position,
facing away from the creature, and accepts only an adjacent START/GO pair with
character `15` as the sole hit target, no miss status, and advanced combat
logging disabled. A miss fails the attempt; do not alter the spell or
character database to force the roll.

## Recording the action

Build and review `wow-test-bot`, then pin its absolute executable path and
SHA-256 alongside the server executable. Use a fresh bot report path inside a
private mode-0700 directory per side. The database config must be the one used
by that runtime. For C++:

```bash
FIXTURE_DIR="${TMPDIR:-/tmp}/rustycore-creature-spell-cpp-$(id -u)"
install -d -m 700 "$FIXTURE_DIR"
CPP_EXEC=/absolute/path/to/the/reviewed/worldserver
CPP_CAPTURE_EXEC="$CPP_EXEC" \
CPP_CAPTURE_EXEC_SHA256="$(sha256sum "$CPP_EXEC" | awk '{print $1}')" \
CPP_CAPTURE_DB_CONF=/absolute/path/to/worldserver.conf \
WOW_BOT_EXEC=/absolute/path/to/the/reviewed/wow-test-bot \
WOW_BOT_EXEC_SHA256="$(sha256sum /absolute/path/to/the/reviewed/wow-test-bot | awk '{print $1}')" \
WOW_BOT_REPORT="$FIXTURE_DIR/wow-test-bot-report.json" \
CREATURE_SPELL_CAPTURE_ACK_FIXTURE_MUTATION=1 \
CREATURE_SPELL_FIXTURE_JOURNAL="$FIXTURE_DIR/fixture.journal" \
crates/capture-diff/scripts/capture-cpp.sh creature-spell-casting
```

For Rust:

```bash
FIXTURE_DIR="${TMPDIR:-/tmp}/rustycore-creature-spell-rust-$(id -u)"
install -d -m 700 "$FIXTURE_DIR"
RUST_EXEC=/absolute/path/to/the/reviewed/world-server
RUST_CAPTURE_EXEC="$RUST_EXEC" \
RUST_CAPTURE_EXEC_SHA256="$(sha256sum "$RUST_EXEC" | awk '{print $1}')" \
RUST_CAPTURE_DB_CONF=/absolute/path/to/worldserver.conf \
RUST_CAPTURE_EFFECTIVE_CONFIG=/absolute/path/to/the/effective/worldserver.conf \
WOW_BOT_EXEC=/absolute/path/to/the/reviewed/wow-test-bot \
WOW_BOT_EXEC_SHA256="$(sha256sum /absolute/path/to/the/reviewed/wow-test-bot | awk '{print $1}')" \
WOW_BOT_REPORT="$FIXTURE_DIR/wow-test-bot-report.json" \
CREATURE_SPELL_CAPTURE_ACK_FIXTURE_MUTATION=1 \
CREATURE_SPELL_FIXTURE_JOURNAL="$FIXTURE_DIR/fixture.journal" \
crates/capture-diff/scripts/capture-rust.sh creature-spell-casting
```

When the wrapper pauses, leave it running and execute the exact command it
prints in a second terminal. Its shape is:

```bash
WOW_BOT_REPORT=/absolute/private/wow-test-bot-report.json \
  /absolute/path/to/the/reviewed/wow-test-bot \
  --creature-spell-capture \
  --single TESTBOT2@bot.local \
  --creature-spell-fixture-manifest \
  /absolute/path/to/crates/capture-diff/flows/creature-spell-casting/fixture/fixture.json
```

Return to the wrapper and press Enter only after the bot reports success. The
wrapper independently verifies the pinned bot executable, its fresh report,
the exact START/GO bodies, and the immediate shutdown of both authenticated
sockets without a combat `CMSG_LOGOUT_REQUEST` before accepting the capture.
Press Enter promptly so the wrapper stops the capture world, proves the exact
offline post-login row (including the bounded stale-online-marker CAS above
when and only when necessary), and restores the fixture before another combat hit.
Repeat
the same bot-directed action from the restored database snapshot for the other
side. A normal game client or a manually selected character is not accredited
evidence for this flow.

If a wrapper is killed before its EXIT cleanup, stop both worlds and verify
ports `8085/8086` are absent before running:

```bash
CREATURE_SPELL_FIXTURE_JOURNAL=/absolute/private/fixture.journal \
  crates/capture-diff/scripts/recover-creature-spell-casting-fixture.sh
```

The recovery command deliberately leaves services stopped. An `applied`
journal has no durable post-login row yet, so automatic recovery is permitted
only while the complete character row still equals the deterministic pre-login
hash. If the world persisted any unjournaled live-session change before the
crash, recovery fails closed without a DB write and requires manual review.
After a normal wrapper has durably entered `captured`, restoration instead
requires the exact recorded post-login row hash. Successful recovery retains
the validated cleanup marker for review; rerun it with `--consume-marker`
before explicitly starting the normal Rust world. For a killed C++ wrapper,
restore its separately retained config backup before restarting any service.

## Promotion

Derive both sides only from reviewed RAW artifacts and schema-v3 manifests:

```bash
cargo run -p capture-diff -- import creature-spell-casting \
  --cpp target/captures/creature-spell-casting/cpp.pkt \
  --rust target/captures/creature-spell-casting/rust \
  --cpp-manifest target/captures/creature-spell-casting/cpp.capture-manifest.json \
  --rust-manifest target/captures/creature-spell-casting/rust/rust.capture-manifest.json \
  --from-opcode s2c:0x2C37 \
  --until-opcode s2c:0x2C36 \
  --direction s2c \
  --strict
```

Review both RAW captures, manifests, fixture hashes, the exact selected packet
bodies, and generated `capture-lineage.json`. Only then change the requirement
to `ready`, remove `blocked_reason`, and run:

```bash
cargo run -p capture-diff -- verify-required creature-spell-casting
```
