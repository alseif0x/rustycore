# Required capture: creature spell casting

This directory retains issue #26's final bounded P1 wire/lifecycle acceptance.
The reviewed v2 pair records the same guarded action from patched C++ source HEAD
`8cfed90bf1720dbf8b9dc109113c8d7d9173ff6c` and clean RustyCore HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`: exactly one adjacent
`SMSG_SPELL_START`/`SMSG_SPELL_GO` pair for spell `15691`, with an empty strict
divergence baseline. Synthetic semantic tests remain parser/comparator coverage;
the committed RAW provenance and lineage are the live acceptance evidence.
Both manifests pin `creature-spell-casting-shell-fixture-v2` and fixture SHA-256
`3cef5dd6201c88fc85c1c2cb767fec27cd11921ec7ecdc2c7705379fd54e356d`.
The C++ source chain is base HEAD
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` plus patch SHA-256
`ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262`,
yielding the patched HEAD above. That one-file patch only fixes the
`ChrSpecialization` index-container bound required to load the installed DB2
dataset; it does not change creature AI, spells, or their wire output.

## Guarded fixture

The shared capture guard verifies the installed world snapshot before either
world starts:

- `creature_template` entry `22378` is the one `Cabal Interrogator`, with
  `AIName=SmartAI` and an empty `ScriptName`;
- its sole difficulty-0 `creature_template_difficulty` row matches all 25
  pinned fields in `fixture/fixture.json`, including `StaticFlags1..8=0`;
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
writes a private mode-0600 recovery journal before either mutation. One exact
multi-table InnoDB CAS updates `creature_template.AIName` from `SmartAI` to
`CombatAI` and difficulty-0 `StaticFlags1` from `0` to `0x00100000`
(`CREATURE_STATIC_FLAG_NO_MELEE`); it must report two changed rows. The CAS
pins every other field of the difficulty row and rejects mixed states. It then
moves character `15` (`TESTBOT2@bot.local`) to the pinned safe login position
with health `50000`. Both character transitions use a hash of the complete
87-column row. `CombatAI` reads the already-installed spell slot; no spawn,
spell row, or attack-time value is created or changed.

Cleanup restores the exact 73-field `CHAR_UPD_CHARACTER` projection used by
the C++ and Rust persistence paths. Its atomic predicate contains both the
durably recorded complete-row hash and a hash of the other 14 columns, then it
proves the original 87-column row was reproduced. It also atomically
CAS-restores the exact `CombatAI/0x00100000` pair to `SmartAI/0`, proves the
original database snapshot, and replaces the journal with a hash-bound v2
cleanup marker that records the difficulty id and both StaticFlags1 values
before normal Rust may start. Any external drift leaves both worlds stopped
and retains recovery evidence.

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
GO `NO_GCD` flag. The retained bot run sends one heartbeat to the pinned pull
position, facing away from the creature, and accepts only an adjacent START/GO
pair with character `15` as the sole hit target, no miss status, and advanced
combat logging disabled. A miss fails that capture attempt; do not alter the
spell or character database to force the roll. The committed `15691` evidence
is therefore one observed **HIT** sample, not evidence that the spell always
hits. It is the final live recapture of the bounded P1 hardening described below.

The final P1 hardening removes the unconditional-hit assumption from
`SMSG_SPELL_GO`. Rust may publish the atomic START/GO pair only for the bounded
C++ resolution of a physical `DmgClass=MELEE` Creature spell whose sole target
is a Player attacked from behind, whose spell and represented effect mechanics
are all zero, and whose complete Creature/Player source authority proves every
omitted spell-hit source hit-inert. The proof deliberately does not require
every external source to be empty: persisted or login-derived sources may
exist when their exact effects are known not to affect this bounded result,
but the reduced runtime's canonical local aura
application/modifier/visible containers must still be empty until it owns
their full C++ semantics.

The Player side fails closed unless exact persistence and login/zone
reconciliation prove the represented map/area ancestry, guild, skills,
active/rewarded and auto-push quests, glyphs, active-specialization traits,
pets and battle-pet slots, FFA/PvP/war-mode state, SpellArea and
outdoor/battlefield sources, and script, legacy/all-rank and SpellLinked hook
sets. Both valid SpellLinked hooks and the absolute trigger IDs retained from
rejected loader rows block a candidate; a loader error is not treated as proof
that no hook exists.

The final live path corrects the external-ID WDC4 offsets used by
`AreaTable`: Shattrath area `3697` resolves to map `530` and Terokkar zone
`3519` like C++. The OutdoorPvPTF source is then admitted only when effective
spell `33377` contains exactly hit-inert XP-percentage and outgoing-damage
auras and no represented runtime hook can alter them. Effective
`ChrSpecialization` hotfix rows are loaded before active-trait authority is
evaluated. The four Auchindoun dungeon zone IDs remain fail-closed because
this authority does not model C++'s `(Map*, zone)` registration key.

A Creature-owned uniform roll in `0..=9_999` resolves `MISS` below `500` (the
base 5% miss chance) and `HIT` otherwise. The temporary
`CREATURE_STATIC_FLAG_NO_MELEE` disables only automatic melee swings: it does
not make the creature passive, suppress threat/chase, change attack timers, or
replace CombatAI's ordinary EventMap scheduler. This prevents an auto-melee
outcome/proc branch from consuming the Creature RNG stream before the due
EventMap cast. CombatAI's local order remains cast then repeat schedule, with
the due EventMap slot cleared before the cast; hit is rolled before the
cooldown draw, and `NO_ATTACK_MISS` consumes exactly one hit roll before
forcing `HIT`. An accepted `HIT` is published and then tombstones
before repeat scheduling because C++'s launch phase consumes an unconditional
critical roll plus possible effect-value draws that this slice omits; a `MISS`
does not enter those target-effect draws and may still schedule from the known
stream. Spell, melee and movement randomness share one fail-closed Creature
tombstone. An unrepresentable random branch therefore
blocks later random-dependent work; specifically, a valid melee swing that
reaches the unrepresented C++ damage/outcome/proc calculation tombstones the
Creature and emits no fabricated damage or melee wire. C++ uses one
process-global RNG while Rust uses one RNG per Creature, so the represented
claim is distribution plus local causal draw order, not an identical global
sequence or cross-Creature interleaving. This resolution selects only GO's
hit/miss topology; spell damage and effects remain outside scope. The v2
recapture and strict verifier close this bounded result only, not the full
Spell effect or combat pipeline.

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

The final v2 import is strict-CLEAN (2/2 packets, zero divergences) with these
retained identities:

- C++ RAW PKT SHA-256 (73,730 bytes / 183 packets):
  `b52cc8ba962160be63286e72eb7611c6282b0cdc3a1cee0082fc6d6d7bf2c7b9`;
- Rust RAW dump tree SHA-256 (113 packets):
  `9aee309d9ffb2e2e1e5a33167c228ccaa8d1634d917efd026e1a525f2a5db94a`;
- C++ and Rust manifest SHA-256:
  `d40e3615b3337a26a3c4d4e380dc665c23719133ec0b3c7a05febdfd640e849d`
  and `3c942209db52f9f36b3d661477f0cad766e7e2ee49cf1fc97d68a74d996f0da0`;
- reviewed C++ and Rust executable SHA-256:
  `9969ec0fce3f2d34974a3fddedd8836bee2367c1b5b1b4c86130c5a3c07d7de6`
  and `2027d8d8a2ecfb2e4f5baf3f1374f1c1d7e3277e28a4ad2e906ee629f83152a3`;
- pinned bot executable SHA-256:
  `099d98144e89890d331759d693dc617c7016d1b4f988ea5033bf80662f3a4ffb`;
- C++ and Rust bot-report SHA-256:
  `fdf1da6266c041b3ee880bd6268c3d24fedb8a85ea23e3655446b98796ac2b34`
  and `1aaf1f369664b2c68ba07ec43a63adfe88b0001a20c833e5638222d282d38c74`;
- filtered C++ PKT SHA-256:
  `a6b32206e3277e455e25f6aa8e491606aa5cd9449e2bf24245ea9dd5db79d932`;
- normalized Rust tree SHA-256:
  `cc8d53b06c2727c95990eda80fd095a1a7e390da0af16981429d07176e0c003b`;
- `capture-lineage.json` file SHA-256:
  `f443539e7857ac27dfb2029012f1e889d92ed27a224f89f7a6247f9510f0479d`.

The requirement is `ready`; verify its exact provenance, packet shape, empty
baseline, hashes, topology/order, and correlated payload semantics with:

```bash
cargo run -p capture-diff -- verify-required creature-spell-casting
```
