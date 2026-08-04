# Migration: BattlePets

> **C++ canonical path:** `src/server/game/BattlePets/` (`BattlePetMgr.{h,cpp}`) + `src/server/game/Handlers/BattlePetHandler.cpp` + `src/server/game/Server/Packets/BattlePetPackets.{h,cpp}` + `WorldPackets::BattlePet::*` family
> **Rust target crate(s):** `crates/wow-world` (account owner and handlers), `crates/wow-database` (Login DB persistence), `crates/world-server` (process owner/bootstrap), and `crates/wow-packet` (journal packets).
> **Layer:** L7 game system plus Login DB account ownership.
> **Status:** represented-partial. The architecture campaign rooted at issue #133 explicitly supersedes the 2026-06-07 WotLK-only scope freeze for the forward-ported legacy surface. Issue #160 moves the represented journal to a durable account owner; trainer purchase remains #161 and full pet-battle gameplay is still outside this slice.
> **Audited vs C++:** `BattlePetMgr.cpp` load/add/count/lock/persistence semantics contrasted for #160; Rust deliberately makes lease/capacity/GUID/persistence/publication atomic instead of reproducing the C++ split `HasMaxPetCount`/`AddPet` race.
> **Last updated:** 2026-08-02

## 2026-08-02 architecture-plan override (#160)

The current GitHub architecture plan requires the forward-ported trainer and battle-pet seams even though a stock 3.4.3 client does not expose the full UI. The old scope-freeze text below is retained as historical product context, not as an instruction to remove or bypass the issue-ordered work.

`BattlePetAccountOwnerLikeCpp` is the single mutable owner per Battle.net account. It loads pets and slots coherently, applies the C++ species/owner/per-species validation before publication, enforces one journal lease, reserves capacity before asynchronous persistence, and publishes only committed state. Adds use the consumed cage item's durable ObjectGuid as their request identity; receipts are intentionally retained after pet deletion because that source item must never grant twice. A singleton Login DB sequence row is locked only for each allocation transaction, while a durable account/species/owner-realm/owner-counter scope row serializes the final capacity recheck with the insert; the global `HighGuid::BattlePet` namespace and caps therefore remain safe when multiple realms share the Login DB. Ambiguous commits are reconciled, and cancellation-safe workers drain with a bounded server-shutdown deadline. `ClearFanfare` preserves the C++ exception that mutates without journal ownership while still using canonical per-pet serialization and persistence. This remains `represented-partial`: #161 owns trainer charging/cross-DB coordination, and unresolved/dormant handlers are not made production-ready by #160.

## 2026-08-03 recoverable trainer purchase saga (#161)

**C++ behavior (authoritative anchors).** `Trainer::TeachSpell` (`Trainer.cpp:79-147`) resolves the species through `BattlePets::BattlePetMgr::GetBattlePetSpeciesBySpell` — the SpellEffect/SummonProperties map built at `SpellMgr.cpp:2510-2514` (`SPELL_EFFECT_SUMMON` whose SummonProperties slot is `SUMMON_SLOT_MINIPET` and whose flags contain `SummonFromBattlePetJournal`), never `BattlePetSpecies.SummonSpellID`. A capped purchase returns silently with no packet (`Trainer.cpp:102-106`, "Don't send any error to client (intended)"); battle-pet spells suppress the trainer/player visual kits 179/362 (`Trainer.cpp:108,121-125`); money is charged with `ModifyMoney` in memory only (`Trainer.cpp:111-119`, `Player.cpp:23374-23407`, failure → `SMSG_TRAINER_BUY_FAILED` reason `NotEnoughMoney=1`, `Trainer.h:47-51`); then `AddPet(species, SelectPetDisplay, RollPetBreed, GetDefaultPetQuality)` (`Trainer.cpp:136-139`; `BattlePetMgr.cpp:449-490`: breed rolled from the world `battle_pet_breeds` table with default 3 B/B at `BattlePetMgr.cpp:201-208`, quality from world `battle_pet_quality` with default Poor at `BattlePetMgr.cpp:210-217`, display from `CreatureTemplate::GetRandomValidModel` unless `BattlePetSpeciesFlags::RandomDisplay` (`0x00800`) at `BattlePetMgr.cpp:219-227`, level 1, `SMSG_BATTLE_PET_UPDATES` petAdded) and `LearnSpell(spellId, dependent=true)` (`Trainer.cpp:140-145`; `Player.cpp:3192-3212` sends `SMSG_LEARNED_SPELLS`; dependent rows are deliberately never saved to `character_spell`, `Player.cpp:20437-20448`). `Trainer::GetSpellState` (`Trainer.cpp:185-223`) has no cap gate, so a purchasable battle-pet spell renders green (`Available`) in `SMSG_TRAINER_LIST`.

**Legacy crash-consistency defect.** Everything above is in-memory until the next `Player::SaveToDB`, which commits Character DB first and Login DB second (`Player.cpp:19336-19344`; the money rides `CHAR_UPD_CHARACTER` at `Player.cpp:19498-19505`, the pet is appended by `GetBattlePetMgr()->SaveToDB(loginTransaction)` at `Player.cpp:19688` → `LOGIN_INS_BATTLE_PETS`, `BattlePetMgr.cpp:340-364`). A crash or failed Login commit between the two commits persists the charge without the pet; `BattlePetMgr::SaveToDB` clears `SaveInfo` to `BATTLE_PET_UNCHANGED` when statements are *appended*, before the commit result is known (`BattlePetMgr.cpp:377`), so the pet is never retried and the loss is silent and permanent; the dependent learned spell is never persisted, leaving no recoverable proof of purchase.

**Deliberate bounded correction.** Rust keeps every C++-observable rule above but closes the two-commit window with a durable saga keyed by a 128-bit `request_key` shared with the #160 Login DB receipt (`battle_pet_add_requests`): (T1) one Character DB transaction deducts the money with a guarded `UPDATE ... WHERE guid=? AND money=?` and inserts the pending command — replay reconciliation requires both the key and the complete immutable payload, so even a random key collision cannot be mistaken for this charge; (T2) the command is applied through `BattlePetAccountOwnerLikeCpp::try_add_pet_like_cpp`, whose Login DB transaction revalidates the fence, the journal lease and the per-species capacity and writes pet + receipt together, so a replay returns the original pet instead of creating another; (T3) packets are queued only after the pet is durable, the `published` marker is then recorded, and completion follows; a `Completed` row with a clear marker remains a recovery-publication signal; (T4) a terminal apply failure is recorded as `CompensationPending`; (T5) the refund and the `Compensated` flip commit in one Character DB transaction, so the refund executes exactly once, and runtime money is reconciled to the absolute durable value; (T6) login recovery converges any interrupted command. Both money-changing commits carry the #159 cancellation fence from the instant COMMIT is awaited through reconciliation, preventing a cancelled future from reopening money persistence with an unknown outcome. Sanctioned behavior deltas vs legacy: admission-time capacity/journal-lock failures return a typed structured result (the wire stays silent exactly like the C++ cap case; no C++ case exists for the journal lock) instead of being indistinguishable from a successful purchase internally, and the trainer list now renders a purchasable battle-pet spell as available because C++ `GetSpellState` has no cap gate. A castable trainer spell with a confirmed species keeps the normal wrapper acquisition but retains the C++-shared silent cap gate and visual-kit suppression (`Trainer.cpp:99-109,121-125`), because C++ resolves the species before `IsCastable()`.

Deliberate deviation recorded for review (local Codex P2, intentionally deferred): the admission gate acquires the #160 journal lease before charging and returns the structured `JournalLocked` unavailable when another session holds it. C++ `TeachSpell` has no journal-lock gate — `AddPet` never checks `HasJournalLock` — but C++'s `BattlePetMgr` is per-session while #160's journal is one account-scoped owner whose lease serializes mutations. The issue explicitly sanctions the admission result ("for capacity or journal lock known at admission, return a structured unavailable result instead of a silent no-op"), and refusing before the charge beats charging and deferring the pet to the next login when the lease is held. The wire stays silent exactly like the C++ cap case; the divergence is inherited from #160's single-lease model, not from this slice. The durable command binds the Battle.net account identity (`battlenet_account_id`), the receipt/owner authority — not the game account id.

**Publication delivery boundary.** A socket enqueue and a Character DB marker cannot be atomic, and the client provides no acknowledgement that could prove exactly-once or at-least-once delivery. Rust therefore chooses the recoverable side of that boundary: durable pet first, packet enqueue second, `published=1` third. Normal execution and settled replay enqueue once; a crash after enqueue but before the marker can cause a re-send attempt on the next login, while a crash can no longer consume the marker before the sole recovery attempt. Pet creation, charge and compensation remain strictly idempotent/exactly-once; enqueue attempts are recoverable and may repeat, but actual network delivery is best-effort. The dependent spell remains runtime-only as C++ requires (`dependent=true`, never a `character_spell` row).

**State model** (`character_battle_pet_purchase.status`, migration `2026_08_03_00_characters.sql`). Five persisted states: `PendingApplication` (0), `Completed` (1), `CompensationPending` (2), `Compensated` (3), `TerminalFailure` (4). The sixth state from the issue's reference model, `PetApplied`, is deliberately *derived*, not persisted: the #160 Login DB receipt is itself the durable "pet applied" fact, and recovery re-derives it by receipt lookup, so a redundant Character DB column could only disagree with the authority.

| Transition | Owner DB | Precondition | Atomic write | Idempotency key | Replay effect | Crash before commit | Crash after commit (reply lost) | Resume |
|---|---|---|---|---|---|---|---|---|
| T1 charge+command | Character | admission revalidation passed (membership, gates, price, money, species metadata, selection inputs, capacity, journal lease) under the exclusive money guard | one tx: guarded money `UPDATE` (expects 1 row) + `INSERT` command `PendingApplication` (expects 1 row) | `request_key` PK + immutable payload comparison | identical-token/payload retry self-reconciles; a colliding key with different payload is a rollback | no charge, no command; client retry gets a fresh key | reconcile by `SELECT` on `request_key` and full payload; exact row ⇒ committed, absent/mismatch ⇒ rolled back, unreadable ⇒ quarantine | recovery treats row as `PendingApplication` |
| T2 apply pet | Login (via #160 owner) | command `PendingApplication`; payload read from the command row | owner tx: fence + capacity + `INSERT battle_pets` + `INSERT battle_pet_add_requests` | `request_key` = receipt PK | receipt replay returns the original pet, never a second one | no pet; recovery retries the add | receipt exists ⇒ next attempt replays | receipt lookup before any add |
| T3 publish+complete | Character | apply returned `Added`/`Replayed` (pet durable, fence revalidated by the owner), or a receipt re-check proves the pet durable for a `CompensationPending` row | enqueue packets, guarded `published=1`, then guarded `status=Completed` | `request_key` + status/marker guards | settled rows are silent | clear marker causes a recovery enqueue attempt | marker/status reads are decisive; a lost marker reply is treated as recorded if observed | recovery re-sends only when `published=0`; enqueue attempts may repeat and delivery is best-effort |
| T4 record compensation | Character | terminal apply failure (capacity, invalid species, key conflict) | guarded `UPDATE status=CompensationPending WHERE key AND status=PendingApplication` | `request_key` + status guard | idempotent | recovery re-applies, fails terminally again, re-decides | status read is decisive | recovery refunds, never re-applies |
| T5 compensate | Character | `CompensationPending` | one tx: refund `UPDATE characters SET money=LEAST(money+price, MAX)` (expects 1 row) + `UPDATE status=Compensated` (expects 1 row) | status flip in the same tx as the refund | replay finds `Compensated` ⇒ no second refund; runtime loads absolute durable money | refund not applied; recovery retries | reconcile by status plus durable absolute money; unreadable ambiguous outcome quarantines | recovery retries until terminal |
| T6 terminal failure | Character | refund matched no character row (character deleted) | guarded `UPDATE status=TerminalFailure` + bounded error diagnostic | `request_key` | — | decision lost; recovery re-derives from the missing character row | status read is decisive | none (operator action); never silently retried |

**Saga invariants.** (1) Money is deducted at most once per exact command (guarded update + PK in one tx; reconciliation compares the full immutable payload). (2) At most one pet per `request_key` (Login DB receipt PK + owner replay). (3) Refund at most once (refund + status flip in one tx), and runtime money is set from the durable absolute value rather than inferred from COMMIT attribution. (4) No publication is attempted before the pet is durable and none is attempted on compensation/failure. Normal settled execution enqueues once, while an enqueue/marker crash window may cause a repeated recovery attempt; without a client ACK, actual delivery remains best-effort. (5) The dependent runtime spell learn is runtime-only, mirroring C++ `dependent=true` (never a `character_spell` row). (6) All recovery is durable-state driven — no in-memory flags decide whether a charge, pet or refund already happened. (7) No lock guard crosses `.await` except the pre-existing async per-character money mutex (#159 pattern); both Character money COMMIT windows are covered by its cancellation quarantine fence. The saga spawns no tasks of its own, and the only worker is the #160 owner worker drained by the registry's bounded shutdown.

## 2026-06-07 scope freeze

Recent Rust slices represented parts of the forward-ported `BattlePetMgr` surface from `/home/server/woltk-trinity-legacy` (journal packets, slots, summon/name handlers, errors, updates, uncage helpers). That work is **preserved** because it may be useful if RustyCore later targets a MoP+ or modern-client feature set.

For the current WotLK port, however, BattlePets is frozen:

- Do not select new BattlePet runtime slices for WotLK progress.
- Do not count BattlePet represented work toward the honest WotLK completion percentage.
- Keep existing code dormant or compatibility-only unless the target version explicitly changes.
- If a 3.4.3 client path needs pet work, route it through WotLK systems: hunter/warlock pets, minions, stable, cosmetic critter/companion summon, pet spellbook, and normal item/spell learning.

---

## 1. Purpose

In retail WoW, BattlePets is the Pokémon-style mini-game introduced in **Mists of Pandaria (5.0.4, October 2012)** — collect 1500+ unique companion species, level them 1→25, train abilities, and 3v3 turn-based PvE/PvP. The `BattlePetMgr` per-session manager owns the player's collection (the "journal"), the 3 active battle slots, the trap level, and the cage-item conversion that lets a battle pet become a tradeable cage item (item id 82800, MoP+).

**This feature does not exist in WoLK 3.4.3.** The 3.4.3.54261 client (Wrath of the Lich King Classic re-release) was branched off the WoLK 3.3.5 era; companion pets in that client are pure cosmetic critters resolved via the regular `summon_companion` spell effect → `Player::SetCritterGUID`, with no levelling, abilities, or battles. The reason `BattlePetMgr.{h,cpp}` (929 lines + 217 lines) exists at all in `/home/server/woltk-trinity-legacy/` is that this is the modern Trinity master (not 3.3.5 branch); it's been left in the repository because the build still pulls some shared opcode definitions from there. The legacy fork includes the code but the system is never wired to a 3.4.3 session — `WorldSession::GetBattlePetMgr()` returns nullptr in 3.4.3 mode.

Bottom line: **document so future maintainers don't re-implement** when a confused client sends `BattlePetRequestJournal`, and to record the deliberate "N/A" decision.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/BattlePets/BattlePetMgr.cpp` | 929 | `prefix` |
| `game/BattlePets/BattlePetMgr.h` | 217 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/BattlePets/BattlePetMgr.h` | 217 | `namespace BattlePets`. `BattlePetMisc` constants (`DEFAULT_MAX_BATTLE_PETS_PER_SPECIES = 3`, `BATTLE_PET_CAGE_ITEM_ID = 82800`, `SPELL_VISUAL_UNCAGE_PET = 222`, `SPELL_BATTLE_PET_TRAINING = 125610`, `SPELL_REVIVE_BATTLE_PETS = 125439`, `SPELL_SUMMON_BATTLE_PET = 118301`). `MAX_BATTLE_PET_LEVEL = 25`. `BattlePetBreedQuality` (Poor/Common/Uncommon/Rare/Epic/Legendary). `BattlePetDbFlags` (Favorite=0x001, Converted=0x002, Revoked=0x004, LockedForConvert=0x008, Ability0/1/2Selection=0x010/0x020/0x040, FanfareNeeded=0x080, DisplayOverridden=0x100). `BattlePetError` (CantHaveMorePetsOfType=3, CantHaveMorePets=4, TooHighLevelToUncage=7). `BattlePetSlot` (Slot0/1/2). `BattlePetXpSource` (PetBattle=0, SpellEffect=1). `BattlePetState` enum mirroring 16 `BattlePetState.db2` rows. `BattlePet` struct (PacketInfo, NameTimestamp, DeclinedName, SaveInfo). `BattlePetMgr` class. |
| `src/server/game/BattlePets/BattlePetMgr.cpp` | 929 | Implementation. Static loaders: `Initialize` (max-guid bump from `battle_pets` table), `LoadAvailablePetBreeds`, `LoadDefaultPetQualities`. Static helpers: `AddBattlePetSpeciesBySpell`, `GetBattlePetSpeciesByCreature`, `GetBattlePetSpeciesBySpell`, `RollPetBreed`, `GetDefaultPetQuality`, `SelectPetDisplay`. Per-session: `LoadFromDB`, `SaveToDB(LoginDatabaseTransaction)`, `GetPet`, `AddPet`, `RemovePet`, `ClearFanfare`, `ModifyName`, `IsPetInSlot`, `GetPetCount`, `HasMaxPetCount`, `GetPetUniqueSpeciesCount`, `UnlockSlot`, `CageBattlePet` (creates the 82800 item), `ChangeBattlePetQuality`, `GrantBattlePetExperience`, `GrantBattlePetLevel`, `HealBattlePetsPct`, `UpdateBattlePetData`, `SummonPet`, `DismissPet`, `SendJournal`, `SendUpdates`, `SendError`, `SendJournalLockStatus`, `IsJournalLockAcquired`, `IsBattlePetSystemEnabled`. `BattlePet::CalculateStats` (the breed × species × quality × level scaling formula). |
| `src/server/game/Handlers/BattlePetHandler.cpp` | 134 | All 9 BattlePet client opcodes: `HandleBattlePetRequestJournal` (→ `SendJournal`), `HandleBattlePetRequestJournalLock` (→ `SendJournalLockStatus`, plus `SendJournal` if lock held), `HandleBattlePetSetBattleSlot` (assign pet to slot 0/1/2), `HandleBattlePetModifyName`, `HandleQueryBattlePetName` (look up pet name + declined names by `BattlePetID` GUID), `HandleBattlePetDeletePet` (→ `RemovePet`), `HandleBattlePetSetFlags` (apply/remove `BattlePetDbFlags`), `HandleBattlePetClearFanfare`, `HandleCageBattlePet`, `HandleBattlePetSummon` (toggle: if same as currently summoned → dismiss; else → summon), `HandleBattlePetUpdateNotify`. |
| `src/server/game/Server/Packets/BattlePetPackets.h` (out of scope, in WorldPackets) | ~600 | All ~25 packet structs: `BattlePetJournal`, `BattlePetSlot` (sub-struct), `BattlePet` (sub-struct: GUID, Species, Breed, DisplayID, CollarID, Level, Exp, BattlePetDBFlags, Name, MaxHealth, Health, Power, Speed, Quality, BreedQuality), `BattlePetRequestJournal`, `BattlePetRequestJournalLock`, `BattlePetJournalLockAcquired`, `BattlePetJournalLockDenied`, `BattlePetUpdates`, `BattlePetSetBattleSlot`, `BattlePetSetFlags`, `BattlePetSummon`, `BattlePetUpdateDisplayNotify`, `BattlePetUpdateNotify`, `BattlePetClearFanfare`, `BattlePetModifyName`, `BattlePetError`, `BattlePetDeleted`, `BattlePetRevoked`, `BattlePetRestored`, `BattlePetsHealed`, `BattlePetTrapLevel`, `BattlePetCageDateError`, `CageBattlePet`, `QueryBattlePetName`, `QueryBattlePetNameResponse`. |

DB2 stores referenced (via `DB2Stores.h`) — these are MoP+ tables that wouldn't be in a 3.4.3 client:

- `BattlePetSpeciesEntry` (BattlePetSpecies.db2) — the species table (bear, sprite darter, etc.)
- `BattlePetBreedQualityEntry` (BattlePetBreedQuality.db2) — quality multipliers
- `BattlePetSpeciesXAbility` (per-species abilities, 3 selectable from ~6)
- `BattlePetBreedState.db2` (per-breed base stats)
- `BattlePetSpeciesState.db2` (per-species state overrides)

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `BattlePetMgr` | per-session class | Lives on `WorldSession` (in modern TC — not on `Player`) since the journal persists to the **login** database (account-wide), not characters DB. |
| `BattlePet` | struct | One owned pet (`PacketInfo: WorldPackets::BattlePet::BattlePet`, `NameTimestamp`, `DeclinedName`, `SaveInfo`). |
| `BattlePetSaveInfo` | enum | `UNCHANGED=0, CHANGED=1, NEW=2, REMOVED=3` — incremental save state machine (mirrors PetSpellState pattern). |
| `BattlePetBreedQuality` | enum class (u8) | Poor=0, Common=1, Uncommon=2, Rare=3, Epic=4, Legendary=5. |
| `BattlePetDbFlags` | bitflag (u16) | 9 flags. The `Ability0/1/2Selection` ones are mutually exclusive within their slot — they encode which of the species' ability options is selected. `FanfareNeeded` is set on first capture and cleared on first summon (the gold-sparkle UX). |
| `BattlePetError` | enum class (u8) | `CantHaveMorePetsOfType=3, CantHaveMorePets=4, TooHighLevelToUncage=7`. Send via `SMSG_BATTLE_PET_ERROR`. |
| `BattlePetSlot` | enum class (u8) | Slot0/1/2/Count — the 3 active battle slots. |
| `BattlePetXpSource` | enum class (u8) | PetBattle=0, SpellEffect=1 — XP grant origin. |
| `FlagsControlType` | enum | APPLY=1, REMOVE=2 — used in `HandleBattlePetSetFlags`. |
| `BattlePetState` | enum (mixed) | 16 named values mirroring `BattlePetState.db2` rows: `STATE_MAX_HEALTH_BONUS=2`, `STATE_INTERNAL_INITIAL_LEVEL=17`, `STATE_STAT_POWER=18`, `STATE_STAT_STAMINA=19`, `STATE_STAT_SPEED=20`, `STATE_MOD_DAMAGE_DEALT_PERCENT=23`, `STATE_GENDER=78`, `STATE_COSMETIC_WATER_BUBBLED=85`, `STATE_SPECIAL_IS_COCKROACH=93`, `STATE_COSMETIC_FLY_TIER=128`, `STATE_COSMETIC_BIGGLESWORTH=144`, `STATE_PASSIVE_ELITE=153`, `STATE_PASSIVE_BOSS=162`, `STATE_COSMETIC_TREASURE_GOBLIN=176`, `STATE_START_WITH_BUFF=183`, `STATE_START_WITH_BUFF_2=184`, `STATE_COSMETIC_SPECTRAL_BLUE=196`. |
| Constants | — | `MAX_BATTLE_PET_LEVEL=25`, `DEFAULT_MAX_BATTLE_PETS_PER_SPECIES=3` (cap on owning duplicates of the same species), `BATTLE_PET_CAGE_ITEM_ID=82800` (the tradeable cage), `SPELL_VISUAL_UNCAGE_PET=222`, `SPELL_BATTLE_PET_TRAINING=125610`, `SPELL_REVIVE_BATTLE_PETS=125439`, `SPELL_SUMMON_BATTLE_PET=118301`. |

Wire packet sub-struct (`WorldPackets::BattlePet::BattlePet`):

| Field | Type | Notes |
|---|---|---|
| `Guid` | ObjectGuid (HighGuid::BattlePet) | Server-generated; persisted in `battle_pets.guid` |
| `Species` | u32 | BattlePetSpecies.db2 row id |
| `Breed` | u32 | BattlePetBreedState.db2 row id |
| `DisplayID` | u32 | Override display (some pets have variable models — DisplayOverridden flag controls) |
| `CollarID` | u32 | Cosmetic (post-MoP); usually 0 |
| `Level` | u16 | 1-25 |
| `Exp` | u16 | XP-toward-next-level |
| `Flags` | u16 | BattlePetDbFlags |
| `Name` | string |  |
| `MaxHealth` | u32 | Computed by `BattlePet::CalculateStats` |
| `Health` | u32 |  |
| `Power` | u32 |  |
| `Speed` | u32 |  |
| `Quality` | u8 | BattlePetBreedQuality |
| `BreedQuality` | u8 | Sometimes diverges from Quality |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `BattlePetMgr::Initialize()` (static) | Read `MAX(guid) FROM battle_pets` (login DB), bump the `HighGuid::BattlePet` generator. Call `LoadAvailablePetBreeds()` and `LoadDefaultPetQualities()`. | `LoginDatabase.Query`, `sObjectMgr->GetGenerator<HighGuid::BattlePet>()` |
| `BattlePetMgr::LoadAvailablePetBreeds()` (static) | Populate `_availableBreedsPerSpecies` from the `battle_pet_breeds` world DB table or `BattlePetSpeciesXBreed.db2` (varies by TC version). | — |
| `BattlePetMgr::LoadDefaultPetQualities()` (static) | Populate `_defaultQualityPerSpecies` from `battle_pet_quality_overrides` world DB table. | — |
| `BattlePetMgr::AddBattlePetSpeciesBySpell(spellId, speciesEntry)` (static) | Build the spell→species index used at summon time. | — |
| `BattlePetMgr::GetBattlePetSpeciesByCreature(creatureId)` (static) | Lookup helper: `BattlePetSpecies.db2[CreatureID == ?]`. | — |
| `BattlePetMgr::GetBattlePetSpeciesBySpell(spellId)` (static) | Lookup helper. | — |
| `BattlePetMgr::RollPetBreed(species)` (static) | Random pick from `_availableBreedsPerSpecies[species]`. | `Trinity::Containers::SelectRandomContainerElement` |
| `BattlePetMgr::GetDefaultPetQuality(species)` (static) | Default quality lookup with species-specific overrides. | — |
| `BattlePetMgr::SelectPetDisplay(speciesEntry)` (static) | If species has multiple displays, pick one weighted by `BattlePetSpeciesEntry.Flags`. | — |
| `BattlePetMgr::LoadFromDB(pets, slots)` | Per-session: deserialize `battle_pets` rows into `_pets` map; deserialize `battle_pet_slots` rows into `_slots[3]`. Sets `_trapLevel` from `account_trap_level` table or default. | `Player::HasSpell(SPELL_BATTLE_PET_TRAINING)` |
| `BattlePetMgr::SaveToDB(LoginDatabaseTransaction trans)` | Walk `_pets`, dispatch by `SaveInfo`: NEW → INS, CHANGED → UPD, REMOVED → DEL, UNCHANGED → skip. Reset to UNCHANGED after. | `LOGIN_INS_BATTLE_PET`, `LOGIN_UPD_BATTLE_PET`, `LOGIN_DEL_BATTLE_PET` |
| `BattlePetMgr::GetPet(guid)` | Map lookup. | — |
| `BattlePetMgr::AddPet(species, display, breed, quality, level=1)` | Generate `HighGuid::BattlePet`, insert into `_pets` with `SaveInfo=NEW`, send `BattlePetUpdates(petAdded=true)`. | `BattlePet::CalculateStats` |
| `BattlePetMgr::RemovePet(guid)` | Mark `SaveInfo=REMOVED`. (Actual DB delete happens in next `SaveToDB`.) Decrement `_petUniqueSpeciesCount` if last of species. Broadcast `BattlePetDeleted`. | — |
| `BattlePetMgr::ClearFanfare(guid)` | Strip `BattlePetDbFlags::FanfareNeeded`. | — |
| `BattlePetMgr::ModifyName(guid, name, declinedNames)` | Rename + declined name persistence. | — |
| `BattlePetMgr::CageBattlePet(guid)` | Create item 82800 in player inventory with the pet's data encoded in item modifiers; mark pet `Revoked`. | `Player::CanStoreNewItem`, `Player::StoreNewItem`, `Item::SetModifier(ITEM_MODIFIER_BATTLE_PET_*)` |
| `BattlePetMgr::ChangeBattlePetQuality(guid, quality)` | Re-roll stats with new quality (admin command / event item). | `BattlePet::CalculateStats` |
| `BattlePetMgr::GrantBattlePetExperience(guid, xp, source)` | Add XP, level up while `xp >= nextLevelXp && level < 25`. | `GrantBattlePetLevel` |
| `BattlePetMgr::GrantBattlePetLevel(guid, grantedLevels)` | Direct level grants (used by `SPELL_BATTLE_PET_TRAINING` and admin commands). | `BattlePet::CalculateStats` |
| `BattlePetMgr::HealBattlePetsPct(pct)` | `SPELL_REVIVE_BATTLE_PETS` (125439) — heal each pet by `pct%` of max health. | — |
| `BattlePetMgr::SummonPet(guid)` | Cast `SPELL_SUMMON_BATTLE_PET` (118301) on owner; sets `SummonedBattlePetGUID`. | `Player::CastSpell` |
| `BattlePetMgr::DismissPet()` | Remove `SummonedBattlePetGUID`'s active critter. | `Unit::RemoveAllMinionsByEntry` |
| `BattlePetMgr::SendJournal()` | Build `SMSG_BATTLE_PET_JOURNAL` with all pets + slots. | `SendPacket` |
| `BattlePetMgr::SendUpdates(pets, petAdded)` | Build `SMSG_BATTLE_PET_UPDATES` for incremental sync. | — |
| `BattlePetMgr::SendError(error, creatureId)` | Build `SMSG_BATTLE_PET_ERROR{ Result, CreatureID }`. | — |
| `BattlePetMgr::SendJournalLockStatus()` | If lock acquired → `SMSG_BATTLE_PET_JOURNAL_LOCK_ACQUIRED`; else `SMSG_BATTLE_PET_JOURNAL_LOCK_DENIED`. | — |
| `BattlePet::CalculateStats()` | The breed × species × quality × level formula. `breed_state.STAT_STAMINA + species_state.STAT_STAMINA` then × `quality.StateMultiplier` then × `Level`, finally rounded as `MaxHealth = (round(health/20) + 100)`, `Power = round(power/100)`, `Speed = round(speed/100)`. | `_battlePetBreedStates`, `_battlePetSpeciesStates`, `BattlePetBreedQualityEntry` |

---

## 5. Module dependencies

**Depends on (in retail TC):**
- `DB2Stores` — `BattlePetSpeciesStore`, `BattlePetBreedStateStore`, `BattlePetSpeciesStateStore`, `BattlePetBreedQualityStore`, `BattlePetAbilityStore`, `BattlePetSpeciesXAbilityStore`. None of these DB2 tables ship with the 3.4.3 client.
- `LoginDatabase` — `battle_pets` (account-wide), `battle_pet_slots`, plus the prepared statements `LOGIN_INS_BATTLE_PET`, `LOGIN_UPD_BATTLE_PET`, `LOGIN_DEL_BATTLE_PET`, `LOGIN_INS_BATTLE_PET_DECLINED_NAME`, etc.
- `WorldDatabase` — `battle_pet_quality_overrides`, `battle_pet_breeds` (or read from DB2 in newer TC).
- `Item` — for cage item creation (`ITEM_MODIFIER_BATTLE_PET_SPECIES_ID` etc., 5 modifier slots used).
- `Spell` — `SPELL_SUMMON_BATTLE_PET` (118301) effect, `SPELL_REVIVE_BATTLE_PETS` (125439), `SPELL_BATTLE_PET_TRAINING` (125610), pet-battle abilities.
- `Player` — `Player::SetSummonedBattlePetGUID`, `Player::SetBattlePetData`, `Player::GetCritterGUID`.
- `WorldSession` — owns the `BattlePetMgr` (via `_battlePetMgr` member).

**Depended on by:**
- `WorldSession::HandleDismissCritter` — checks `_player->GetSummonedBattlePetGUID() == pet->GetBattlePetCompanionGUID()` to clear battle pet data on dismiss. **This branch is reachable in 3.4.3 if a summoned critter happens to also have a non-zero `BattlePetCompanionGUID`** — but in practice it's always zero in 3.4.3 because no spell sets it.
- Pet battle scripts (PvE encounters, captured wild pets) — entirely absent in 3.4.3.
- `ObjectMgr::GetGenerator<HighGuid::BattlePet>()` — the GUID generator. Even if unused, the generator should exist if any code references the type.

---

## 6. SQL / DB queries (if any)

Schema (login DB, retail TC — **not present in RustyCore login schema and won't be**):

```sql
CREATE TABLE battle_pets (
  guid          BIGINT UNSIGNED PRIMARY KEY,
  ownerAccount  INT UNSIGNED,
  species       INT UNSIGNED,
  breed         SMALLINT UNSIGNED,
  displayId     INT UNSIGNED,
  level         SMALLINT UNSIGNED,
  exp           SMALLINT UNSIGNED,
  health        INT UNSIGNED,
  quality       TINYINT UNSIGNED,
  flags         SMALLINT UNSIGNED,
  name          VARCHAR(31),
  nameTimestamp INT UNSIGNED,
  ...
);
CREATE TABLE battle_pet_slots (account INT UNSIGNED, slot TINYINT, guid BIGINT UNSIGNED, PRIMARY KEY(account, slot));
CREATE TABLE battle_pet_declined_name (...);
```

| Statement | Purpose | DB |
|---|---|---|
| `LOGIN_SEL_BATTLE_PETS` | `SELECT ... FROM battle_pets WHERE ownerAccount = ?` | login |
| `LOGIN_SEL_BATTLE_PET_SLOTS` | per-account active slots | login |
| `LOGIN_INS_BATTLE_PET` | new pet | login |
| `LOGIN_UPD_BATTLE_PET` | mutation save | login |
| `LOGIN_DEL_BATTLE_PET` | delete | login |
| `LOGIN_REP_BATTLE_PET_SLOTS` | replace active slots | login |
| (declined name CRUD) | | login |

DB2 stores: see §5 — all are 5.x+ tables. **The 3.4.3 client does not have these in its data files.**

---

## 7. Wire-protocol packets (if any)

The opcodes are **already present in `crates/wow-constants/src/opcodes.rs`** (see lines 85-92, 468, 775, 792-802, 1369). They were added during a prior batch by mistake or during opcode-table generation; they should be left in place because removing constants is more disruptive than leaving them unused.

| Opcode | Direction | C++ handler |
|---|---|---|
| `CMSG_BATTLE_PET_REQUEST_JOURNAL` (0x3625) | C→S | `HandleBattlePetRequestJournal` |
| `CMSG_BATTLE_PET_REQUEST_JOURNAL_LOCK` (0x3624) | C→S | `HandleBattlePetRequestJournalLock` |
| `CMSG_BATTLE_PET_SET_BATTLE_SLOT` (0x362e) | C→S | `HandleBattlePetSetBattleSlot` |
| `CMSG_BATTLE_PET_SET_FLAGS` (0x3631) | C→S | `HandleBattlePetSetFlags` |
| `CMSG_BATTLE_PET_SUMMON` (0x362a) | C→S | `HandleBattlePetSummon` |
| `CMSG_BATTLE_PET_UPDATE_NOTIFY` (0x31df) | C→S | `HandleBattlePetUpdateNotify` |
| `CMSG_BATTLE_PET_UPDATE_DISPLAY_NOTIFY` (0x31e0) | C→S | `HandleBattlePetUpdateDisplayNotify` (TC stub) |
| `CMSG_BATTLE_PET_CLEAR_FANFARE` (0x3126) | C→S | `HandleBattlePetClearFanfare` |
| `CMSG_QUERY_BATTLE_PET_NAME` (0x3276) | C→S | `HandleQueryBattlePetName` |
| `SMSG_BATTLE_PET_JOURNAL` (0x25ef) | S→C | `BattlePetMgr::SendJournal` |
| `SMSG_BATTLE_PET_JOURNAL_LOCK_ACQUIRED` (0x25ed) | S→C | `BattlePetMgr::SendJournalLockStatus` |
| `SMSG_BATTLE_PET_JOURNAL_LOCK_DENIED` (0x25ee) | S→C | `BattlePetMgr::SendJournalLockStatus` |
| `SMSG_BATTLE_PETS_HEALED` (0x25f3) | S→C | `BattlePetMgr::HealBattlePetsPct` |
| `SMSG_BATTLE_PET_DELETED` (0x25f0) | S→C | `BattlePetMgr::RemovePet` |
| `SMSG_BATTLE_PET_RESTORED` (0x25f2) | S→C | (admin restore) |
| `SMSG_BATTLE_PET_REVOKED` (0x25f1) | S→C | `BattlePetMgr::CageBattlePet` |
| `SMSG_BATTLE_PET_ERROR` (0x2638) | S→C | `BattlePetMgr::SendError` |
| `SMSG_BATTLE_PET_TRAP_LEVEL` (0x25eb) | S→C | (per-account trap upgrade) |
| `SMSG_BATTLE_PET_UPDATES` (0x25ea) | S→C | `BattlePetMgr::SendUpdates` |
| `SMSG_BATTLE_PET_CAGE_DATE_ERROR` (0x2678) | S→C | (cage date validation) |
| `SMSG_BATTLE_PAY_BATTLE_PET_DELIVERED` (0x277d) | S→C | (BattlePay shop, post-MoP) |
| `SMSG_QUERY_BATTLE_PET_NAME_RESPONSE` (0x291a) | S→C | `HandleQueryBattlePetName` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/opcodes.rs` — 22 BattlePet-related opcode constants present (see §7).
- `crates/wow-packet/src/packets/misc.rs:1005-1015` — `pub struct BattlePetJournalLockAcquired;` (zero-byte unit struct) implementing `ServerPacket` with `OPCODE = ServerOpcodes::BattlePetJournalLockAcquired`. Plus a unit test at `:2382` building the packet.
- `crates/wow-world/src/handlers/character.rs:4254-4255` — sends `BattlePetJournalLockAcquired` during `send_initial_packets_after_load` (the post-login flow), comment "BattlePetJournalLockAcquired (empty packet — journal access granted)". This may actually be incorrect for 3.4.3 — the right semantic is _Denied_, not _Acquired_, because there is no journal — see §11.
- `crates/wow-world/src/handlers/misc.rs:606` — `pub async fn handle_battle_pet_request_journal(&mut self, _pkt: WorldPacket) {}` — empty body. Registered in the handler-entry table at `:266`.
- `crates/wow-world/src/session.rs:1817` — dispatcher routes `ClientOpcodes::BattlePetRequestJournal` → `handle_battle_pet_request_journal(pkt)`.
- No other BattlePet code anywhere. No `BattlePetMgr`, no `BattlePet` struct, no DB2 stores, no login-DB schema, no other handlers.

**What's implemented:**
- A single-packet defensive ack on login (`BattlePetJournalLockAcquired`) and an empty handler for `CMSG_BATTLE_PET_REQUEST_JOURNAL`.

**What's missing vs C++:**
- Everything else. But that's intentional. **Do not implement.**

**Suspicious / likely divergent:**
- The C++ flow on login is `HandleBattlePetRequestJournalLock` → `SendJournalLockStatus` (which sends *either* Acquired or Denied based on `IsJournalLockAcquired()`), then `SendJournal`. The current Rust flow proactively sends `BattlePetJournalLockAcquired` without being asked, before any `CMSG_BATTLE_PET_REQUEST_JOURNAL_LOCK`. A 3.4.3 client likely never sends that opcode anyway, but if it does, our handler is empty and returns nothing — the client will sit waiting for an `SMSG_BATTLE_PET_JOURNAL` that never comes.
- If a 3.4.3 client somehow sends `CMSG_BATTLE_PET_REQUEST_JOURNAL` (it doesn't — but if it does), our handler discards silently. That's fine. Maybe upgrade to "send `BattlePetJournalLockDenied`" for explicit rejection if any client ever exposes the journal UI. This is a one-line addition.
- The other 8 CMSG_BATTLE_PET_* opcodes are listed in `ClientOpcodes` but **have no dispatcher arms in `session.rs`** — they will hit the unknown-opcode default branch and be logged. That's the correct behavior.

**Tests existing:**
- 1 — the round-trip test for `BattlePetJournalLockAcquired` at `crates/wow-packet/src/packets/misc.rs:2382`. Fine as-is.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#BATTLEPETS.WBS.001** Partir y cerrar la migracion auditada de `game/BattlePets/BattlePetMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp`
  Rust target: `crates/wow-packet/src/packets/misc.rs`, `crates/wow-world/src/handlers/character.rs`, `crates/wow-world/src/handlers/misc.rs`, `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 929 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BATTLEPETS.WBS.002** Cerrar la migracion auditada de `game/BattlePets/BattlePetMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.h`
  Rust target: `crates/wow-packet/src/packets/misc.rs`, `crates/wow-world/src/handlers/character.rs`, `crates/wow-world/src/handlers/misc.rs`, `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

**These tasks are mostly intentionally not-to-do. Listed for completeness.**

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [x] **#BPETS.1 (superseded design)** Do not implement another per-session manager: #160 introduces one shared account-scoped owner and leaves the session-local collection only as a unit-test fallback. Full gameplay remains partial. (XL)
- [x] **#BPETS.2** Login-DB `battle_pets`, `battle_pet_slots`, and declined-name persistence is active; #160 adds the durable `battle_pet_add_requests` idempotency receipt migration. (M)
- [ ] **#BPETS.3** **(WONTFIX in 3.4.3)** Port the 9 CMSG handlers and ~15 SMSG packets. (XL)
- [ ] **#BPETS.4** **(LOW PRIORITY, audit)** Verify the post-login `BattlePetJournalLockAcquired` send is correct vs the 3.4.3 client. If the client never opens the journal UI, this packet is harmless filler — but if a confused addon triggers a journal request, sending Acquired then nothing else is incorrect. Consider switching to `BattlePetJournalLockDenied` (define the packet type if not present), or remove the proactive send entirely (the client doesn't ask for it, so we wouldn't need to send anything). (L)
- [ ] **#BPETS.5** **(LOW PRIORITY, audit)** Walk all 9 CMSG_BATTLE_PET_* opcodes and confirm they all silently-default in `session.rs` without crashing the dispatcher. Add explicit log-and-discard arms if any are not handled. (L)
- [ ] **#BPETS.6** **(DOCUMENTATION)** Add a comment at `BattlePetJournalLockAcquired` send site explaining "intentionally a no-op ack; full BattlePet system is post-3.4.3 and not implemented." (L)
- [ ] **#BPETS.7** **(LOW PRIORITY)** Decide: should `crates/wow-world/src/handlers/misc.rs` have an `async fn handle_battle_pet_request_journal_lock` stub (it currently doesn't because the opcode dispatcher in `session.rs` has no arm for `BattlePetRequestJournalLock`)? Probably no, but log the absence. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#BATTLEPETS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 1146 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.h`. Rust target: `workspace / target pending`. | `cargo test --workspace` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#BATTLEPETS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 1146 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.h`. Rust target: `workspace / target pending`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#BATTLEPETS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 1146 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.h`. Rust target: `workspace / target pending`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#BATTLEPETS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 1146 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.h`. Rust target: `workspace / target pending`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `BattlePetJournalLockAcquired` serializes to exactly `[]` bytes (empty body), opcode `0x25ed`. (Already covered by the existing test.)
- [ ] Test: A login flow simulation observes the proactive `BattlePetJournalLockAcquired` send in the correct ordering position (after `FeatureSystemStatus`, before `TimeSyncRequest`). Catch ordering regressions.
- [ ] Test (negative): Sending `CMSG_BATTLE_PET_REQUEST_JOURNAL` (0x3625) to the dispatcher does NOT crash the session and produces no SMSG response.
- [ ] Test (negative): Sending any of the other 8 BattlePet CMSG opcodes lands in the unknown-opcode default branch and is logged (not silently swallowed without a log entry).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `product_out_of_scope_or_post_wolk` | Keep C++ coverage assigned; implementation may be stubbed/disabled only with explicit client behavior and tests/n/a recorded. | 2 files / 1146 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.h` | No target crate intended for 3.4.3 client. Today only one packet stub exists: `crates/wow-packet/src/packets/misc.rs:1005` defines `BattlePetJournalLockAcquired` (zero-byte ServerPacket), used by `crates/wow-world/src/handlers/character.rs:4255` during the post-login packet pipeline (defensive ack so a 3.4.3 client that hasn't been told the journal is unavailable doesn't sit waiting). The single CMSG handler stub is `handle_battle_pet_request_journal` at `crates/wow-world/src/handlers/misc.rs:606` (empty body, registered at `:266`). \| ⚠️ **N/A for WoLK 3.4.3.** The opcodes exist in the WoLK Trinity-Legacy codebase because that fork tracks a forward-ported feature surface, but a 3.4.3.54261 client has no Pet Battle UI, no companion summoning that resolves to a `BattlePet` species entry, and no journal screen. The only thing the Rust port needs to do is **not break the login flow** by sending a token `BattlePetJournalLockAcquired` (already done) and silently accept any of the 9 CMSG_BATTLEPET_* opcodes if a modded client ever sends them. **Do not implement the system.** |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#BATTLEPETS.DIV.001` | _none generated_ | 2 C++ files / 1146 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **`BattlePets` is the right call to skip.** Every minute spent porting this is a minute not spent on hunter pets, which 3.4.3 does need.
- **`BattlePetJournalLockAcquired` was likely added on a guess.** The current send happens unconditionally during the initial packets; the C++ flow only sends it in response to `CMSG_BATTLE_PET_REQUEST_JOURNAL_LOCK` and only if `IsJournalLockAcquired()`. Sending it proactively to a 3.4.3 client is harmless (the client doesn't render a journal anyway) but **semantically wrong** — the client never asks. If the 3.4.3 client ignores unsolicited 0x25ed, fine; if it doesn't, the right thing is to either (a) remove the proactive send, or (b) switch to `BattlePetJournalLockDenied` (0x25ee). Verify against a packet log from a vanilla 3.4.3.54261 client.
- **Account-wide vs character-wide.** BattlePets persist to the **login** database (account-wide), unlike hunter pets which are character-wide in `character_pet`. This is by design — Blizzard wanted players to share pets across alts on the same account.
- **`HighGuid::BattlePet` is a separate GUID type.** Modern Trinity has a `HighGuid::BattlePet` enum value with its own per-account generator; our 3.4.3 ObjectGuid system likely doesn't define it. Don't define it. If it ever needs to be added, mark with a `// post-3.4.3` comment.
- **Item modifier slots** for the cage item (82800): 5 of them — `ITEM_MODIFIER_BATTLE_PET_SPECIES_ID`, `ITEM_MODIFIER_BATTLE_PET_BREED_DATA`, `ITEM_MODIFIER_BATTLE_PET_LEVEL`, `ITEM_MODIFIER_BATTLE_PET_DISPLAY_ID`, plus one more. These don't exist in 3.4.3's `Item` field layout.
- **MaxHealth formula has a `+100` floor.** `BattlePet::CalculateStats` does `MaxHealth = (round(health/20) + 100)`. The `+100` is a constant base health pool — without it, level-1 pets would have 1 HP. Mirror exactly if anyone ever ports.
- **`LUA_EVAL_CHECK` four-digit limit on Warden** is unrelated but worth noting in case someone confuses BattlePet pet-battle Lua scripting with Warden Lua eval — they're entirely different code paths.
- **The "fanfare" UX**: `FanfareNeeded=0x080` is set on first capture and triggers a gold-sparkle effect when summoning. `ClearFanfare` is called as soon as the player clicks summon for the first time. This is a 5.x+ feel-good detail — not relevant for 3.4.3.
- **`WardenWin` and BattlePets share the `enuminfo_*` smart-enum mechanism**. If anyone ever renames `BattlePetSlot::Slot0` etc., make sure the `enuminfo_BattlePetMgr.cpp` (auto-generated, not in this directory but in the build tree) keeps up. Not relevant for Rust port.

---

## 12. C++ → Rust mapping (high-level)

(For completeness; **none of these are intended to be implemented for 3.4.3**.)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class BattlePets::BattlePetMgr` (per WorldSession) | `pub struct BattlePetMgr { _pets: HashMap<u64, BattlePet>, _slots: [BattlePetSlot; 3], _has_journal_lock: bool, _trap_level: u16 }` | Would live as `Option<BattlePetMgr>` on `WorldSession` |
| `struct BattlePet` | `pub struct BattlePet { packet_info: BattlePetWire, name_timestamp: i64, declined_name: Option<DeclinedName>, save_info: BattlePetSaveInfo }` | — |
| `enum BattlePetSaveInfo` | `#[repr(u8)] pub enum BattlePetSaveInfo { Unchanged=0, Changed=1, New=2, Removed=3 }` | Same incremental-save pattern as Pet |
| `enum class BattlePetBreedQuality : u8` | `#[repr(u8)] pub enum BattlePetBreedQuality { Poor=0, Common=1, Uncommon=2, Rare=3, Epic=4, Legendary=5 }` | — |
| `EnumFlag<BattlePetDbFlags>` | `bitflags! { pub struct BattlePetDbFlags: u16 { const FAVORITE = 0x001; const CONVERTED = 0x002; const REVOKED = 0x004; const LOCKED_FOR_CONVERT = 0x008; const ABILITY_0_SELECTION = 0x010; const ABILITY_1_SELECTION = 0x020; const ABILITY_2_SELECTION = 0x040; const FANFARE_NEEDED = 0x080; const DISPLAY_OVERRIDDEN = 0x100; }}` | — |
| `enum class BattlePetError : u8` | `#[repr(u8)] pub enum BattlePetError { CantHaveMorePetsOfType = 3, CantHaveMorePets = 4, TooHighLevelToUncage = 7 }` | — |
| `enum class BattlePetSlot : u8` | `#[repr(u8)] pub enum BattlePetSlot { Slot0=0, Slot1=1, Slot2=2 }` plus `pub const COUNT: usize = 3;` | — |
| `enum BattlePetState` | `#[repr(u32)] pub enum BattlePetState { ... 16 entries ... }` | Wire-stable ID values from BattlePetState.db2 |
| `MAX_BATTLE_PET_LEVEL = 25` | `pub const MAX_BATTLE_PET_LEVEL: u16 = 25;` | — |
| `BATTLE_PET_CAGE_ITEM_ID = 82800` | `pub const BATTLE_PET_CAGE_ITEM_ID: u32 = 82800;` | — |
| `WorldPackets::BattlePet::BattlePet` (wire) | `pub struct BattlePetWire { pub guid: ObjectGuid, pub species: u32, pub breed: u32, pub display_id: u32, pub collar_id: u32, pub level: u16, pub exp: u16, pub flags: u16, pub name: String, pub max_health: u32, pub health: u32, pub power: u32, pub speed: u32, pub quality: u8, pub breed_quality: u8 }` | Don't add unless implementing |
| `BattlePetMgr::SaveToDB(LoginDatabaseTransaction)` | `async fn save_to_db(&mut self, tx: &mut Transaction) -> Result<()>` | login DB |
| `unordered_map<u64, BattlePet> _pets` | `HashMap<u64, BattlePet>` (key = battle-pet GUID low part) | — |
| `vector<WorldPackets::BattlePet::BattlePetSlot> _slots` | `[BattlePetSlot; 3]` | Fixed size 3 |

---

## 13. Audit (2026-05-01)

**Status confirmed: ✅ n/a for WoLK 3.4.3.**

BattlePets is a **Mists of Pandaria (5.0.4, Oct 2012)** feature — the Pokémon-style companion mini-game. The 3.4.3.54261 (WotLK Classic) client has no Pet Battle UI, no journal screen, no `BattlePetSpecies.db2` / `BattlePetBreedQuality.db2` / `BattlePetSpeciesState.db2` data files, no `HighGuid::BattlePet` namespace, and no spell `SPELL_SUMMON_BATTLE_PET` (118301) or `SPELL_BATTLE_PET_TRAINING` (125610) cast paths. Companion pets in 3.4.3 are pure cosmetic critters resolved via `summon_companion` spell effect → `Player::SetCritterGUID`, with no levelling, abilities, or battles — entirely separate code. Verified that no `BattlePetMgr` / `BattlePet` struct / DB2 stores / login-DB schema exists in `crates/wow-*` and that 8 of 9 `CMSG_BATTLE_PET_*` opcodes silently default in the dispatcher (correct).

**Residual cleanup (low priority):** the existing `BattlePetJournalLockAcquired` zero-byte stub at `crates/wow-packet/src/packets/misc.rs:1009` is sent **proactively and unconditionally** during `send_initial_packets_after_load` (`crates/wow-world/src/handlers/character.rs:4255`), which is semantically wrong vs the C++ flow (only sent in response to `CMSG_BATTLE_PET_REQUEST_JOURNAL_LOCK`, and only if `IsJournalLockAcquired()`). For 3.4.3 the cleanest fix is to **remove the proactive send entirely** (preferred — the client never asks); alternatively switch to `BattlePetJournalLockDenied` (0x25ee) which would require defining a new packet type. Tracked as #BPETS.4 / #BPETS.6. Empty handler at `handlers/misc.rs:606` is fine as-is.

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.
