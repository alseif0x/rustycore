# RustyCore — Defects in EXISTING (already-developed) code

**Date:** 2026-06-27 · **Base:** `develop` @ `d171117c`.

This is the adversarial audit the user asked for: **not what's missing, but what's wrong
in what already exists.** An 8-agent parallel pass tried to *break* the capabilities STATE.md
labels WORKS, contrasting each against C++ (`/home/server/woltk-trinity-legacy`).

**Reliability note:** these are agent findings; each carries `file:line` + a C++ ref, but
they are **leads to verify, not proven verdicts**. Two are explicitly contested between
agents (marked ⚠VERIFY). Signedness divergences (i32 vs u32) are rated **LOW** because they
only differ for values >2³¹ that never occur in normal play (stack counts, durability) —
identical bytes otherwise. Severity reflects my judgment after that filter.

Headline: **even the live core loop has correctness/integrity bugs.** "Sends the packet and
mutates DB" ≠ "computes the right result / can't lose or dupe data."

---

## CRIT — data loss / duplication / corruption (fix before trusting the server with real chars)

- [ ] **D-C1 Item enchantments not loaded on relog.** `SEL_CHAR_EQUIPMENT`/`SEL_CHAR_BAG_CONTENTS`
  select enchantment cols but the load hardcodes 0 → equipped/bagged enchants vanish on
  logout. `handlers/character.rs:4617-4618,4760-4761`. C++ `Player::_LoadInventory`.
  - 2026-07-01 issue #20 local slice: Rust now selects `item_instance.enchantments` for the
    specialized equipment/bag login queries, parses the 13x `(id,duration,charges)` fields like
    C++ `Item::LoadFromDB`, applies them to runtime `Item` objects, and includes them in item
    `CREATE_OBJECT` blocks. Kept open until capture-diff/live relog QA is run.
- [ ] **D-C2 Item random properties not loaded on relog.** Same query gap → magical items
  become non-magical. `handlers/character.rs:4617`.
  - 2026-07-01 issue #20 local slice: the same login path now loads `randomPropertiesId` and
    `randomPropertiesSeed` for equipped and bagged items into runtime item state and login create
    data. Kept open until capture-diff/live relog QA is run.
- [ ] **D-C3 Bank/equipment-set/void-storage persistence incomplete.** The original audit found
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
  - 2026-07-22 issue #114 local slice: void storage now loads into a validated fixed 160-slot
    authority and uses one process-wide, startup-initialized item-ID generator like C++
    `ObjectMgr`. Unlock, query, transfer and swap enforce the represented C++ gates. Deposit,
    withdrawal and swap commit flags/money, inventory/item mutations and all affected void rows
    in one CharacterDB transaction before publishing runtime or success packets; definite
    rollback stays invisible and indeterminate COMMIT is fenced from stale saves. C++ packet
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
    The repeated complete local preflight and local Codex review are clean on `83cee501`. All
    three D-C3 children are implemented locally; this aggregate remains open for issue #114 CI,
    current-HEAD GitHub Codex verdict and merge.
- [ ] **D-C4 Inventory swap not transactional.** Two separate `execute()` calls; mid-fail
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
    passes. Kept open only until the PR current-HEAD gates pass.
- [ ] **D-C5 Loot item TOCTOU → duplication.** Slot marked looted *after* the async inventory
  store; two concurrent looters both store it. `handlers/loot.rs`. C++ instead gets safety from
  object-owned `Loot` plus globally serialized `PROCESS_THREADUNSAFE` session work; it validates
  storage before mutating the shared slot.
  - 2026-07-18 issue #106 local slice: creatures/gameobjects now own a generation-tagged shared
    loot authority; item/master/roll/disenchant paths use cancellation-safe leases whose detached
    persistence worker owns the claim across SQL `COMMIT`. Session loot tables are packet caches,
    and stale corpse/GO generations and stale group rolls fail closed. Kept open until the live
    two-bot race, single-session capture, manual original-client QA, CI, and current-HEAD review
    gates pass.
- [ ] **D-C6 Loot money TOCTOU → duplication.** `loot.coins` zeroed *after* distribute; two
  concurrent `handle_loot_money` both pay out. `handlers/loot.rs`.
  - 2026-07-18 issue #106 local slice: one detached worker atomically persists every connected,
    allowed, in-range group share, commits the object-owned money claim, and then schedules one
    exact-once runtime application per session without cross-session acknowledgement waits. A
    cancelled packet future cannot reopen a successful DB transaction. Kept open for the same
    runtime/review gates as D-C5.
  - **Open crash boundary (not acceptance credit for #106):** these detached workers and their
    completion trackers are in-process only. A runtime/process abort exactly after SQL `COMMIT`
    but before the synchronous authority/completion continuation can still lose that continuation.
    Closing `kill -9` recovery requires a durable claim journal written in the same transaction
    and replayed at startup; neither the current authority nor the session tracker is that journal.
- [ ] **D-C7 Player save has incomplete transaction coverage.** Issue #17 wraps the
  Rust-covered represented `Player::SaveToDB` character statements in one `SqlTransaction`, but
  full C++ save parity, login/account transaction coupling, capture diff, and manual live-client
  QA remain pending. Automated runtime QA now covers login/logout plus preservation of action rows
  outside the active spec, travel columns, and unchanged quest-objective rows outside this seam's
  ownership. Previous serial awaits could leave DB inconsistent (gold debited, item not added;
  level saved, position reverted).
  `session.rs`.
- [ ] **D-C8 Vendor buy not atomic.** Gold/currency applied to runtime before item DB commit;
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
    same byte-exact loss packet as C++ (quantity 15, delta -15, Vendor reason), not a refund. Kept
    open until final CI, current-HEAD reviewer verdict, and merge.
- [ ] **D-C9 Group full-check race.** Size checked then join without re-check → 6+ member
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
    Kept open until CI, current-HEAD reviewer verdict, and merge.

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
- [ ] **D-M7 Void storage not saved.** Issue #114 fixes this locally with one atomic
  flags/money/inventory/void transaction and fresh-auth lifecycle proof; the GitHub-review fixes
  and repeated full preflight/local review are clean, so CI, current-HEAD GitHub review and merge
  gates remain.
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
