# RustyCore — Defects in EXISTING (already-developed) code

**Date:** 2026-07-22 · **Base:** `3.4.3` @ `55719eb4` plus issue #20 closeout QA.

This is the adversarial audit the user asked for: **not what's missing, but what's wrong
in what already exists.** An 8-agent parallel pass tried to *break* the capabilities STATE.md
labels WORKS, contrasting each against C++ (`/home/server/woltk-trinity-legacy`).

**Reliability note:** these are agent findings; each carries `file:line` + a C++ ref, but
they are **leads to verify, not proven verdicts**. Two are explicitly contested between
agents (marked ⚠VERIFY). Signedness divergences (i32 vs u32) are rated **LOW** because they
only differ for values >2³¹ that never occur in normal play (stack counts, durability) —
identical bytes otherwise. Severity reflects my judgment after that filter.

Headline: the scoped **D-C1…D-C9 CRIT integrity track is closed**. The HIGH/MED defects below
remain real; "sends the packet and mutates DB" still does not imply full gameplay parity.

## Later verified open findings

- **2026-09-05, #578 save-owner read — save projection and writeback debt.**
  Verified on `b813d262`: `current_player_save_to_db_snapshot_like_cpp` uses
  Session's staged level/map, although C++ `Player.cpp:19480-19514` reads Player
  directly and substitutes only the persisted teleport destination. Rust also
  projects dead health differently by residence: active reads force zero when
  `is_alive()` is false, while detached reads clamp health to max without that
  gate; C++ writes `GetHealth()` (`19557`). The existing
  `sync_session_from_save_to_db_snapshot_like_cpp` then reapplies position, level,
  XP, money and health, including derived side effects, before the save request.
  The single-owner read refactor preserves these rules and does not retire this
  writeback bridge. Identity migration and separation of save-only destination
  from runtime mutation remain explicit #578 work, not approved parity or a
  deferred #153 exception. No reproduced live-client failure is asserted.
  **Local correction after `720b2519`:** the full-save recording-port regression
  reproduced relocation to the pending near destination even on definite
  rollback. The writeback method is now deleted; the request uses the captured
  header without replaying setters. Applied/Failed/Unknown outcomes are covered
  by the new regression. The staged level/map and residence-specific health
  projection findings remain open, as does live save/teleport acceptance.

- **2026-09-05, #578 talent-reset cost ownership — arithmetic boundary discrepancy.**
  Verified on `95cb0a34`: Rust's `next_reset_talents_cost_like_cpp` uses
  saturating time subtraction and fee addition, and a widened signed monthly
  reduction. C++ `Player.cpp:3472-3503` uses unsigned subtraction followed by
  signed narrowing. Normal reset history follows the same schedule, but future
  reset timestamps and extreme stored costs are not proven equivalent. The
  ownership move preserves Rust arithmetic; reconciling abnormal persisted
  values requires a separate behavior analysis, not an unannounced refactor
  change. No live-client failure is asserted.

- **2026-09-05, #578 talent-tab extraction — login applies extra tab/class gates.**
  On pre-slice `194f9d1b`, `load_represented_talent_row_like_cpp` validates a
  TalentTab row and class mask for both login and learning. C++
  `Player.cpp:26036-26058` applies these gates in `LearnTalent`, whereas
  `_LoadTalents` (`26623-26633`) delegates directly to `AddTalent`
  (`2644-2692`), which does not perform a tab/class lookup. The Rust login
  filtering is preserved by the catalog refactor, not claimed as C++ parity.
  Any behavior change needs separate analysis of persisted invalid rows and
  client/runtime effects; no observed client failure is asserted here.

- **2026-09-05, #578 glyph catalog extraction — represented glyph loading differs from C++.**
  Verified on pre-slice `b4d407b9`: `load_represented_glyph_row_like_cpp` in
  `crates/wow-world/src/session/mod.rs` skips catalog validation for glyph ID zero
  and writes `glyph_groups[talent_group][glyph_slot]`. C++ `Player.cpp:26573-26598`
  checks `sGlyphPropertiesStore.LookupEntry(glyphId)` even for zero and calls
  `SetGlyph`; `Player.cpp:25477-25481` writes to `GetActiveTalentGroup()`.
  The represented zero-row clearing and row-selected group remain unchanged in
  this ownership refactor. The active/detached borrowed-catalog regression retains
  zero clearing explicitly. Whether the legacy group selection is itself a defect
  needs separate client/persistence evidence before changing either policy. This
  is a verified source discrepancy, not a claim of a reproduced client failure.

- **2026-09-05, #578 catalog extraction — HotfixConnect uses the primary socket.**
  Confirmed against pre-slice `13c984a6`: `handle_hotfix_request` in
  `crates/wow-world/src/handlers/character/account.rs` calls generic `send_packet`, which
  writes to the primary channel (`session/mod.rs`). `wow-session/src/lib.rs`
  `poll_instance_link` replaces that channel with the instance writer after ConnectTo.
  C++ `Opcodes.cpp:1566` routes `SMSG_HOTFIX_CONNECT` exclusively over Realm.
  Before ConnectTo the primary is Realm and delivery agrees. The new shared-catalog
  dispatch test reproduces primary delivery with a parked Realm channel, including an
  empty response. This behavior is deliberately preserved by the structural extraction;
  a separate response-routing correction needs byte/routing regression and capture
  evidence. No live client failure or affected-client frequency is claimed.

## Later verified Rust-port repairs

- **2026-09-05, #578 optimized runtime QA — Map insertion vanished in release.**
  `Map::insert_map_object_record` performed `entity_world.insert(record)` inside `debug_assert!`.
  With debug assertions disabled, the record was never inserted; the derived indexes could still
  be updated and a Player lifetime could claim Active residence without a stored Player. This
  affects all map-record kinds, not only login. The insertion now executes unconditionally and
  only the displaced-record invariant is debug-only. C++ `Map::AddPlayerToMap`
  (`Map.cpp:427-445`) performs insertion independently of `ASSERT`. The production-linked login
  test now also reaches EquipmentInventory after map selection and interleaved map ticks.
  On the old code it passes in dev and fails in release; the missing-manager rejection and
  pre-map hydration tests pass in both. No ownership duplication, SQL, opcode or new clock is
  introduced. Post-fix validation and installed QA are recorded in the Session checkpoint.

- **2026-09-04, #578 runtime QA — initial Player construction depended on its own inventory.**
  Production login reached the instance socket, then kicked with `canonical Player mail owner
  disappeared`. `build_initial_player_for_owner_like_cpp` called presentation hydration, which
  queried canonical inventory before the new Player handle existed. Unit fixtures supplied a
  Session-side inventory and masked the cycle. Initial equipment hydration is now fixture-only;
  production keeps Player's initial empty equipment until the existing inventory load. C++
  constructs Player in `CharacterHandler.cpp:1065-1070`, establishes the session Player at
  `Player.cpp:17378`, and loads inventory/mail at `17748/17759`. No SQL or packet layout changes.
  The production-linked `production_login_player_owner` regression fails on the old code and
  passes with the fix; its missing-manager case rejects continuation. It stops at the PetStable
  read after mail/scalar hydration and does not claim a complete login. Live QA is recorded in
  the Session checkpoint separately.

- **2026-09-04, #578 runtime QA — nullable LFG hotfix text aborted startup.** The checked
  candidate rejected `LFGDungeons.Description` SQL NULL; the local positive-build batch has
  99 rows, two with NULL descriptions. C++ `Field::GetString` (`Field.cpp:118-126`) returns
  empty text for NULL. `DB2DatabaseLoader.cpp:121-132,275-287` preserves an existing localized
  string for an empty hotfix; `DB2LoadInfo.h:3365-3372` classifies both Name and Description
  as `FT_STRING`. The MariaDB adapter now distinguishes a valid nullable text value from a
  missing/mistyped column, and `wow-data::LfgDungeonsStore` preserves previous text while
  applying numeric fields. Focused tests cover missing rows, null/empty/nonempty SQL text,
  wrong types and missing columns, previous/new IDs and successive overlays; the explicit
  read-only MariaDB regression passed. This is a behavior correction separate from the
  Session capability extraction. Custom-row batching and other locale coverage remain outside
  this bounded fix; full startup/login acceptance is recorded separately.

## Bounded legacy repairs accepted during the port

- [x] **Issue #161 — battle-pet trainer purchases no longer strand the charge across the
  Character/Login commit window.** Legacy `Trainer::TeachSpell` charges money in memory and
  `BattlePetMgr::AddPet` builds the pet in memory; both sides persist only at the next
  `Player::SaveToDB`, which commits Character DB first and Login DB second
  (`Player.cpp:19336-19344`; money via `CHAR_UPD_CHARACTER` at `Player.cpp:19498-19505`, pet via
  `LOGIN_INS_BATTLE_PETS` at `BattlePetMgr.cpp:340-364`). A crash or failed Login commit between
  the two keeps the charge and loses the pet, and `BattlePetMgr::SaveToDB` clears
  `SaveInfo = BATTLE_PET_UNCHANGED` when statements are *appended*, before the commit result is
  known (`BattlePetMgr.cpp:377`), so the insert is never retried and the loss is silent; the
  dependent learned spell is intentionally never persisted (`Player.cpp:20437-20448`), leaving no
  proof of purchase. Rust instead records a durable saga command in the same Character DB
  transaction that deducts the guarded money, applies it once through the #160 account owner
  (whose Login DB transaction writes pet + receipt together under the account fence), queues
  publication only after the pet is durable, records the publication marker after enqueue, then
  completes the command, and refunds terminal failures exactly once; login recovery
  converges any interrupted command. Focused fault-injection tests distinguish this repair from
  both the legacy loss (charged without pet) and a speculative rewrite (no distributed
  transaction, no second journal owner): every crash boundary converges to either paid+pet or
  refunded+no-pet. Packet enqueue attempts are recoverable and may repeat after a crash between
  enqueue and marker; actual delivery remains best-effort without a client ACK. This is preferable
  to consuming the sole durable recovery signal before attempting the notification.
- [x] **Issue #159 — keep arena/battleground spell disables contextual.** Legacy
  `DisableMgr::IsDisabledFor` checks the arena and battleground flags, but when neither context
  matches and no map/area flag follows it falls through to the unconditional global-disable
  return (`DisableMgr.cpp:285-345`). Rust treats arena, battleground, map and area as location
  scopes: a scoped row disables the spell only when at least one declared scope matches. Focused
  tests pin normal-world rejection of the legacy fallthrough and positive arena/battleground
  matches.
- [x] **Issue #163 — rebuild skill indexes after final hotfix removals.** Legacy C++ builds
  selected `SkillLineAbility` / `SkillRaceClassInfo` derived indexes before
  `DB2Manager::LoadHotfixData` performs its final `RecordRemoved` pass
  (`DB2Stores.cpp:1328-1334,1539-1607`). That can leave a removed record reachable through a
  stale index. Rust composes WDC4 → official SQL → custom SQL → final removal first, then rebuilds
  every acquisition index from the surviving rows in ascending record-ID order. Focused fixtures
  distinguish this repair from both the stale C++ outcome and an unrelated rewrite.
- [x] **Issue #163 — an empty world `spell_learn_spell` table no longer erases canonical
  learning edges.** Legacy `SpellMgr::LoadSpellLearnSpells` returns before scanning
  `SpellEffect` and `SpellLearnSpell.db2` when the custom world query has no rows
  (`SpellMgr.cpp:990-1135`). Rust treats that result as zero custom rows and still builds the
  canonical graph. The loader test pins both effective edge families with an empty SQL input.
- [x] **Issue #163 — reject lossy acquisition narrowing.** Legacy
  `SpellMgr::LoadSpellLearnSkills` implicitly narrows effect-derived skill and step values to
  `uint16`, and DB-backed difficulty values to the `uint8` `Difficulty` enum
  (`SpellMgr.cpp:947-988,2730-2940`). Rust preserves checked source values in the immutable
  acquisition catalog and omits an unrepresentable compatibility node instead of authorizing a
  wrapped identifier. It also rejects an `EffectBasePoints` value whose C++ `float` round-trip
  would fall outside `int32`, rather than inheriting an undefined C++ cast or Rust saturation.
  Positive and negative fixtures pin the first-final-effect rule.
- [x] **Issue #163 — ranged learn-skill tiers are explicit instead of restart-random.** Legacy
  `SpellMgr::LoadSpellLearnSkills` calls `SpellEffectInfo::CalcValue()` once during startup
  (`SpellMgr.cpp:947-988`, `SpellInfo.cpp:495-559`), so a custom `SPELL_EFFECT_SKILL` with
  variance or ranged `DieSides` can select a different skill tier, tier maximum and durable
  player state after a restart whenever its rounded result domain has multiple values. The audited
  effective 3.4.3 data has 98 such effects and all are deterministic (`DieSides = 1`, zero
  variance/coefficient), so Rust preserves every official node and step. For custom/future
  ambiguous metadata it retains the complete checked value domain—including `frand`'s exclusive
  upper endpoint—and publishes a typed indeterminate lookup instead of silently treating the
  spell as having no learn-skill effect or inventing a minimum/maximum/average. The pure
  acquisition planner in #164 must consume that lookup and fail before mutation.
- [x] **Issue #163 — malformed effective rank graphs cannot hang startup or masquerade as
  unranked spells.** Legacy `SpellMgr::LoadSpellRanks` follows `SupercedesSpell` without cycle
  detection (`SpellMgr.cpp:812-902`); a custom/hotfix graph with a reachable cycle can loop
  forever, while merges and stale predecessor bookkeeping can construct incoherent chains. Rust
  builds a rank-specific projection from every final effective `SkillLineAbility` identity before
  hydrating unrelated acquisition fields, so an invalid race/skill mask neither erases a valid
  rank edge nor hides an invalid rank endpoint. Final hotfix removals still win. Rust then resolves
  valid and indeterminate candidates through one RecordID-ordered, last-wins authority per
  predecessor, rejects the complete ambiguous component for self-loops, cycles, multiple
  predecessors, ranks outside `uint8`, or unrepresentable endpoints, and retains a tri-state
  diagnostic lookup so later acquisition planning fails closed. A later valid candidate can
  repair an earlier malformed candidate for the same predecessor. If a representable endpoint is
  absent from exact spell authority, Rust skips the row just as C++'s paired `GetSpellInfo` gate
  does. Only a row with neither endpoint representable in C++'s `int32` source domain makes the
  rank projection globally indeterminate rather than inventing `Unranked`.
- [x] **Issue #163 — sign-extend narrow WDC4 signed-immediate fields.** The generic Rust WDC4
  reader previously returned an unextended `u32` payload from `get_field_i32` when a signed field
  occupied fewer than 32 bits. C++ explicitly extends `SignedImmediate` values before copying them
  into the requested signed type (`DB2FileLoader.cpp:858-869`). Rust now does the same while
  preserving raw unsigned access; synthetic bit-width fixtures and the real 3.4.3
  `SpellEffect.EffectBasePoints` data pin both paths. This fixes signed acquisition payloads and
  other existing `i32` consumers without treating the separate floating-point
  `world.serverside_spell_effect` source as regular DB2 metadata.
- [x] **Issue #164 — do not publish a newly inserted lower rank as active.** Legacy
  `Player::AddSpell` demotes a newly learned lower rank when a higher rank is already active, but
  returns the stale local `active` argument rather than the final `PlayerSpell::active` value
  (`Player.cpp:2855-2897,3135-3137`). `Player::LearnSpell` can consequently emit both
  `SMSG_SUPERCEDED_SPELL(low, high)` and a contradictory learned-spell publication
  (`Player.cpp:3192-3214`). The immutable Rust plan uses the final row state, retains the
  supersede intent, and deliberately omits the contradictory learned intent. A focused fixture
  distinguishes this bounded repair from ordinary higher-rank replacement.
- [x] **Issue #164 — reject skill-slot alias/capacity corruption instead of reproducing it.**
  Legacy `Player::SetSkill` uses `0` both as a valid array index and as “no free slot”, then
  activates parent/child skills after selecting but before claiming the slot
  (`Player.cpp:5799-5856`). Near capacity this can reject a genuinely free slot or let recursive
  activation reuse a stale position. Rust requires exact occupied-slot authority, activates
  causal parents/children, rechecks capacity, and returns a structured indeterminate outcome
  without exposing partial state. This is an intentional safety repair, not a claim that the
  legacy sentinel behavior was desirable protocol semantics.

---

## CRIT — data loss / duplication / corruption (fix before trusting the server with real chars)

- [x] **D-C1 Item enchantments not loaded on relog.** `SEL_CHAR_EQUIPMENT`/`SEL_CHAR_BAG_CONTENTS`
  select enchantment cols but the load hardcodes 0 → equipped/bagged enchants vanish on
  logout. `handlers/character.rs:4617-4618,4760-4761`. C++ `Player::_LoadInventory`.
  - 2026-07-01 issue #20 local slice: Rust now selects `item_instance.enchantments` for the
    specialized equipment/bag login queries, parses the 13x `(id,duration,charges)` fields like
    C++ `Item::LoadFromDB`, applies them to runtime `Item` objects, and includes them in item
    `CREATE_OBJECT` blocks. PR #89 merged with all required checks green. The issue-#20 closeout
    then loaded an enchanted/random-property item through both installed C++ and Rust runtimes,
    preserved the exact CharacterDB metadata around occupied forward/reverse swaps, and produced
    the same complete item-create block SHA-256 on both sides. Final reviewer hardening requires
    an observed empty-body logout packet in every phase, the second item's exact all-zero
    enchantment state, and a third Rust authentication proving the reverse-save reload:
    `25238a033be693b4969b9412f1666074e5d9be76c6db3b188e021a60b4feb2c8`.
- [x] **D-C2 Item random properties not loaded on relog.** Same query gap → magical items
  become non-magical. `handlers/character.rs:4617`.
  - 2026-07-01 issue #20 local slice: the same login path now loads `randomPropertiesId` and
    `randomPropertiesSeed` for equipped and bagged items into runtime item state and login create
    data. The same paired C++/Rust logout/relog proof above covers the nonzero property ID, seed,
    generated property enchantment and exact serialized item block.
- [x] **D-C3 Bank/equipment-set/void-storage persistence incomplete.** The original audit found
  these storage paths represented only in memory, with loss on logout.
  - 2026-07-13 issue #102 local slice: personal `AUTOBANK` / `AUTOSTORE_BANK_ITEM` now plans
    C++ `CanBankItem` / `CanStoreItem` destinations, commits every stack/location plus the
    surviving items' count/expiration/charges/flags/enchantments/durability/played-time and
    applicable quest-status change in one character transaction, and mutates runtime only after
    a successful commit. Fully absorbed sources also delete both stored-container-loot tables.
    Login now loads expiration/charges, normalizes template duration, limits charges to real
    ItemEffects, and restores duration trackers so that mutable-state save cannot overwrite them
    with defaults. Coverage includes empty destinations, merge+remainder, full bank,
    bank withdrawal, the C++ first-match stop and special item-push packet for quest-bound
    objectives (with no generic item-objective credit), equipment removal packet
    masks, merge-destination enchantment timer refresh without item-expiration registration,
    current enchantment durations, binding, obtain spells, a real failed-connection SQL commit
    regression, and fully merged metadata cleanup. The regression also repaired an old
    false-positive test that sent
    legacy slot `19` instead of the 3.4.3 backpack start slot `35`.
    PR #103 merged on 2026-07-14 after the full live bot bank deposit/relog/withdraw/relog
    round-trip passed. Personal-bank movement is therefore closed.
  - 2026-07-21 issue #112, merged as PR #113: equipment sets and transmog outfits now share one
    process-wide, startup-initialized GUID namespace like C++ `ObjectMgr`; initialization uses the
    exact combined CharacterDB maximum and fails closed. The existing player-save transaction
    persists new/changed/deleted rows, and login correctly decodes signed transmog schema values.
    A two-client installed runtime run proved concurrent distinct GUIDs, exact durable rows, two
    fresh-auth relogs with exact loaded sets, and cleanup. The committed one-packet
    `SMSG_EQUIPMENT_SET_ID` action capture is byte-clean against C++ on the instance route with no
    accepted divergence. Equipment-set persistence is therefore closed.
  - 2026-07-22 issue #114 / PR #115: void storage now loads into a validated fixed 160-slot
    authority and uses one process-wide, startup-initialized item-ID generator like C++
    `ObjectMgr`. Unlock, query, transfer and swap enforce the represented C++ gates. Deposit,
    withdrawal and swap commit flags/money, inventory/item mutations and all affected void rows
    in one CharacterDB transaction before publishing runtime or success packets; definite
    rollback stays invisible and indeterminate COMMIT is fenced from stale saves. This explicitly
    accepts issue #114's failure-only divergence from C++ intermediate deposit publication when a
    later withdrawal fails validation; the issue's Done contract requires one transaction and no
    runtime change on definite failure. C++ packet
    contrast also corrected every void-storage GUID from a fixed 16-byte Rust/bot encoding to
    C++ `PackedGuid`. Focused tests, an installed unlock/deposit/relog/swap/relog/withdraw/relog
    lifecycle with exact cleanup, and a 1/1 byte-clean real C++/Rust query capture have passed.
    Review hardening also restores C++ random-property/suffix enchantment slots on withdrawal and
    persists the same effective enchantment array, so a later save/relog cannot strip item affixes.
    Locked-character login now skips residual void rows like C++ while initializing coherent empty
    storage; unlock deletes those skipped rows in the same money/flag transaction, so neither a
    same-session query nor a restart can expose contents C++ never loaded.
    Withdrawal now honors C++ merge-before-empty `CanStoreNewItem` placement across the entire
    atomic request while excluding stacks/children already planned for deposit destruction, login
    replays each valid row's represented collection appearance hook, and swap destination values
    preserve C++'s `uint32` to `uint8` truncation before range checks.
    GitHub review also published a withdrawn item's pre-random/pre-handler `CREATE_OBJECT` and its
    post-store random-property/creator/binding VALUES update before the slot update, and capped
    allocation at the packet GUID's 40-bit counter so raw IDs cannot alias after truncation.
    Context-column, void-packet random-affix, and fixed-scaling review suggestions are intentionally
    not applied because exact C++ contrast confirms the existing Rust behavior in all three cases.
    A later current-HEAD review also adds an explicit older-Rust compatibility repair that restores
    the C++ schema default for legacy zero-slot characters, atomically plans and persists
    item-objective quest state across ordered deposit destruction (including bag children) and
    withdrawal credit, preserves intermediate recursive removal checks plus quest-bound
    no-physical-item withdrawals, and sends live collection updates for both new and merged
    physical withdrawals. These latest fixes pass focused void/capacity/quest/collection tests;
    the complete local PR preflight and local Codex review completed CLEAN on `2143334b` in 471.8
    seconds. PR #115 merged as `55719eb4` with CI and the current-HEAD Codex verdict green. Bank,
    equipment/transmog sets and void storage are therefore all closed for the scoped D-C3 paths.
- [x] **D-C4 Inventory swap not transactional.** Two separate `execute()` calls; mid-fail
  orphans/dupes items. `handlers/character.rs:11668-11681`. C++ appends both changed positions to
  the character save transaction through `Player::_SaveInventory`.
  - 2026-07-14 issue #104 local slice: every represented direct-inventory `SwapItem` route now
    appends the final positions to one character transaction with C++
    `CHAR_REP_INVENTORY_ITEM`/`REPLACE INTO` semantics. `REPLACE` is required for occupied swaps
    because `character_inventory.uk_location` forbids the first half of a two-`UPDATE` exchange.
    Runtime slots, equipment modifiers, accessor/registry state, loot release, stat/value packets,
    and success logging now occur only after commit; missing/failed persistence sends
    `SMSG_INVENTORY_CHANGE_FAILURE` and leaves runtime unchanged. Focused coverage includes empty
    and occupied plans, generic commit failure, explicit auto-equip-slot failure, and all 2,763
    `wow-world` library tests. The 2026-07-14 release-build live QA passed both the isolated bot
    round-trip (occupied swap, logout, full re-auth/relogin, inverse swap, second persistence
    check) and a manual client swap/relogin check. An accidental debug-binary deployment was
    rejected after a stack overflow and replaced with the verified release artifact before these
    passes. PR #105 merged with every required check and the current-HEAD Codex verdict green.
    The issue-#20 closeout reran both occupied swaps through logout/fresh-auth and exact DB checks.
  - Separate validation follow-up, 2026-07-22 issue #52 local slice: the live move/equip/store
    handlers now use C++ `IsValidPos`, bank-interaction, `CanUnequipItem`, `CanStoreItem`/
    `CanBankItem`, `CanEquipItem`, bag, unique-equip and recursive-destroy rules instead of the
    former direct-inventory simplification. `Player::SwapItem` now covers empty moves, merges,
    bidirectional real swaps, bag exchanges, child redirects and persisted offhand follow-up, with
    each concrete mutation committed before runtime publication. Paired installed C++/Rust QA
    proved the invalid container-aware source error on the C++ realm route plus forward/reverse
    occupied swaps and fresh-auth metadata; strict capture-diff matched request and response with
    zero value/routing/count differences. This closes the bounded C#-ITEM.2 behavior, not broader
    item/gem/durability parity, and remains pending PR CI/current-HEAD review/merge. GitHub review
    additionally applied current upstream TrinityCore's missing legacy `AutoUnequipChildItem`
    pre-step before child redirects and stopped internal inventory relocations from re-crediting
    quest objectives. Proposed `CanUseBank` guards for auto-equip/auto-store were not applied:
    both the local 3.4.3 source and current upstream omit them, while the swap handlers retain them.
- [x] **D-C5 Loot item TOCTOU → duplication.** Slot marked looted *after* the async inventory
  store; two concurrent looters both store it. `handlers/loot.rs`. C++ instead gets safety from
  object-owned `Loot` plus globally serialized `PROCESS_THREADUNSAFE` session work; it validates
  storage before mutating the shared slot.
  - 2026-07-18 issue #106 local slice: creatures/gameobjects now own a generation-tagged shared
    loot authority; item/master/roll/disenchant paths use cancellation-safe leases whose detached
    persistence worker owns the claim across SQL `COMMIT`. Session loot tables are packet caches,
    and stale corpse/GO generations and stale group rolls fail closed. The guarded two-bot race,
    single-session C++/Rust capture, original-client QA, CI and current-HEAD review all completed;
    PR #107 merged.
- [x] **D-C6 Loot money TOCTOU → duplication.** `loot.coins` zeroed *after* distribute; two
  concurrent `handle_loot_money` both pay out. `handlers/loot.rs`.
  - 2026-07-18 issue #106 local slice: one detached worker atomically persists every connected,
    allowed, in-range group share, commits the object-owned money claim, and then schedules one
    exact-once runtime application per session without cross-session acknowledgement waits. A
    cancelled packet future cannot reopen a successful DB transaction. The same required gates
    completed in merged PR #107.
  - **Separate crash-recovery boundary (does not reopen D-C5/D-C6):** these detached workers and their
    completion trackers are in-process only. A runtime/process abort exactly after SQL `COMMIT`
    but before the synchronous authority/completion continuation can still lose that continuation.
    Closing `kill -9` recovery requires a durable claim journal written in the same transaction
    and replayed at startup; neither the current authority nor the session tracker is that journal.
- [x] **D-C7 Player save had incomplete transaction coverage.** Issue #17 / PR #88 adds the
  periodic save timer and wraps the Rust-covered represented `Player::SaveToDB` statements in one
  `SqlTransaction`, clearing dirty state only after commit. Automated runtime QA covers
  login/logout, inactive action rows, travel columns and unchanged quest-objective rows; manual
  original-client QA confirmed logout/relog plus action-bar and cooldown persistence. PR #88
  merged with CI and Codex review green. The issue-#20 paired C++/Rust run additionally verifies
  the observable logout envelope, including C++'s realm-routed empty
  `SMSG_LOGOUT_COMPLETE`. Full C++ save breadth and login/account cross-database coupling remain
  Part-2 parity work, not an open instance of this scoped CRIT transaction defect. `session.rs`.
- [x] **D-C8 Vendor buy not atomic.** Gold/currency applied to runtime before item DB commit;
  commit fail = paid, no item. `handlers/character.rs:10177-10292`.
  - 2026-07-20 issue #108 local slice: ordinary item purchases already gained a combined
    gold/item/turn-in transaction in #107, but item extended-cost currencies and the entire
    currency-vendor branch still changed session currency before awaiting COMMIT. Both paths now
    build detached currency plans and publish them only after the purchase transaction commits.
    Currency-only purchases reuse the cancellation/unknown-COMMIT quarantine with equal money
    markers, so definite rollback leaves runtime untouched and an ambiguous result requires relog
    without allowing a stale full save. A failed-connection handler regression exercises the real
    rollback branch and proves that it emits only `BuyFailed`, preserves runtime currency, and
    reopens payout/save admission. Paired C++/Rust bot QA now proves a real extended-cost purchase,
    currency debit, item creation, fresh-authentication persistence, packet routing, and cleanup;
    the committed post-COMMIT realm response is 2/2 CLEAN with no accepted divergences. Capture
    contrast also fixed zero-price Coinage publication and C++ vendor-item create/context/flag
    metadata. The wider action still shows the separately scoped missing achievement
    `SMSG_CRITERIA_UPDATE`. Installed original-client QA on 2026-07-21 bought two extended-cost
    items across a relog and confirmed exact item/currency persistence in CharacterDB; the fixture
    was then fully restored. The confusing client `You receive currency` line was backed by the
    same byte-exact loss packet as C++ (quantity 15, delta -15, Vendor reason), not a refund.
    PR #109 merged after final CI and the current-HEAD Codex verdict passed.
- [x] **D-C9 Group full-check race.** Size checked then join without re-check → 6+ member
  groups under concurrent accepts. `handlers/group.rs:928-1044`.
  - 2026-07-21 issue #110 local slice: C++ checks `Group::IsFull` immediately before
    `Group::AddMember` on its serialized execution path. Rust now performs that pair under one
    mutable `GroupRegistry` guard and returns explicit Full versus AddFailed results to the live
    handler; `ERR_GROUP_FULL` leaves the rejected session and group unchanged, while AddFailed
    retains C++'s silent return. A barrier-synchronized
    regression starts two simultaneous joins for the fifth party slot, proves exactly one Added
    plus one Full result, and verifies the final member count remains five. The installed
    `4adf87e1` runtime and three-client bot race passed locally on 2026-07-21: one candidate
    received exact `Invite/GROUP_FULL`, the other joined, CharacterDB contained exactly the four
    initial members plus that winner, all sessions logged out, and the fixture was restored.
    PR #111 merged after final CI and the current-HEAD Codex verdict passed.

## HIGH — broken mechanics / silent failure / exploit

- [ ] **D-H1 Melee damage has no formula.** Uses raw weapon-damage range as final damage; **no
  armor mitigation, no AP scaling, no level reduction.** `session.rs:7913-7942`. C++
  `Unit::CalcArmorReducedDamage` / AP→damage.
- [ ] **D-H2 Melee hit table absent.** miss/dodge/parry/block/glancing/crit all bypassed;
  hardcoded `HIT_INFO_NORMAL_SWING|VICTIM_STATE_HIT`. `session.rs:47813-47823`. C++
  `Unit::MeleeSpellHitResult`.
- [ ] **D-H3 Spell damage/heal uses raw base points.** No coefficient, crit, or resist.
  `session.rs:49014-49026`.
- [ ] **D-H4 ⚠VERIFY Quest kill-credit (MONSTER objective) not wired.** No
  `KilledMonster`→objective path found; "kill X" may be uncompletable. **Contested:** a
  separate pass said monster/GO kills advance. Must verify on a live kill. `handlers/quest.rs`.
- [ ] **D-H5 Quest area-trigger (explore) objectives not wired.** Type 10 falls to `_=>false`;
  "explore Y" uncompletable. `handlers/quest.rs:653`.
- [ ] **D-H6 Quest item-loot objectives not credited.** Loot path doesn't advance "collect X"
  objectives. `handlers/loot.rs:6786`.
- [ ] **D-H7 Auras not saved at logout.** All buffs/debuffs reset on relog. `session.rs:21656`.
  C++ `Player::_SaveAuras`.
- [ ] **D-H8 Periodic save represented-partial + incomplete logout save.** Issue #17 adds a
  `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` session timer for represented `Player::SaveToDB`,
  but full inventory / mid-quest progress / newly-learned spells may still be outside the Rust
  save surface; first-save randomization, capture diff, and manual live-client QA remain pending.
  The installed runtime passed bot login/logout and action/travel/quest-objective preservation QA.
  (Pairs with M0.4.)
- [ ] **D-H9 Trainer skips req-skill-rank + prerequisite-spell checks.** Loaded but ignored →
  learn spells you shouldn't. `handlers/trainer.rs:405-463`. C++ `Trainer.cpp:195-200`.
- [ ] **D-H10 Movement trusts client position.** Only NaN/map-bounds checks; no speed/teleport
  validation → speed/teleport hacking. `handlers/movement.rs:310-356`.
- [ ] **D-H11 Vendor stock-limit TOCTOU → oversell.** Count read then commit without re-check.
  `handlers/character.rs:10056-10070`.
- [ ] **D-H12 Buyback slot TOCTOU + overwrite without cleanup → item loss.** `character.rs:10781-10882`.
- [ ] **D-H13 Group created without leader in member list** on a creation-fail path → runtime/DB
  mismatch. `handlers/group.rs:1050-1074`.
- [ ] **D-H14 Duplicate-CREATE crash: async race window.** Fix relies on `client_visible_guids`
  diff, but the set is mutated *after* send; async concurrency can resend CREATE (client
  crash). `handlers/character.rs:6485-6488,7713-7716`.
- [ ] **D-H15 Creature DESTROY_OBJECT deferred to player movement.** Creature that walks away
  stays a phantom (targetable, not rendered) until the player moves. `handlers/movement.rs:274`.
- [ ] **D-H16 PartyUpdate omits offline group members.** C++
  `Group::SendUpdateToPlayer` serializes every `m_memberSlots` entry and marks disconnected
  members through `PartyPlayerInfo.Connected`; Rust builds `PlayerList` with
  `filter_map(PlayerRegistry::get)` but computes `MyIndex` from the complete member vector. The
  issue #110 live race observed a two-entry leader/winner list and complete-list `MyIndex=4` for
  a five-member persisted party. `handlers/group.rs:505-512`; C++ `Group.cpp:820-873`.

## MED — wrong values / loose checks / minor loss

- [ ] **D-M1 Silent gold-save error.** `let _ = char_db.execute(stmt).await` swallows failures. `session.rs:21495`.
- [x] **D-M2 was not a defect: C++ also discards group-money division remainder.**
  `LootHandler.cpp::HandleLootMoneyOpcode` computes `loot->gold / playersNear.size()` and credits
  that same truncated amount to every recipient; there is no first-recipient remainder branch.
- [ ] **D-M3 Off-hand dual-wield damage has no penalty** (~25% too high). `session.rs:7923`.
- [ ] **D-M4 Haste has no attack-speed cap** → scales unbounded. `session.rs:1358`.
- [ ] **D-M5 Threat = raw damage**, no ability/role threat modifiers. `session.rs:47875`.
- [x] **D-M6 Equipment sets in-memory only** was closed by issue #112 / PR #113: sets and
  transmog outfits now persist transactionally and load on fresh authentication with a shared
  collision-safe GUID namespace.
- [x] **D-M7 Void storage not saved** was closed by issue #114 / PR #115 with one atomic
  flags/money/inventory/void transaction, fresh-auth lifecycle proof, legacy-backpack repair,
  deposit quest-objective persistence, live withdrawal collection updates, focused tests and a
  byte-clean C++/Rust query capture. PR #115 merged as `55719eb4` with all required gates green.
- [ ] **D-M8 Group member DB insert fail logged-only**, runtime kept → reload drops member. `handlers/group.rs:1090`.
- [ ] **D-M9 Phase not re-checked on movement** → out-of-phase objects linger. `handlers/movement.rs:274`.
- [ ] **D-M10 Position save binds extra `instance_id`** vs C++ 7-field SavePosition (verify SQL param alignment). `session.rs:21578`.
- [ ] **D-M11 Loaded-grid GameObject/AreaTrigger GUID helpers hardcode realm 0.**
  `create_gameobject_like_cpp` and `create_area_trigger_like_cpp` in
  `wow-core/src/guid.rs` are used by the typed world-server/area-trigger load
  paths with realm zero. Both C++ trees pass zero at the world-object callsite
  but `ObjectGuidFactory::CreateWorldObject` replaces it with the active
  `realm.Id.Realm`; Rust currently skips that substitution. Creature/Vehicle
  callers were corrected after the issue #81 capture exposed the same defect,
  but this separate GO/AreaTrigger boundary remains open. C++
  `ObjectGuid.cpp:590-631`; Rust `world-server/src/main.rs` and
  `area_trigger_loaded_grid.rs`.
- [ ] **D-M12 `SMSG_LOGOUT_COMPLETE` uses the instance socket before channels
  are restored.** Live C++/Rust bot QA for issue #81 observed Rust routing
  `0x2684` on instance while stock C++ routes it on realm. The immediate logout
  path calls `send_packet(&LogoutComplete)` before `restore_realm_channels()`;
  the timed path also sends through the current channel. C++
  `Opcodes.cpp:1665` (`CONNECTION_TYPE_REALM`); Rust
  `handlers/character.rs::handle_logout_request` and
  `session.rs::complete_logout`. Functional relog succeeds because both bot
  sockets remain open, but wire routing is not parity-clean.
- [ ] **D-M13 Base `AreaTable.db2` loader reads four physical fields one
  position late.** `AreaTableMeta` uses an external ID (`IndexField = -1`), so
  the WDC4 indices for `ContinentID`, `ParentAreaID`, `AreaBit`, and
  `ExplorationLevel` are respectively `2`, `3`, `4`, and `11`; Rust currently
  reads `3`, `4`, `5`, and `12`. Hotfix rows use the C++ `DB2LoadInfo`/SQL
  column ordinals including ID and are not affected. The issue #81 review
  verified that the newly used `FactionGroupMask` index `14` is already
  correct (hotfix column `15`), as are `MountFlags` `16` and `Flags1` `21`.
  C++ `DB2Metadata.h::AreaTableMeta` / `DB2LoadInfo.h::AreaTableLoadInfo`;
  Rust `wow-data/src/area.rs::AreaTableStore::load`.
- [ ] **D-M14 Effective skill relation stores use source-interleaved startup
  order instead of C++ table-granular order.** Rust loads both
  `SkillLineAbility` and `SkillRaceClassInfo` WDC4 bases, then queries ability
  official, race-class official, ability custom, race-class custom. C++
  `DB2Manager::LoadStores` completes each `LOAD_DB2` independently, and
  `DB2StorageBase::LoadFromDB` loads official then custom before advancing to
  the next table. The persistence refactor #523 intentionally preserves this
  observable pre-existing query/failure order; #524 owns the separate fidelity
  correction and its order/failure tests. C++ `DB2Stores.cpp:848-850`,
  `DB2Store.cpp:127-133`, `DB2DatabaseLoader.cpp:28-33`; Rust
  `wow-data/src/skill.rs::SkillStore::load_wdc4_base_like_cpp` and
  `wow-database/src/skill_catalog_hotfix_adapter.rs`.

## LOW — non-issues in practice / cosmetic (recorded for completeness)

- [ ] **D-L1 Item StackCount/Durability written `i32`** vs C++ `uint32` — identical bytes for
  realistic values; only wraps >2³¹. `update.rs:5271,5294`. Tidy, not urgent.
- [ ] **D-L2 Item Expiration/Artifact size fields** type/empty-array cosmetics. `update.rs:5272,5307`.
- [ ] **D-L3 DK DisplayPower=5 (Runes) vs 6 (RunicPower)** — also tracked as #1213/M1.4. `update.rs:1793`.

---

## How this feeds the plan

These are **bugs in shipped code**, distinct from missing features. In `PORT_PLAN.md` they are
the **D-track** (existing-code hardening), checkbox-tracked here. Priority placement:
- **CRIT data-loss/dupe (D-C1..C9)** → pulled into **M0/M1** (we can't validate gameplay on a
  server that loses enchants, wipes banks, or dupes loot).
- **Combat correctness (D-H1..H3)** → folded into **M3** (real combat) — they're why M3 exists.
- **Quest crediting (D-H4..H6)** → **M4.7**.
- The rest → addressed in their owning milestone, verified by capture/round-trip test.
