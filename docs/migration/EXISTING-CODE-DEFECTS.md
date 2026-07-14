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
- [ ] **D-C3 Bank contents never persisted.** Bank moves recorded in-memory only
  (`represented_bank_item_moves`), no DB write → 100% bank loss on logout. `session.rs:31575`.
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
    round-trip passed. Personal-bank movement is therefore closed; equipment-set and void-storage
    persistence remain under the wider D-C3 heading and keep this aggregate item open.
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
    `wow-world` library tests. Kept open until the complete bot relog persistence smoke and PR
    current-HEAD gates pass.
- [ ] **D-C5 Loot item TOCTOU → duplication.** Slot marked looted *after* the async inventory
  store; two concurrent looters both store it. `handlers/loot.rs:1143-1219`. C++ blocks the
  slot *before* store.
- [ ] **D-C6 Loot money TOCTOU → duplication.** `loot.coins` zeroed *after* distribute; two
  concurrent `handle_loot_money` both pay out. `handlers/loot.rs:1293-1356`.
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
- [ ] **D-C9 Group full-check race.** Size checked then join without re-check → 6+ member
  groups under concurrent accepts. `handlers/group.rs:928-1044`.

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

## MED — wrong values / loose checks / minor loss

- [ ] **D-M1 Silent gold-save error.** `let _ = char_db.execute(stmt).await` swallows failures. `session.rs:21495`.
- [ ] **D-M2 Money split rounding loss.** Integer division discards remainder copper (C++ gives it to first recipient). `handlers/loot.rs:1323`.
- [ ] **D-M3 Off-hand dual-wield damage has no penalty** (~25% too high). `session.rs:7923`.
- [ ] **D-M4 Haste has no attack-speed cap** → scales unbounded. `session.rs:1358`.
- [ ] **D-M5 Threat = raw damage**, no ability/role threat modifiers. `session.rs:47875`.
- [ ] **D-M6 Equipment sets in-memory only** (no DB) → lost on logout. `handlers/character.rs:4798`.
- [ ] **D-M7 Void storage not saved.** `session.rs:21656`.
- [ ] **D-M8 Group member DB insert fail logged-only**, runtime kept → reload drops member. `handlers/group.rs:1090`.
- [ ] **D-M9 Phase not re-checked on movement** → out-of-phase objects linger. `handlers/movement.rs:274`.
- [ ] **D-M10 Position save binds extra `instance_id`** vs C++ 7-field SavePosition (verify SQL param alignment). `session.rs:21578`.

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
