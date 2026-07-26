# Required capture: chase around a synthetic obstacle

This is the fail-closed live C++/Rust gate for issue #24. It is intentionally
`awaiting-real-captures`: the checked-in Detour assets and semantic contract do
not substitute for running the same connected-client action against both
servers. Do not mark the requirement `ready` until the private runtime fixture,
RAW manifests, strict import, and retained lineage have all been reviewed.

## Fixture contract

`fixture/fixture.json` pins map `1`, world grid `50,26`, the reserved Tender
entry/spawn `15271/9102401`, disposable character `15`, every action coordinate,
and SHA-256/size for:

- `fixture/mmaps/0001.mmap`;
- `fixture/mmaps/00015026.mmtile`.

The MMap pair is generated only by the `wow-recastdetour`
`generate_detour_chase_fixture` example with feature `test-fixtures`. Production
server builds never enable that feature. Each capture side creates its own
fresh private DataDir from the same byte-pinned assets; neither run may replace
files under the normal server DataDir.

The fixture's creature starts at `(-10118.333, 2671.667, 218.49)`. The player
starts one yard south, attacks it, waits for the first accepted player swing,
and then sends one heartbeat to `(-10118.333, 2691.667, 218.49)`. The direct
line crosses the absent centre square:

`x=[-10123.333,-10113.333], y=[2676.667,2686.667]`.

Both capture wrappers now use the shared fail-closed fixture guard. Before the
first database write it records an exact mode-0600 restore journal, creates a
fresh private DataDir containing only the pinned synthetic MMaps plus read-only
links to normal data, and snapshots the complete `characters` row, every
directly character/account-owned auxiliary table, auth account/BNet rows,
Battle.net collections (including the unconditionally rewritten battle-pet
slots), received mail, creature `respawn`, the reserved `world.creature` row,
and every world table that can augment that spawn. The disposable identity
must have no inventory, character pets, Battle.net pets, group, guild, or
corpse ownership, avoiding an ambiguous dependent-object restore. The wrapper
will not restart the normal runtime unless every domain regenerates the
original aggregate DB digest, the private DataDir is removed, and cleanup is
verified. Both RAW manifests must publish the same initial DB digest.

The bot's reserved `--detour-chase-capture` mode validates the committed
manifest/assets, the exact account/character/spawn fixture, and the isolated
heartbeat → MonsterMove → ping/Pong evidence window. Its JSON report is bound
into each RAW manifest and independently revalidated during strict import.
The reported heartbeat and MonsterMove hashes and MonsterMove length must also
match the exact selected RAW packet bodies, so a report from another execution
cannot be paired with the capture.

The disposable TESTBOT2 identity is synthesized only in this explicitly
acknowledged mode; no credential belongs in committed `config.json`. Supply the
password through the runner's protected
`WOW_BOT_PASSWORD_TESTBOT2_BOT_LOCAL` environment and invoke the pinned bot as:

```bash
./wow-test-bot \
  --single TESTBOT2@bot.local \
  --detour-chase-capture \
  --ack-disposable-detour-fixture \
  --detour-fixture-manifest \
    crates/capture-diff/flows/detour-chase-around-obstacle/fixture/fixture.json \
  --report /absolute/fresh/detour-report.json
```

## Exact wire window

After symmetrically removing the periodic time-sync request/response pair and
`SMSG_UPDATE_OBJECT` combat VALUES fanout (whose ordering relative to movement
is outside this Detour path slice), the imported action is exactly:

1. instance `CMSG_MOVE_HEARTBEAT` (`c2s`, connection `1`, `0x3A10`);
2. instance `SMSG_ON_MONSTER_MOVE` (`s2c`, connection `1`, `0x2DD4`);
3. instance `CMSG_PING` (`c2s`, connection `1`, `0x3768`).

Do not ignore `SMSG_ON_MONSTER_MOVE`: it is the evidence. Any other extra
combat or movement packet inside the window is a failed isolation run, not a
reason to widen the ignore list.

`ChaseAroundObstacleV1` fully decodes the C++ packet layout. The persistent
spawn ID remains `9102401`, while C++ `Creature::LoadFromDB` assigns its wire
GUID with `Map::GenerateLowGuid`; each side must therefore report a nonzero
runtime counter, but the counters need not equal the DB ID or each other.
Cross-runtime comparison omits only those process-local creature/spline
allocation IDs. All other GUID bits, float bits, flags, timing,
facing/transport/options, endpoint, and packed deltas remain exact. Each side
is also validated independently: the heartbeat must target the pinned
destination, the movement must belong to the reserved creature and face the
exact player GUID like C++ `ChaseMovementGenerator::Update`, and the
reconstructed compressed path must bend outside and avoid the missing square.

## Promotion

Capture both sides from the same clean DB snapshot with
`DETOUR_CAPTURE_ACK_FIXTURE_MUTATION=1`, pinned server/bot executable paths and
hashes, and an explicit capture config. Then run:

```bash
cargo run -p capture-diff -- import detour-chase-around-obstacle \
  --cpp target/captures/detour-chase-around-obstacle/cpp.pkt \
  --rust target/captures/detour-chase-around-obstacle/rust \
  --cpp-manifest target/captures/detour-chase-around-obstacle/cpp.capture-manifest.json \
  --rust-manifest target/captures/detour-chase-around-obstacle/rust/rust.capture-manifest.json \
  --from-opcode c2s:0x3A10 \
  --until-opcode c2s:0x3768 \
  --ignore-opcode s2c:0x2DD2 \
  --ignore-opcode s2c:0x27CB \
  --ignore-opcode c2s:0x3A3D \
  --direction both \
  --strict
```

Inspect both RAW artifacts, bot reports, manifests, and `capture-lineage.json`.
Then change the requirement to `ready`, remove `blocked_reason`, and run:

```bash
cargo run -p capture-diff -- verify-required detour-chase-around-obstacle
```
