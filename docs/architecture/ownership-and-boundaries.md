# RustyCore ownership and dependency boundaries

This document is the executable architecture baseline for the incremental refactor tracked by
issue #133. It records the current owners and intentional mirrors before code is moved. The
machine-readable dependency rules live in
`tools/architecture/dependency-policy.json`; `tools/architecture/check_architecture.py check`
enforces them. The checked-in issue ledger
`tools/architecture/architecture-issue-ledger.json` records every architecture epic and
implementation slice, including follow-ups omitted by the previous snapshot. It is organized
around four lanes: physical modules, encapsulation/ownership, runtime/persistence authority, and
stable crate/extension seams. External prerequisites such as QA issue #177 are explicit nodes.
The checker keeps this document, the ledger, and the JSON policy in agreement without contacting
GitHub; the displayed topological order never serializes independent work.

## Decision

RustyCore remains a **modular monolith** while the port is incomplete:

1. preserve one mutable owner for each gameplay concept;
2. split large files into private, cohesive modules inside their current crate first;
3. move a responsibility only after its behavior, ordering, persistence, packet metadata, and
   capture contract are frozen;
4. promote a module to a crate only when its dependency direction and public API are already
   stable;
5. never create a second state mirror, a new `Arc<Mutex<_>>`, or a wider public API merely to make
   a physical extraction compile.

The C++ server is the source of truth for behavior, state ownership, and update ordering. Its
physical layout is not a target: TrinityCore compiles the large `server/game` tree as one library,
and files such as `Player.cpp`, `Unit.cpp`, and `Spell.cpp` remain large themselves. Rust modules
therefore may be smaller and more cohesive while preserving the same semantic owners.

### Session-kernel boundary after #378

Issue #378 audited the five module families left beside `wow-session` after #297. None is a
transport-kernel module in its current shape, so none moves by weakening the P4 dependency rule.
This is a terminal classification of the present modules, not permission to leave their debts
ownerless:

| Current module | Why it stays in `wow-world` | Required inversion and owner |
|---|---|---|
| `dispatch` + `registry` | The registered thunk is `fn(&mut WorldSession, WorldPacket)`; moving it would make `wow-session` depend back on its consumer, while a downstream-instantiated generic inventory violates Rust's orphan rule. | #153 owns a terminal typed `PacketRouter`/adapter inversion. Until then the sole registry and dispatch call remain together in `wow-world`. |
| `admission` | Packet-rate accounting is session work, but the spoof-ban completion writes through `LoginDatabase`; importing that adapter would violate P4. | #169 owns a SQLx-free outbound ban capability. Split the pure decision only when the caller can execute the returned ban outcome without defaulting a missing authority. |
| `lifecycle` | Character claims, canonical map/accessor teardown, loot fences and durable Player persistence are application/gameplay lifecycle, matching C++ `WorldSession::LogoutPlayer`, not transport state. | #252 owns canonical Player/Map authority cuts and #169 owns the remaining persistence ports. Keep the ordered session adapter here; extract only a future pure state machine whose outcomes name every owed application step. |
| `driver` | The pass interleaves packet ingestion with Player timers, aura/spell/gameobject work, creature compatibility ticks, saves, logout and dispatch. C++ `WorldSession::Update` is likewise session/application orchestration. | #371 converged every legacy creature-local deadline onto the owning tick's propagated `diff_ms`; #252 removed Player-state directory mirrors; #153 performs the terminal shell audit. The transport kernel may return connection outcomes, but must not schedule gameplay. |
| `mailbox` | `SessionCommand` is a typed gameplay protocol and the durable rail carries creature/combat results; moving it would import domain payloads into P4. | Capability owners retire commands vertically as canonical Player/Map ownership lands. #371 removed the independent creature wall-clock source but did not change the gameplay payloads; #153 decides whether a payload-agnostic bounded queue port is earned after that burn-down. |
| `directory` | Presence/addressability, generation fences, delivery handles and the bounded loot-roll identity list are session concerns; gameplay reads now resolve from canonical Player/Map owners. | #252 retired the gameplay mirror. #153 now classifies the remaining narrow presence/address directory over opaque handles; `None` remains unknown and is never converted to a fabricated default during far teleport. |

The resulting boundary is deliberate: `wow-session` owns only the logical connection and
cross-socket ordering already extracted by #297. The five current module families remain private
application adapters until their named authority or port issues remove the blocker. A future
extraction must use an outcome/command/query boundary, preserve C++ order, and remove the old
access in the same slice; copying state or adding a P4 dependency is not an extraction.

This direction also follows Rust's native boundaries: modules control visibility inside a crate,
while Cargo packages are separate compilation and public-API boundaries. The checker publishes
two independent measurements. The **physical view** counts every real `.rs` file separately, with
production and tests split; ordinary `mod` extraction therefore appears immediately. Around 2,000
productive lines triggers review and above 4,000 requires a split plan or explicit cohesive
exception, but neither is a daily blocking threshold. The **logical view** counts each curated
owner root together with its reviewed private descendants, so distributing a God object cannot be
claimed as ownership reduction. Logical production/test/total counts in
`runtime-ownership-ledger.json` are independent non-growth ceilings. A logical owner row cannot
disappear until it is explicitly retired or replaced.

## Dependency direction

Every workspace package has one dominant category:

| Category | Intended dependency direction |
|---|---|
| `foundation` | foundation only |
| `domain-runtime` | foundation and domain/runtime |
| `application` | foundation, domain/runtime, and application |
| `adapter-platform` | inward layers and other adapters |
| `composition` | all production layers; owns process wiring |
| `tooling` | may consume production layers but production must not consume tooling |

Four packages have a stricter exact direct-dependency allowlist: `wow-network`, `wow-packet`,
`wow-data` and `wow-session`. For the first three this prevents mixed boundary surfaces from
growing while responsibilities are extracted. `wow-session` is there for the opposite reason
(#297): it was extracted precisely so a transport decision cannot reach gameplay, a map or a
query, and without the exact allowlist that isolation would hold only by accident — its
`adapter-platform` category alone would permit a later edge to `wow-map` or `wow-database`. Every listed edge must remain present: once an extraction
removes it, the checker rejects the obsolete allowance so the dependency cannot later be
reintroduced without review.

The inward `foundation`, `domain-runtime`, and `application` categories also have an exact
per-package allowlist for direct third-party `normal` and `build` dependencies. The same external
ratchet protects the deliberately narrow `wow-network`, `wow-packet`, and `wow-data` adapters, so,
for example, raw network code cannot bypass its workspace allowlist by importing `sqlx` directly.
This is deliberately not a global taxonomy of crates.io: Cargo metadata cannot say whether an
arbitrary package is SQL, networking, configuration, process, or runtime infrastructure.
Instead, every new direct external dependency on a protected surface requires a reviewed policy
change. Existing utility libraries such as `rand` and adapter-owned infrastructure such as
`wow-network → tokio` remain explicitly allowed, while current inward `sqlx`/`tokio` leaks are
issue-linked exceptions. Other adapter and composition packages may integrate concrete
infrastructure; the workspace-edge policy still constrains which RustyCore layers they consume.
Development-only dependencies are outside this production boundary. The checker uses Cargo's
resolved package IDs with all features enabled and no platform filter rather than package-name
equality. It validates package/source/version identity for every resolved external edge and
globally rejects duplicate JSON keys, duplicate Cargo package/node/member IDs, and multiple direct
package IDs collapsing to the same package/source/kind identity. On a protected surface, the
allowlist additionally requires the exact canonical crates.io registry source recorded in the
policy; a same-named path, Git, or alternate-registry package is a different edge and fails. Thus
renames, source substitution, malformed or ambiguous metadata, and inactive target-specific
dependencies cannot bypass the boundary. Workspace members themselves must be source-null
`path+…` packages with coherent name/version identity, so registry or Git packages cannot be
mislabelled as internal members. The self-test pins the exact
`cargo metadata --locked --all-features --format-version 1` command, and the checker also fails
closed if Cargo's `workspace.default-members` stops covering every member, because
`--all-features` would no longer prove optional dependencies of omitted packages.

The policy's workspace and external `exceptions` are a **ratchet, not an endorsement**. Each
exception:

- describes an edge present in the current Cargo graph;
- names the issue responsible for deciding or removing it;
- permits no neighboring edge by implication;
- becomes an error as soon as the underlying dependency disappears, so obsolete debt cannot stay
  silently allowlisted.

When several ordered slices retire distinct uses of the same Cargo edge, `tracking_issue` names
the final slice that can remove the dependency and the reason lists every intermediate slice.
Closing an earlier slice must not leave an exception pointing at an already completed issue.
The checker enforces that rule through the issue ledger: an exception whose `tracking_issue` is
absent from the ledger, or owned by a completed issue, fails the architecture check. Generic
exceptions keep `tracking_issue` 133 only while the parent genuinely remains open, and the
ledger's `reaudit_issue` (#153) explicitly owns their re-audit and final classification.

A new package, new upward edge, undeclared restricted-package edge, undeclared direct external
dependency in an inward package, duplicate classification, stale allowed dependency, or obsolete
exception fails the architecture check. A deliberate baseline change must update this document
and the JSON policy in the same reviewed commit.

## Current ownership and mirror ledger

“Writer” means the authority allowed to mutate the state, not every call site that requests a
mutation. “Clock/task” records who advances time-dependent behavior. A mirror row must have an
explicit synchronization direction and retirement condition; there is no implicit
last-writer-wins policy.

| Concept | Current owner and storage | Writers | Readers / delivery | Clock, lifetime, and synchronization | Retirement |
|---|---|---|---|---|---|
| Authenticated world connection | `wow-network::accept` and the connection task | socket/authentication task | `WorldSession` dispatch boundary | one connection; created after authentication and dropped on disconnect | Remains a network responsibility. #134 narrowed the listener to transport-owned configuration and authenticated connection outputs. |
| Reserved-but-empty crates | `wow-achievement`, `wow-combat`, `wow-pvp` and `wow-spell` (`domain-runtime`) contain no code | nothing writes them yet | nothing reads them, and nothing they read either: #298 removed the phantom Cargo edges the four still declared after #288 removed `wow-ecs`'s | none - they hold no state and run on no clock | The reservation is now checked, not described: `dependency-policy.json#reserved_packages` names the owning issue for each (`wow-combat` #29, `wow-spell` #30, `wow-pvp` and `wow-achievement` #48), `check_architecture.py` fails when one acquires a dependent or a dependency and when its owning issue closes with the crate still empty, and `refresh-issue-state` keeps that owner state derived rather than hand-maintained. `wow-ecs` was resolved by removal: the terminal architecture makes `wow-entities::Player` the sole mutable owner, no entity-storage slice was ever planned, and the name can be re-created the day one is. |
| Database updater adapter | private `wow_database::updater::DbUpdater` owns its cloned SQLx pool and raw mysql-CLI connection parameters | `world-server` selects typed Login/Character/World/Hotfix adapters and `bnet-server` selects Login; both supply configuration to `populate_typed_database_like_cpp` / `update_typed_database_like_cpp`, which alone construct the updater and extract its pool | the ordered database-update bootstraps observe the existing populate/update results but cannot name the updater or its pool | process startup only; World preserves Login then Character then World then Hotfix order and enabled-mask gates, while BNet preserves populate failure as warn-and-continue before update | #412 makes both updater constructors private and removes updater-specific pool/updater ownership from both composition roots. Other bootstrap database capabilities retain their separately audited typed-adapter access. #153 still audits the broad bootstrap workflows, but #169 no longer owns the World concrete-pool leak. |
| Player lifecycle persistence capability | `wow_persistence`: offline marks, typed core-character/account-collection/admission/auxiliary-login/initial-world-state/transport-login load requests and outcomes (including spell cooldown/charge, raw trait config/entry rows, pet stable/active-pet rows, group id, equipment/transmog sets, CUF profiles, currencies, Player progression, character aura, inventory rows, and uncaged-item recovery state), ordered login-item repair actions, semantic homebind, logout-buyback, talent-reset, represented XP/rest, direct money/durability, bank-slot purchase, standalone durability/currency and realm-character-count writes, account-collection saves, the semantic `PlayerCharacterSaveRequestLikeCpp` snapshot and the three-way `PersistenceOutcomeLikeCpp`, behind `PlayerLifecyclePortLikeCpp`. The crate has **no dependencies at all** — no pool, row, transaction, statement or SQL string | `wow_database::player_lifecycle_adapter` is the only implementation and the only place that maps the semantic requests/snapshot to MariaDB statements/parameters, owns the pools and classifies driver errors | private `wow_world::session::lifecycle::persistence` holds the port; Session callers retain gameplay validation and never name a statement; mixed vendor/quest transactions temporarily pass the same typed currency rows to a narrow MariaDB rendering bridge without changing their wider atomic boundary | composed in `world-server` before any session is accepted, so a build that cannot persist lifecycle state fails at startup rather than dropping writes | #200 earned the crate with offline marks; #287 moved the five account-collection writes; #286 moved the represented Character save as one ordered transaction; #384 retired its unreachable Session statement builders; #386 moves the five collection reads and preserves the two independent item-appearance failure branches; #390 moves customization, completed-achievement and instance-time reads while preserving row/default/clear rules; #394 moves delete/insert/update homebind writes while retaining the live FIFO; #396 moves the independent spell-cooldown and spell-charge reads while `wow-world` retains DB2 filtering, expiry and aggregation; #398 moves trait entries/configs in their existing order while preserving missing columns as unknown and retaining DB2/gameplay validation in `wow-world`; #400 moves logout buyback cleanup while retaining each inventory/item delete pair, the single transaction and publication only after `Applied`; #402 moves the Characters-count then Login-replace realm refresh while preserving independent connections and failure short-circuit; #404 moves World templates followed by Characters overrides while retaining their independent failure branches and leaving DB2 validation/publication in `wow-world`; #406 moves both transport-login reads while leaving route, phase, clock and publication decisions in `wow-world`, and preserves an absent/unresolved owner as `None`; #408 moves the absolute-money/reset-metadata/delete-all/retained-talent transaction and exact unknown-COMMIT money reconciliation while Session retains its exclusive money fence and post-commit gameplay publication; #410 moves Rust's represented immediate XP/rest transaction without claiming that timing as C++ parity; #414 moves the core `CHAR_SEL_CHARACTER` read/row decode while leaving every Player validation/default and every later login query in `wow-world`; #416 moves the battleground-location, homebind-location and guild-membership reads while preserving their distinct optional, fatal and unknown-authority branches in `wow-world`; #418 moves the pet-stable plus six active-pet reads while retaining active-pet gating, gameplay defaults, publication order and the separate talent-reset writes in `wow-world`; #420 moves the group db-store-id lookup while retaining registry resolution and sequence reset in `wow-world`; #422 moves equipment sets, transmog outfits, CUF profiles and currencies while leaving canonical validation/publication in `wow-world`; #424 moves spell/favorite, skill, talent, glyph, action-button and reputation rows while keeping every catalog validation, side effect, completion marker and publication in `wow-world`; #426 moves the two character-aura row families while retaining correlation, authority gating, gameplay application and item-mod ordering in `wow-world`; #428 moves top-level inventory, bag-content and void-storage rows while retaining item interpretation, repair decisions and installation ordering in `wow-world`; #430 moves the two existing login item-repair transactions while retaining those gameplay decisions and transaction order in `wow-world`; #444 moves direct money writes and player-funded durability transactions while preserving money-first order and Session-owned reconciliation/publication; #446 moves standalone quest-reward currency saves and the shared statement rendering while preserving mixed transaction atomicity; #456 moves the remaining standalone no-cost durability write while preserving persistence-before-publication and its missing-port fixture fallback; #464 moves the atomic money/bank-slot purchase write while retaining Session's exclusive-money fence, unknown-COMMIT reconciliation and post-commit publication; #466 moves the uncaged-item owner/link recovery read while preserving the foreign-owner refusal and idempotent absent-item postcondition. Executable SQL order remains frozen in the adapter contract from #187/#286/#386/#390/#394/#396/#398/#400/#402/#404/#406/#408/#410/#414/#416/#418/#420/#422/#424/#426/#428/#430/#444/#446/#456/#464/#466. |
| Session account-state persistence capability | `wow_persistence::SessionAccountStatePortLikeCpp` owns semantic global/character account-data scope, typed account-data rows, tutorial values and classified outcomes without a database dependency | `wow_database::session_account_state_adapter` alone maps those requests to `SEL/REP_ACCOUNT_DATA`, `SEL/REP_PLAYER_ACCOUNT_DATA` and `SEL_TUTORIALS` | `WorldSession` retains the C++ mask/type validation, cache reset, tutorial coherence flags and publication-after-success rule | composed once in `world-server` and installed into every authenticated session; the missing-port write fallback deliberately preserves the pre-cut in-memory behavior | #388 moves the three account-data/tutorial workflows out of concrete persistence while leaving session-owned authority in place. #169 owns the remaining persistence cuts and #153 the terminal Session audit. |
| PacketSpoof admission persistence capability | `wow_persistence::PacketSpoofBanPersistencePortLikeCpp` owns the semantic account/IP targets, IP-account lookup outcome, write request and classified result without a database dependency | `wow_database::packet_spoof_ban_adapter` alone maps the capability to Login statements, row decoding and the account-ban transaction | Session admission retains packet counters, ban policy/target selection, query-failure warning, exact pending-plan retry and generation-aware kick fanout | composed once in `world-server`; IP lookup finishes before the independent IP insert, while account clear-active plus insert share one Login transaction | #434 removes `LoginDatabase`, `LoginStatements` and `SqlTransaction` from `session::admission` while preserving the current C++-anchored order and failure branches. |
| Void-storage persistence capability | `wow_persistence::VoidStoragePersistencePortLikeCpp` owns semantic unlock, swap and complete mixed-transfer requests plus the three-way money-transaction result without a database dependency | `wow_database::void_storage_adapter` alone maps those requests to Character statements, transactions and the post-unknown-COMMIT money read | Session retains NPC/player/slot and gameplay validation, the payout-admission and money-mutation fence, reconciliation/quarantine, runtime publication and packets | composed once in `world-server`; unlock preserves money -> flags -> delete-all order, swap preserves replace-destination -> replace/delete-source order, and transfer preserves money -> deposit destruction/void rows -> withdrawal item/inventory/void rows -> quest rows in one transaction | #436 moves unlock and swap; #438 moves the complete mixed transfer transaction as one semantic plan rather than exposing isolated statement helpers; #442 removes the superseded Session full-save statement builders once no production caller remains. |
| Player social persistence capability | `wow_persistence::SocialPersistencePortLikeCpp` owns typed contact rows, add-candidate state, relationship kinds, classified party-invite membership reads and mutation outcomes without a database dependency | `wow_database::social_adapter` alone owns the Characters queries, SQLx row decoding, MariaDB upserts, flag clearing and cleanup order | Session retains name normalization, self/faction/list-limit and party-invite admission, online-status projection, packet construction and logging | composed once in `world-server`; candidate lookup precedes the two tolerant state/count reads, removal clears the selected flag before deleting an empty row, and party invite preserves ignore before the conditional low-level friend lookup | #440 removes concrete CharacterDB/SQLx/raw-SQL access from every represented social handler without claiming the still-missing C++ account-level ignore authority; #448 moves the two remaining party-invite social reads out of `handlers::group` while preserving its represented character/account ignore query and C++ short-circuit order. |
| Quest POI persistence capability | `wow_persistence::QuestPoiPersistencePortLikeCpp` owns typed point/blob rows and a stage-classified load outcome without a database or packet dependency | `wow_database::quest_poi_adapter` alone owns the two World queries and SQLx row decoding | Session retains the represented lazy cache, point/blob association, missing-point skip, packet projection and empty-on-failure behavior | composed once in `world-server`; the current points-then-blobs read order remains unchanged in this structural cut | #450 removes the quest-POI SQLx/WorldDatabase leak from `handlers::quest`; C++-faithful process-wide startup loading remains an explicit later convergence rather than a hidden timing change. |
| Stored Item loot-money persistence capability | `wow_persistence::StoredItemMoneyPersistencePortLikeCpp` owns the semantic player/item request, all-or-nothing capped-money outcome, definite-rollback/unknown-COMMIT result and joint-fact reconciliation classification without a database dependency | `wow_database::stored_item_money_adapter` alone owns the Character SQLx transaction, row locks, affected-row checks, trace identities and driver/deadlock classification | the loot application retains the per-character mutation mutex, retry policy, durable completion guards, runtime balance application, kick decision and packet publication | composed once in `world-server`; mutation locks character before container, updates money and consumes exactly one source row atomically, while reconciliation locks the same facts in the same order and rolls back its read-only transaction | #452 removes the stored-item raw SQL/SQLx transaction from `handlers::loot` without changing C++ loot rules or the existing crash-safe extension; #454 handles the distinct multi-recipient group transaction rather than broadening this port. |
| Group loot-money persistence capability | `wow_persistence::GroupLootMoneyPersistencePortLikeCpp` owns the SQLx-free payout request, per-recipient durable outcomes and definite-rollback/unknown-COMMIT/reconciliation vocabulary | `wow_database::group_loot_money_adapter` alone owns the Character SQLx transaction, stable GUID row-lock order, capped updates, affected-row checks, trace identities, driver/deadlock classification and reconciliation reads | the loot application retains recipient admission and deduplication, every per-character mutation mutex, deadlock retry policy, durable guards, authority commit/quarantine, runtime balance application, kicks and packet/viewer publication | composed once in `world-server`; all admitted rows lock and update in one transaction with one commit, while unknown COMMIT compares every changed recipient and treats all-before as rollback, all-after or cap-only no-op as committed, and mixed/missing/error as indeterminate | #454 removes the group payout SQLx transaction and reconciliation reads from `WorldSession` without changing C++ loot rules or RustyCore's existing crash-safe extension. Remaining concrete workflows stay explicit #169 cuts. |
| Represented Group persistence capability | `wow_persistence::RepresentedGroupPersistencePortLikeCpp` owns primitive ordered Group commands, the existing sequential/atomic execution distinction and applied-prefix/rollback/unknown-COMMIT outcomes without a database dependency | `wow_database::represented_group_persistence_adapter` alone maps every command to the exact Character statement and bind order, executes general transitions sequentially and retains the represented difficulty transaction | `wow-social::GroupRegistry` remains the aggregate owner and emits database-neutral intents; `wow-world::handlers::group` only maps those intents to the persistence vocabulary after releasing registry guards and logs the typed result | composed once in `world-server`; general Group commands stop at the first failed pooled execute, while the existing difficulty path keeps its one-command atomic batch and reset/mutation/fanout order | #468 removes Group `CharStatements`, `PreparedStatement`, `CharacterDatabase`, driver errors and the difficulty `SqlTransaction` from `wow-world` without moving the aggregate, awaiting under its guard or changing connection choice. |
| Represented Group startup-load capability | `wow_persistence::RepresentedGroupStartupLoadPortLikeCpp` owns typed character-cache, group and member rows plus an exact seven-stage failure vocabulary | `wow_database::represented_group_persistence_adapter` alone owns the four Character cleanup statements, three queries and tolerant SQL-row decoding | `wow-social::GroupRegistry` remains the aggregate owner; `world-server::runtime::map` maps the typed rows and invokes its existing materializer only after all database awaits finish | composed once before group startup; cleanup order is members-without-character, groups-without-leader, groups-with-fewer-than-two-members, members-without-group, followed by character cache, groups and members | #470 removes represented Group startup queries/results from `world-server` without adding a per-session resource, changing empty/default decoding, or materializing partial state after a database failure. |
| Represented Player spell-acquisition persistence capability | `wow_persistence::PlayerSpellAcquisitionPersistencePortLikeCpp` owns the complete SQLx-free source/result authority, deterministic replacement operations, absolute trainer fee, opaque 16-byte attempt token and classified commit/reconciliation outcomes | `wow_database::player_spell_acquisition_adapter` alone owns the Character transaction, money and authority row locks, SQL text, affected-row checks, rollback, deadlock classification and lost-COMMIT proof | `wow-world::spell_acquisition` remains the application planner and validates the prepared Player snapshot before constructing the typed request; Session retains the money exclusion/cancellation fence and installs/publishes runtime state only after the port proves durability | one Character transaction preserves money lock/compare, complete source authority lock/compare, spell/favorite/skill replacement order, guarded money update, token upsert and COMMIT; reconciliation requires exact money, complete result authority and token | #472 removes SQLx and the concrete Character database from the trainer acquisition application path without changing gameplay, packet or publication order. |
| Battle-pet account persistence capability | `wow_persistence::BattlePetAccountPersistencePortLikeCpp` owns SQLx-free durable pet/slot/receipt rows, the opaque add-request key, process lease and classified mutation outcomes | `wow_database::battle_pet_account_adapter` alone owns the Login database, named-lock broker, GUID sequence transaction, SQL/statement selection, row decoding, fencing and duplicate-key classification | `wow-world::battle_pet_account` retains the account-scoped mutable owner, lease attachment rules, capacity/gameplay validation, durable-to-packet projection and post-durability publication | one adapter instance is composed in `world-server`; named-lock scope/fence, request replay, capacity check, GUID reservation, pet/receipt mutation and slot replacement preserve the #160/#161 ordering and failure rules | #474 removes the complete battle-pet SQLx/LoginDatabase implementation from `wow-world` without moving account/gameplay ownership or changing packet order. |
| Battle-pet purchase-saga persistence capability | `wow_persistence::BattlePetPurchasePersistencePortLikeCpp` owns the SQLx-free command/status/outcome/error vocabulary and the narrow arm/disarm COMMIT-cancellation fence | `wow_database::battle_pet_purchase_adapter` alone owns Character DB row decoding, guarded money and command statements, T1/T3/T4/T5/T6 transactions, affected-row checks and lost-COMMIT reconciliation | `wow-world::battle_pet_purchase` retains offer admission, request-key generation, retry policy, the exclusive Player-money guard, canonical battle-pet account application, compensation decisions and ordered packet/criteria publication | one adapter is composed in `world-server`; the gameplay money owner supplies the cap and a cancellation fence for each charge/refund attempt, so the adapter acquires neither `wow-entities` nor session ownership | #476 removes the concrete Character DB store and Session-side adapter construction from the #161 saga while preserving exact-once charge/refund, receipt replay, quarantine and publication-marker behavior. |
| Support bug-report persistence capability | `wow_persistence::SupportBugReportPersistencePortLikeCpp` owns the SQLx-free text/diagnostic request and classified result | `wow_database::support_bug_report_adapter` alone maps the request to `CHAR_INS_BUG_REPORT` and owns the Character database handle | the support handler retains the feature gate, packet decode and wire-silent failure behavior; it cannot name a statement or database | composed once in `world-server`; one non-transactional Character statement binds text before diagnostic information exactly like C++ | #458 removes the legacy bug-report insert from `WorldSession` through a dedicated support capability instead of broadening the Player lifecycle port. The parsed report-type bit remains intentionally unpersisted like C++. |
| Next-mail-time persistence capability | `wow_persistence::NextMailTimePersistencePortLikeCpp` owns the SQLx-free player-guid request, five-column represented mail row and loaded/failed outcome | `wow_database::next_mail_time_adapter` alone owns `CHAR_SEL_MAIL`, its u64 bind, Character database handle and tolerant row decoding | the player handler retains identity, clock comparison, read/delivery filters, sender dedupe, three-entry cap, packet construction, logging and Realm routing | composed once in `world-server`; this preserves Rust's existing on-demand query without claiming C++ ownership parity | #460 removes the concrete mail read from `WorldSession`. C++ reads `_player->GetMails()` and `unReadMails`; #153/the mail vertical must retire this transitional query when Rust has that canonical in-memory owner. |
| Gameobject-use template persistence capability | `wow_persistence::GameObjectUseTemplatePersistencePortLikeCpp` owns the SQLx-free entry request and typed type/icon/size/Data0..34/content-tuning projection | `wow_database::gameobject_use_template_adapter` alone owns `SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY`, its u32 bind, World database handle and tolerant row decoding | the gameobject handler retains object/visibility admission, template interpretation, conditions, distance/mover/cooldown rules and type-specific gameplay dispatch | composed once in `world-server`; missing port, row or query result remains an explicit no-op without fabricating a template | #462 removes the concrete per-use World query from `WorldSession`. C++ loads `ObjectMgr::_gameObjectTemplateStore` at startup and `GameObject::Use` reads `GetGOInfo()`; #153/the gameobject vertical must replace this transitional read with that canonical store. |
| Canonical map-corpse persistence capability | `wow_persistence::MapCorpsePersistencePortLikeCpp` owns the SQLx-free `(map, instance)` request, raw persisted corpse/phase/customization rows and independent base/auxiliary outcomes | `wow_database::map_corpse_adapter` alone maps that request to `SEL_CORPSES`, `SEL_CORPSE_PHASES` and `SEL_CORPSE_CUSTOMIZATIONS`, preserving exact bind width, order and query failure classification | the transitional `wow-world` application adapter retains corpse validation, item-cache parsing, faction resolution, map-local GUID allocation and publication into canonical `wow_map::Map` | composed once in `world-server`; the map lock is checked before I/O and reacquired only after the complete typed result returns | #392 removes concrete Character-database access from `Map::LoadCorpseData` hydration without folding map state into the Player lifecycle port. #153 owns relocating the transitional Session caller; the canonical corpse owner and clock do not change in this cut. |
| Respawn persistence capability | `wow_persistence::RespawnPersistencePortLikeCpp` owns SQLx-free respawn rows, stable `(spawn type, spawn id, instance)` keys, save/delete mutations and classified load/mutation outcomes | `wow_database::respawn_persistence_adapter` alone owns `SEL_RESPAWNS`, `SEL_ALL_RESPAWNS`, `REP_RESPAWN`, `DEL_RESPAWN`, row decoding and bind order | legacy/canonical map runtimes retain eligibility, latest-per-key coalescing, retry cadence and lifecycle ordering; they emit typed mutations without inspecting SQL | composed once in `world-server`; startup loading and the shared writer use the same port, I/O stays outside map locks, and shutdown still drains pending mutations | #493 removes respawn SQL identity, statement parameters and concrete Character-database execution from map/session/runtime orchestration without changing gameplay authority, clock ownership, save-before-delete order or retry behavior. The exhaustive inventory falls to 21,346 exact rows (10,974 production and 10,372 fixtures), with multiplicity 23,674 (12,557 production and 11,117 fixtures); 29 obsolete workflow groups become 10 stable adapter groups. |
| Represented game-event persistence capability | `wow_persistence::GameEventPersistencePortLikeCpp` owns SQLx-free condition-save rows, semantic condition/state/delete/seasonal-reset mutations and classified load/mutation outcomes | `wow_database::game_event_persistence_adapter` alone owns the represented Character statements, row decoding, bind order and the two delete-then-insert transactions | `GameEventDataStoreLikeCpp`, the scheduler and runtime bridges retain event validation, activation/deactivation policy, quest-reset fanout and publication ordering; they emit typed requests without inspecting SQL | composed once in `world-server`; condition saves load before canonical event materialization, runtime I/O remains outside map locks, and seasonal DB deletion still precedes live-player resets | #495 removes represented game-event SQL identity and concrete Character-database execution from loader/runtime orchestration without changing event, map, scheduler, clock or fanout authority. The exhaustive inventory falls to 21,252 exact rows (10,876 production and 10,376 fixtures), with multiplicity 23,591; 17 obsolete orchestration workflow annotations become 9 stable adapter workflows. C++ also loads `game_event_save`, but Rust did not do so before this structural cut; adding that missing behavior remains explicit porting work rather than being hidden in the refactor. |
| Instance-lock persistence capability | `wow_persistence::InstanceLockPersistencePortLikeCpp` owns SQLx-free startup rows, semantic mutation plans and classified load/commit outcomes | `wow_database::instance_lock_persistence_adapter` alone owns the Character statements, tolerant row decoding, exact binds and transaction rendering | `wow_instances::InstanceLockMgr` remains the sole lock-rule and in-memory state owner; it mutates first and emits ordered plans without seeing a pool, row or transaction | composed once in `world-server`; startup loads shared rows before ordered character locks, Session releases the manager lock before awaiting, and success packets remain after a successful commit | #497 removes the entire `wow-instances -> wow-database` edge plus direct transaction construction from the three represented Session instance paths. Delete-before-insert player/shared updates, extension and force-expire order remain exact; a failed commit preserves the already-applied in-memory mutation and suppresses publication exactly as before the cut. The exhaustive inventory falls to 21,093 exact rows (10,797 production and 10,296 fixtures); 21 obsolete domain/handler workflows and the package fixture group disappear, replaced by 8 stable adapter workflows, and `wow-instances` falls from 256 concrete accesses to zero. |
| PlayerChoice startup catalog capability | `wow_persistence::PlayerChoiceCatalogPersistencePortLikeCpp` owns the SQLx-free core/locale row bundles and loaded/failed outcomes | `wow_database::player_choice_catalog_adapter` alone owns the ten World statements, exact eight-core/two-locale query order, null handling and concrete row decoding | `wow_data::PlayerChoiceStoreLikeCpp` retains validation, attachment, diagnostics and immutable catalog ownership; `world-server::player_choice_catalog` performs only the immutable boundary-DTO conversion required by the restricted `wow-data` dependency surface | composed once in `world-server`; core loading and all existing diagnostics complete before locale loading, matching the represented C++ startup order | #499 removes all 188 concrete accesses from `wow-data::player_choice` without adding a dependency, moving gameplay rules or creating a generic query API. The exhaustive inventory falls to 20,966 exact rows (10,668 production and 10,298 fixtures); three unstable workflows disappear and eight stable adapter workflows replace them. |
| Mount startup catalog capability | `wow_persistence::MountCatalogPersistencePortLikeCpp` owns SQLx-free rows and loaded/failed outcomes for the four effective DB2 overlays plus `mount_definitions` | `wow_database::mount_catalog_adapter` alone owns the four Hotfix statements, the World statement, concrete database handles and tolerant row decoding | `wow_data::mount` retains DB2 parsing, by-ID replacement, derived spell/type/display indices, faction-definition validation and all mount selection rules; private `world-server::mount_catalog` performs immutable boundary conversion | composed once in `world-server`; the existing startup sequence remains Mount overlay, definitions, capability overlay, type-x-capability overlay and display overlay, and any query failure remains fatal | #501 removes all 132 concrete accesses from `wow-data::mount` without adding a forbidden dependency or fabricating missing rows. Hotfixes replace records by ID and rebuild only the affected immutable indices; C++ validation still skips unknown mount-definition spells. The exhaustive inventory falls to 20,919 exact rows (10,621 production and 10,298 fixtures); ten unstable workflows disappear and eleven stable adapter workflows replace them, leaving 889 groups (438 stable and 451 unstable). |
| Reputation startup catalog capability | `wow_persistence::ReputationCatalogPersistencePortLikeCpp` owns SQLx-free rows and loaded/failed outcomes for reward rates, creature-on-kill awards and spillover templates | `wow_database::reputation_catalog_adapter` alone owns the three World statements, concrete database handle and the existing tolerant signed/unsigned/bool coercions | `wow_data::reputation` retains immutable store ownership, faction/creature/rate/rank validation, duplicate replacement and skip reports; private `world-server::reputation_catalog` converts typed rows and preserves the existing warnings/load summaries | composed once in `world-server`; startup remains reward-rate then creature-onkill then spillover, every query fully decodes before the next, and a query failure remains fatal before the affected store is published | #503 removes all 104 exact production entries (multiplicity 161) from `wow-data::reputation` without changing gameplay or silently correcting represented coercions. The exhaustive inventory falls to 20,873 exact rows (10,574 production and 10,299 fixtures); direct production access falls 5,108→5,004, while nine unstable workflows become fourteen stable adapter workflows, leaving 890 groups (452 stable and 438 unstable). |
| AreaTrigger template startup capability | `wow_persistence::AreaTriggerTemplateCatalogPersistencePortLikeCpp` owns one SQLx-free six-row-family load bundle and loaded/failed outcome | `wow_database::area_trigger_template_catalog_adapter` alone owns the six World statements, concrete database handle, null checks and row decoding | `wow_data::area_trigger_template` retains immutable template/create-properties ownership, validation/correction reports, action/shape/spline/orbit attachment, curve/world-safe-location decisions and script interning; private `world-server::area_trigger_template_catalog` converts boundary DTOs | composed once in `world-server`; every row family fully decodes before the next and any query failure remains fatal before immutable publication. The represented Rust query/failure order remains actions, polygon vertices, spline points, create properties, orbit, templates; C++ queries templates before create properties/orbit, and correcting that pre-existing difference is explicitly outside #505 | #505 removes all 117 concrete production entries from `wow-data::area_trigger_template` without mixing a parity correction into the boundary change. The exhaustive inventory falls to 20,796 exact rows (10,496 production and 10,300 fixtures); direct production access falls 5,004→4,887, while two unstable workflows disappear and seven stable adapter workflows replace them, leaving 895 workflows (459 stable and 436 unstable). |
| Remaining AreaTrigger World-catalog capability | `wow_persistence::AreaTriggerWorldCatalogPersistencePortLikeCpp` owns SQLx-free destination, script, teleport-relation, quest-relation and tavern rows behind five independent loaded/failed operations | `wow_database::area_trigger_world_catalog_adapter` alone owns the five World statement identities, concrete handle and checked row decoding | `wow_data::area_trigger` retains the represented fallback geometry, collision store, script interning, DB2/WorldSafeLoc/quest validation, duplicate behavior and reports; private `world-server::area_trigger_world_catalog` converts boundary DTOs | one adapter is composed once; production preserves destination -> script -> tavern and does not invoke the represented-but-dormant teleport-relation or quest-relation operations. Each selected read fully decodes before domain mutation and failure remains fatal at its existing startup boundary | #536 removes all 62 direct production identities from `wow_data::area_trigger` without activating dormant loaders or correcting its pre-existing placeholder geometry. Exact inventory becomes 20,427 rows (10,093 production and 10,334 fixtures), multiplicity 22,801 (11,722 production and 11,079 fixtures); direct production falls 3,564→3,502, reviewed adapters rise 4,180→4,272 and composition rises 2,317→2,319. Six unstable workflows become nineteen stable adapter workflows, leaving 965 workflows (616 stable and 349 unstable) and 969 policy groups (616 stable and 353 unstable). |
| Vehicle startup capabilities | `wow_persistence::VehicleHotfixPersistencePortLikeCpp` owns SQLx-free Vehicle/VehicleSeat overlay rows and typed outcomes; the distinct `VehicleWorldCatalogPersistencePortLikeCpp` owns typed template and accessory row families without implying cross-database atomicity | `wow_database::vehicle_catalog_adapter` alone owns the two Hotfix statements, three exact World SQL shapes, concrete handles, row decoding and query-failure classification | `wow_data::vehicle` retains DB2 parsing, overlay replacement, vehicle/seat rules, immutable template/accessory stores, duplicate/group order and spawn-specific precedence; private `world-server::vehicle_catalog` converts DTOs and preserves logs/publication | Hotfix and World remain independent connections. Startup order stays Vehicle DB2/overlay, VehicleSeat DB2/overlay, template, template accessories, spawn accessories; each failure remains fatal before its affected store publication and no missing C++ validation is smuggled into the structural cut | #507 removes all 91 concrete production entries from `wow-data::vehicle`. The exhaustive inventory falls to 20,778 exact rows (10,478 production and 10,300 fixtures); direct production access falls 4,887→4,796, reviewed-adapter access rises 3,369→3,436 and composition rises 2,240→2,246. Nine unstable workflows disappear and fourteen stable adapter workflows replace them, leaving 900 workflows (473 stable and 427 unstable). |
| Core SpellInfo Hotfix DB2 capability | `wow_persistence::SpellCoreDb2HotfixPersistencePortLikeCpp` owns SQLx-free typed rows and loaded/failed outcomes for the DB2 authorities used by represented SpellInfo and startup spell metadata | `wow_database::spell_core_db2_hotfix_adapter` alone owns the Hotfix statements, official/custom bind order, concrete handle, tolerant row decoding and query-failure classification | `wow_data::spell_db2` retains WDC4 parsing, record replacement and final `hotfix_data` tombstones; `wow_data::SpellStore` retains derived hydration; private `world-server::spell_core_db2_hotfix` converts boundary DTOs and composes the effective stores | each family preserves DB2 → official → custom → final tombstones. Query failure remains fatal before affected publication | #509 moved eleven core contributors and removed 236 net direct production accesses. #513 extends the same bounded capability to AuraRestrictions, Category, Duration, Radius, Range, EquippedItems, TargetRestrictions and XSpellVisual, removing all 171 remaining concrete accesses from `wow_data::spell_db2`. Exact inventory falls 20,616→20,501 (production 10,315→10,200; fixtures stay 10,301); direct production falls 4,472→4,301, reviewed adapters rise 3,589→3,637 and composition rises 2,254→2,262. Workflows fall 905→904 while stable boundaries rise 497→505; exact policy groups fall 909→908. |
| Exact SpellInfo key Hotfix capability | `wow_persistence::SpellInfoKeyHotfixPersistencePortLikeCpp` owns the SQLx-free twenty-contributor manifest, ordered typed batches, SpellPowerDifficulty rows and loaded/failed outcome | `wow_database::spell_info_key_hotfix_adapter` alone owns the twenty contributor SQL shapes plus the SpellPowerDifficulty join, official/custom binds, concrete Hotfix handle and tolerant row decoding | `wow_data::spell_info_keys` retains all WDC4 parsing, replacement by RecordID, final tombstones, candidate-key composition, SpellName filtering and SpellPower difficulty fallback; private `world-server::spell_info_key_hotfix` converts boundary DTOs | the adapter queries contributors in C++ `LoadSpellInfoStore` order, with SpellPowerDifficulty immediately after SpellPower; every query keeps official before custom, and publication waits for the complete typed outcome | #511 removes all 88 direct production accesses from the exact-key seed without adding `wow-data → wow-persistence`. The exhaustive inventory falls to 20,616 exact rows (10,315 production and 10,301 fixtures); direct production access falls 4,560→4,472, reviewed-adapter access rises 3,526→3,589 and composition rises 2,252→2,254. Seven unstable workflows become eight stable adapter workflows, leaving 905 workflows (497 stable and 408 unstable) and 909 exact policy groups (497 stable and 412 unstable). |
| SpellMgr World-catalog capability | `wow_persistence::SpellWorldCatalogPersistencePortLikeCpp` owns SQLx-free typed rows and loaded/failed outcomes for all ten represented SpellMgr World catalog families | `wow_database::spell_world_catalog_adapter` alone owns the ten exact World statements, concrete handle and existing tolerant row decoding | `wow_data::spell::stores` retains spell/effect/rank/area/quest/map and proc validation, duplicate handling, derived indexes, warnings and immutable catalogs; private `world-server::spell_world_catalog` converts boundary DTOs and preserves startup calls | every table remains an independent non-transactional World read; empty results are successful empty batches, query failure remains fatal before the affected publication, nullable orientation remains unknown, and the pre-cut startup order is unchanged | #515 removes 115 direct production accesses from the five foundational loaders. #517 completes the same narrow port for SpellArea, SpellTargetPosition, SpellProc, SpellGroup and SpellGroupStackRule and removes the concrete database import from `wow_data::spell::stores`. Exact inventory falls 20,449→20,398 (production 10,145→10,094; fixtures remain 10,304), multiplicity 22,748→22,714, direct production falls 4,186→4,093, reviewed adapters rise 3,691→3,728 and composition rises 2,268→2,273. Six unstable workflows become stable adapter workflows, leaving 910 workflows (522 stable and 388 unstable) and 914 policy groups (522 stable and 392 unstable). |
| Trainer startup-catalog capability | `wow_persistence::TrainerCatalogPersistencePortLikeCpp` owns one SQLx-free batch of trainer-spell, trainer, locale and creature-trainer rows plus a loaded/failed outcome | `wow_database::trainer_catalog_adapter` alone owns the four World statements, concrete database handle and tolerant row decoding | `wow_data::trainer` retains trainer/spell/locale/creature association ownership, C++-ordered catalog validation, duplicate behavior and diagnostics; private `world-server::trainer_catalog` converts the typed rows and supplies existing immutable catalog authorities | reads remain sequential and non-transactional in the represented C++ order: trainer spells, trainers, locales, creature trainers; each empty result is a successful empty batch and any query failure remains fatal before domain publication | #519 removes the concrete World database and statement vocabulary from `wow_data::trainer` without moving gameplay validation into persistence or fabricating missing values. Exact inventory falls 20,398→20,366 (production 10,094→10,060; fixtures 10,304→10,306), multiplicity 22,714→22,670, direct production falls 4,093→4,020, reviewed adapters rise 3,728→3,765 and composition rises 2,273→2,275. Two unstable workflows become six stable adapter workflows, leaving 914 workflows (528 stable and 386 unstable) and 918 policy groups (528 stable and 390 unstable). |
| ChrSpecialization Hotfix capability | `wow_persistence::ChrSpecializationHotfixPersistencePortLikeCpp` owns SQLx-free official/custom overlay rows plus a loaded/failed outcome | `wow_database::chr_specialization_hotfix_adapter` alone owns the Hotfix statement, verified-build bind, concrete handle and checked integer decoding | `wow_data::ChrSpecializationStore` retains WDC4 parsing, replacement by record ID, the pre-removal class/order index and final tombstones; private `world-server::chr_specialization_hotfix` converts boundary rows | WDC4 loads before the port is called; the adapter then fully decodes official before custom, and domain publication waits for both batches. Missing or out-of-range integers fail rather than becoming defaults | #521 removes the concrete Hotfix database, statement and result vocabulary from `wow_data::chr_specialization` without moving DB2 authority or loot-spec rules. Exact inventory falls 20,366→20,346 (production 10,060→10,038; fixtures 10,306→10,308), multiplicity 22,670→22,651, direct production falls 4,020→3,954, reviewed adapters rise 3,765→3,807 and composition rises 2,275→2,277. Six unstable workflows become six stable adapter workflows, leaving 914 workflows (534 stable and 380 unstable) and 918 policy groups (534 stable and 384 unstable). |
| Effective skill-catalog Hotfix capability | `wow_persistence::SkillCatalogHotfixPersistencePortLikeCpp` owns SQLx-free rows and loaded/failed outcomes for one cross-indexed SkillLine/ability/race-class authority, exposed as two dependency-ordered startup stages rather than table CRUD | `wow_database::skill_catalog_hotfix_adapter` alone owns the three Hotfix statements, verified-build binds, concrete handle and checked SQL decoding | `wow_data::{SkillLineStore, SkillStore}` retain WDC4 parsing, overlay precedence, incomplete/invalid diagnostics, tombstones, derived indexes and gameplay queries; private `world-server::skill_catalog_hotfix` converts and composes the typed rows | SkillLine must publish before relation validation; any stage query/decode failure prevents that store's publication. The adapter deliberately preserves the pre-#523 Rust relation order; C++ finishes official/custom per table, and that behavior correction is isolated in #524 | #523 removes Hotfix database, statement and result vocabulary from both skill domain modules without moving the independent WorldDatabase `SkillTiers` lifecycle or adding a dependency edge. Exact inventory falls 20,346→20,328 (production 10,038→10,013; fixtures 10,308→10,315), multiplicity 22,651→22,631, direct production falls 3,954→3,853, reviewed adapters rise 3,807→3,880 and composition rises 2,277→2,280. Eight unstable workflows become eight stable adapter workflows, leaving 914 workflows (542 stable and 372 unstable) and 918 policy groups (542 stable and 376 unstable). |
| Immutable World skill-rules capability | `wow_persistence::SkillWorldRulesPersistencePortLikeCpp` owns SQLx-free fishing-base and 16-value skill-tier rows, two independent load outcomes and no table-generic query surface | `wow_database::skill_world_rules_adapter` alone owns the two World statements, concrete handle and checked signed/unsigned integer decoding | `wow_data::FishingBaseSkillStoreLikeCpp` retains AreaTable validation and duplicate replacement; `wow_data::SkillTiersStoreLikeCpp` retains immutable tier ownership; private `world-server::skill_world_rules` converts and publishes each store at its existing startup point | the same adapter is composed once; fishing publishes at its earlier AreaTable-dependent point and tiers at their later point. Each query fully decodes before publication, empty is a valid empty batch, and failure remains explicit without defaults or a partial batch | #526 removes World database, statement and result vocabulary from both skill-rule domain modules without combining their distinct publication points or changing the #523 Hotfix lifecycle. Exact inventory becomes 20,344 rows (10,025 production and 10,319 fixtures), multiplicity 22,656 (11,592 production and 11,064 fixtures); direct production falls 3,853→3,806, reviewed adapters rise 3,880→3,936 and composition rises 2,280→2,283. Three obsolete unstable workflows become ten stable adapter workflows, leaving 921 workflows (552 stable and 369 unstable) and 925 policy groups (552 stable and 373 unstable). |
| Player base-stat World capability | `wow_persistence::PlayerBaseStatsPersistencePortLikeCpp` owns SQLx-free race-modifier and class-level-base rows plus explicit loaded/failed outcomes for two dependency-ordered stages | `wow_database::player_base_stats_adapter` alone owns the two World statements, concrete handle and checked integer decoding, including C++ uint16 conversion for the signed SQL spirit column | `wow_data::player_stats` retains empty-table integrity checks, `gt/BaseMp.txt`, valid race/class selection, configured-level filtering, duplicate replacement, signed modifier combination, gap filling and immutable `PlayerStatsStore` ownership; private `world-server::player_base_stats` converts and sequences the stages | one adapter is composed once; race rows are queried and domain-validated before the class-level query can run, preserving C++'s race-empty short circuit. BaseMp loads only after both stages validate, and final publication remains atomic | #528 removes all concrete World database, statement and result vocabulary from `wow_data::player_stats` without moving stat rules or local game-table parsing into persistence. Exact inventory becomes 20,354 rows (10,031 production and 10,323 fixtures), multiplicity 22,676 (11,608 production and 11,068 fixtures); direct production falls 3,806→3,756, reviewed adapters rise 3,936→3,990 and composition rises 2,283→2,285. Two unstable domain workflows become ten stable adapter workflows, leaving 929 workflows (562 stable and 367 unstable) and 933 policy groups (562 stable and 371 unstable). |
| Represented Player-creation World capability | `wow_persistence::PlayerCreationCatalogPersistencePortLikeCpp` owns SQLx-free base-definition, cast-spell and custom-spell rows plus explicit loaded/failed outcomes for three staged operations | `wow_database::player_creation_catalog_adapter` alone owns the three World statements, concrete handle, nullable-column distinction and checked numeric decoding | `wow_data::player_create` retains race/class/model/map/position validation, complete NPE-position grouping, represented transport-template validation, mask/create-mode expansion, diagnostics and immutable store ownership; private `world-server::player_creation_catalog` converts and publishes each stage | one adapter is composed once. Empty base definitions remain fatal before later startup work; cast and custom empty batches remain valid. The cut preserves the existing Rust base → cast → custom publication order even though C++ loads custom before cast, and does not claim full `TransportMgr::GetTransportSpawn` parity | #530 removes all 71 concrete production persistence accesses from `wow_data::player_create` without adding an eager query bundle, a generic repository or a new process. Exact inventory becomes 20,370 rows (10,042 production and 10,328 fixtures), multiplicity 22,715 (11,642 production and 11,073 fixtures); direct production falls 3,756→3,685, reviewed adapters rise 3,990→4,068 and composition rises 2,285→2,289. Four unstable workflows are retired and fifteen stable adapter workflows are added, leaving 940 workflows (577 stable and 363 unstable) and 944 policy groups (577 stable and 367 unstable). |
| Effective Difficulty Hotfix capability | `wow_persistence::DifficultyHotfixPersistencePortLikeCpp` owns SQLx-free official/custom batches of the five represented fields plus an explicit loaded/failed outcome | `wow_database::difficulty_hotfix_adapter` alone owns `SEL_DIFFICULTY`, the verified-build bind order, concrete Hotfix handle and checked C++-width integer decoding | `wow_data::DifficultyStore` retains WDC4/table-hash ownership, whole-row replacement, final tombstones and gameplay queries; private `world-server::difficulty_hotfix` converts and publishes the effective store | WDC4 loads before the port is called; the adapter fully decodes official before custom and returns neither batch on failure; final removals run only after both successful batches. Empty batches preserve the base authority | #532 removes all 59 direct production identities (multiplicity 74) from `wow_data::difficulty` without inventing defaults, a table repository or another process. Exact inventory becomes 20,357 rows (10,027 production and 10,330 fixtures), multiplicity 22,699 (11,624 production and 11,075 fixtures); direct production falls 3,685→3,626, reviewed adapters rise 4,068→4,110 and composition rises 2,289→2,291. Four unstable workflows are retired and six stable adapter workflows are added, leaving 942 workflows (583 stable and 359 unstable) and 946 policy groups (583 stable and 363 unstable). |
| Hotfix delivery-metadata capability | `wow_persistence::HotfixDeliveryMetadataPersistencePortLikeCpp` owns SQLx-free blob, push/data and optional-data rows with three separately invoked loaded/failed stages | `wow_database::hotfix_delivery_metadata_adapter` alone owns the three exact Hotfix statements, concrete handle, binary/string reads and checked C++-width integer decoding | `wow_data::HotfixBlobCache` retains local WDC4 record discovery, locale admission, known-store/blob filtering, push grouping and optional-data indexing; `wow_data::Db2HotfixRemovalStoreLikeCpp` retains the last-status tombstone projection; private `world-server::hotfix_delivery_metadata` converts and applies each stage | one adapter is composed once before the early effective-removal read and reused later. The early data read remains fatal; the later blob, data and optional-data reads/applications remain ordered but independently warn-and-continue. Reads deliberately remain separate, preserving connection/timing and failure behavior rather than introducing caching or cross-stage atomicity | #534 removes all 62 concrete production identities from `wow_data::hotfix_cache` without an eager table bundle, repository, RPC or new process. #538 then removes the independent 59-identity `wow_data::db2_hotfix` SQL implementation by reusing the same typed data capability while preserving C++ last-status semantics and the two existing read points. Exact inventory falls to 20,369 rows (10,035 production and 10,334 fixtures), multiplicity 22,734 (11,655 production and 11,079 fixtures); direct production falls 3,502→3,443, reviewed adapters remain 4,272 and composition rises 2,319→2,320. Five unstable workflows disappear, leaving 960 workflows (616 stable and 344 unstable) and 964 policy groups (616 stable and 348 unstable). This ownership-only cut explicitly does not claim C++ parity for all-locale masks, typed `DB2StorageBase::WriteRecord`, blob/store diagnostics or optional-key/data validation. |
| Creature display/model Hotfix capability | `wow_persistence::CreatureDisplayHotfixPersistencePortLikeCpp` owns SQLx-free consumed-field rows and independent loaded/failed outcomes for CreatureDisplayInfo and CreatureModelData | `wow_database::creature_display_hotfix_adapter` alone owns both exact Hotfix statements, the concrete handle and checked integer/float decoding | `wow_data::creature_display` retains WDC4 parsing, by-ID last-row replacement, immutable store ownership and collision/model math; private `world-server::creature_display_hotfix` converts and publishes each stage | one adapter is composed once; CreatureDisplayInfo WDC4/overlay/publication completes before CreatureModelData WDC4/overlay/publication. Empty is a successful overlay and any query/decode failure remains fatal before mutation of that stage | #540 removes all 56 concrete production identities from `wow_data::creature_display` without adding custom overlays, tombstones, a generic DB2 repository, RPC or a new process. Exact inventory becomes 20,386 rows (10,049 production and 10,337 fixtures), multiplicity 22,768 (11,686 production and 11,082 fixtures); direct production falls 3,443→3,387, reviewed adapters rise 4,272→4,339 and composition rises 2,320→2,323. Five unstable workflows become nine stable adapter workflows, leaving 964 workflows (625 stable and 339 unstable) and 968 policy groups (625 stable and 343 unstable). |
| Gossip startup catalog capability | `wow_persistence::GossipStartupCatalogPersistencePortLikeCpp` owns SQLx-free menu, option, locale and addon rows plus four independent loaded/failed outcomes; the existing on-demand gossip port remains a distinct runtime view of the same capability | the existing `wow_database::gossip_catalog_adapter` implements both typed views and alone owns the four startup statement identities, the shared World handle, checked integer/null decoding and MariaDB errors | `wow_data::gossip` retains menu/option containers, locale filtering, duplicate replacement, condition attachment and reports; private `world-server::gossip_startup_catalog` maps and publishes the single store | one adapter instance is composed before the startup read and reused by later runtime wiring. Startup preserves Rust's current menu → options → locales → addon order, fully decodes each batch, accepts empty batches and stops before later reads/publication on failure. C++ loads locales in an earlier localization phase; reconciling that pre-existing order drift is behavior work outside this boundary-only slice | #542 removes all 61 concrete production identities from `wow_data::gossip` without introducing a table repository, eager universal catalog, RPC, process or cache. Exact inventory becomes 20,413 rows (10,064 production and 10,349 fixtures), multiplicity 22,803 (11,709 production and 11,094 fixtures); direct production falls 3,387→3,326, reviewed adapters rise 4,339→4,413 and composition rises 2,323→2,325. Two unstable workflows are retired and thirteen stable adapter workflows added, leaving 975 workflows (638 stable and 337 unstable) and 979 policy groups (638 stable and 341 unstable). |
| Effective Phase Hotfix capability | `wow_persistence::PhaseHotfixPersistencePortLikeCpp` owns SQLx-free Phase and PhaseXPhaseGroup rows plus two independently invoked loaded/failed stages | `wow_database::phase_hotfix_catalog_adapter` alone owns both Hotfix statements, the concrete handle and checked C++ unsigned-width decoding | `wow_data::{PhaseStore, PhaseGroupStore}` retain WDC4 parsing, whole-row replacement, effective phase validation, group-index rebuilding and gameplay queries; private `world-server::phase_hotfix_catalog` converts and coordinates the pair | Phase WDC4 loads before its overlay; only the effective Phase store may feed PhaseXPhaseGroup WDC4 validation, after which the group overlay rebuilds the index. Either read failure prevents every later stage and publication of the pair | #544 removes all 50 concrete production identities from `wow_data::phase` without introducing a generic DB2 repository, RPC, process or cache. Exact inventory becomes 20,421 rows (10,069 production and 10,352 fixtures), multiplicity 22,821 (11,724 production and 11,097 fixtures); direct production falls 3,326→3,276, reviewed adapters rise 4,413→4,466 and composition rises 2,325→2,327. Five unstable workflows become nine stable adapter workflows, leaving 979 workflows (647 stable and 332 unstable) and 983 policy groups (647 stable and 336 unstable). |
| Immutable World phasing capability | `wow_persistence::PhaseWorldCatalogPersistencePortLikeCpp` owns SQLx-free terrain-world-map, terrain-default, phase-area and phase-name rows behind four independent loaded/failed operations | `wow_database::phase_world_catalog_adapter` alone owns the four exact World statements, one concrete handle, complete-batch query decoding and checked unsigned widths | `wow_data::{PhaseInfoStore, PhaseNameStoreLikeCpp, TerrainSwapStore}` retain phase/area/map/UI-map validation, sub-area-exclusion derivation, duplicate behavior, immutable indices and gameplay queries; private `world-server::phase_world_catalog` converts and composes the stores | one adapter is composed once. Production preserves Rust's existing phase-area → phase-name → terrain-world-map → terrain-default order, fully decodes each batch and returns no catalog tuple after any failure. C++ `LoadPhases` performs the terrain stages before area phases and loads phase names later; correcting that pre-existing startup-order drift is behavior work outside this boundary-only slice | #546 removes all 80 concrete production identities (multiplicity 89) from `wow_data::{phasing,terrain_swap}` without adding `wow-data → wow-persistence`, a generic repository, RPC, process or cache. Exact inventory becomes 20,424 rows (10,071 production and 10,353 fixtures), multiplicity 22,825 (11,727 production and 11,098 fixtures); direct production falls 3,276→3,196, reviewed adapters rise 4,466→4,546 and composition rises 2,327→2,329. Five unstable workflows become seventeen stable adapter workflows, leaving 991 workflows (664 stable and 327 unstable) and 995 policy groups (664 stable and 331 unstable). |
| Session login/logout lifecycle | private `wow_world::session::lifecycle`: `login` (the single-live-session character claim), `logout` (timed logout finalize and the disconnect save), `cleanup` (registry/visibility/map/accessor teardown) | the owning Session task on its exit paths | the Session driver's logout timer, the disconnect path in the composition root, and the login handlers | claim held from before the login sequence commits until any exit path; cleanup tears down publication before ownership, and the disconnect save keeps the represented player alive until it has run — C++ `LogoutPlayer(true)` saves while `_player` still exists | #184 extracted the exact current behaviour, concrete DB calls included, behind one private seam. #200 replaces that persistence seam once #187 freezes the focused Player contract. |
| Session phase driver | private `wow_world::session::driver`: the ordered pass (`update` ingestion + Session timers, `process_pending` async phases), the shared ingestion budget in `driver::budget`, and the frozen phase trace in `driver::phases` | the one Session task; there is no second scheduler | the composition root that spawns the Session task calls the pass and owns cadence, cancellation and the idle sleep | one pass per loop iteration; ingestion is bounded by a single shared budget so a busy realm channel cannot starve the instance channel, and exit is decided inside the pass (disconnected channel, idle deadline, logout timer) | #183 extracted the driver from `session/mod.rs`. It is deliberately not the world/Map/gameplay tick owner — those clocks are unchanged and traced in `runtime-clock-phase-trace.md` (#188). #28 and #153 own semantic convergence. |
| Logical realm/instance connection | `wow_session::SessionConnection` since #297: the attach/switch/restore state machine, realm-vs-primary send selection and the two cross-socket ordering fences, in a crate that cannot reach gameplay, a map or a database. `wow_world::session::connection` is now a delegation shim that performs the session steps the kernel reports back | the owning Session task, which is the only writer of the logical primary | `send_packet_realm`/`send_raw_packet_realm` callers, the Session driver polling the instance link, and logout restore | connection lifetime; after `SMSG_CONNECT_TO` the instance channel becomes primary and the realm channel is parked, never closed — the client drops the session if either socket dies | #182 extracted the transition implementation out of `session/mod.rs` and #297 moved it into the `wow-session` crate, taking the eleven channel, fence and ConnectTo fields with it; `WorldSession` now holds one opaque `connection` handle. `account_id` and `player_loading` deliberately stayed behind — identity and login-loading state are not transport. #378 classified the five remaining module families and proved that each is still an application/gameplay adapter with a named authority or port blocker; #153 owns replacing the concrete `wow-network` types with ports. |
| Gameplay session construction aggregate | private `SessionResources` in `crates/world-server/src/session_resources.rs` | `world-server` bootstrap constructs it; the outer session callback captures and clones the `Arc` | `world-server::create_session` copies stores, registries, DB adapters, and runtime handles into each `WorldSession`; the aggregate never enters `wow-network` | process lifetime; no independent clock | #134 moved the aggregate to composition code; #136 moved the process into a library-backed composition root and extracted the private factory. Capability-specific dependency reduction remains in the physical and ownership lanes. |
| Session directory | opaque `wow_world::session::directory::PlayerRegistry`; its `DashMap<ObjectGuid, private entry>` storage is private to the owner module | named generation-aware registration/unregistration owns lifecycle; canonical gameplay writes remain outside the directory | consumers use bounded Player/Map-owned results and generation-checked delivery; durable loot rolls publish only the identities required to address the owning session | process lifetime; presence, incarnation and addressability are Session concerns; gameplay has no directory mirror | #150 installed the opaque lifecycle seam; #192-#196 closed broad access; #138 relocated the directory; #252 removed `PlayerBroadcastInfo` and its field ratchets. #378 keeps the application adapter out of P4, and #153 performs the terminal classification of the remaining narrow seam. |
| Session mailbox and durable rails | private `wow_world::session::mailbox::{protocol, durable, pump}`: `SessionCommand` plus every ordinary payload, `SharedClientVisibleGuidsLikeCpp`, the durable creature-runtime rail and the single pump. #189 moved the durable loot-money coordinator to `wow_world::loot_persistence` | session/global runtime and gameplay producers enqueue commands; one session task consumes them | the owning session consumes FIFO commands and publishes acknowledgements | connection/session lifetime; queue identity, capacity, FIFO, incarnation fences, acknowledgements, and shutdown drain are observable | #191 and #190 completed their bounded protocol/rail relocations, #138 closed broad directory access, and #140 moved the complete vertical without changing queue identity, FIFO, durability, capacity, incarnation fences, acknowledgements or shutdown drain. `wow-network` now owns only transport primitives. #189 still removes durable loot persistence coordination. |
| Group registry and pending invites | `wow_social::group::{GroupRegistry, PendingInvites}`, backed by concurrent maps inside private `model`/`invites`/`membership`/`settings`/`ready_check`/`outcome` submodules | group handlers and group timer/ready-check paths | group handlers, connected-player fanout, world-server ready-check loop | process lifetime; timed group work is driven outside the network listener | #151 created opaque facades; #197 and #198 centralized atomic transitions; #199 separated persistence/publication and closed storage; #195 adapted session addressing; #137 moved the owner into `wow-social`. The transitional `wow-network → wow-social` edge exists only because the Session mailbox still names `GroupDifficultyKindLikeCpp`; #140 removes it. |
| Legacy creature runtime | shared `wow_world::MapManager` behind `Arc<RwLock<_>>` | production `GlobalLegacy` runtime tick plus explicit spawn/respawn bridges; `Session` writer exists only for tests and the diagnostic config override | world handlers, global runtime bridge, visibility/fanout routing | process lifetime; production startup defaults `RuntimeTickOwner` to `GlobalLegacy` and uses the configured map-update interval; session ticks read the shared owner and must skip to prevent double resolution. #371 retired each creature's independent wall-clock epoch: the owning tick advances one logical elapsed value exactly once from its propagated `diff_ms`, and spline, motion generator, melee, spells, assistance, corpse and respawn deadlines all read it | #188 froze the pre-cut trace. #28 moved the player melee transition to the selected owner; #371 completed the named clock-identity cut without changing packet or bridge ownership. Later cuts retire legacy behavior method-by-method under `docs/migration/adr-runtime-tick-ownership.md`. |
| Canonical map runtime | `wow_map::MapManager` | canonical global map loop, grid/spawn/respawn paths, explicit selected legacy-result adapters | world-server orchestration and session map/player bridges | process lifetime; canonical loop uses the configured map interval; preserves the C++ `Map::Update` phase order represented by the ADR | #188 records the current phase trace. It becomes the sole map/entity authority only after each later method cut has parity tests and removes the corresponding legacy writer. |
| Creature legacy/canonical mirror | canonical loaded-grid records are mirrored into `wow_world::MapManager`; selected lifecycle, movement, aggro, attack-stop, melee, health, and respawn outcomes are explicitly bridged back to canonical state | named bridge functions in `world-server`, including `mirror_loaded_grid_creature_to_legacy_like_cpp` and the `run_legacy_creature_*_and_deliver_once_like_cpp` family | both runtimes and post-lock packet/command delivery | load/respawn synchronization begins canonical → legacy; only explicitly modelled runtime outcomes travel legacy → canonical; delivery occurs after map locks are released | #181 inventories every bridge and #188 freezes its phase trace. Remove one bridge only when its destination runtime becomes authoritative for that whole transition; never add generic bidirectional sync. |
| Represented player gameplay state | mostly fields on `wow_world::WorldSession`; canonical value types and partial state also exist in `wow_entities::Player` | session handlers and session update code | packet builders, persistence helpers, `PlayerRegistry` summaries, canonical snapshot bridges | connection/selected-character lifetime; `canonical_player_entity_snapshot_*_like_cpp` currently rebuilds a `Player` snapshot from represented session fields | #181 records field families, writers, mirrors, and cutover owners. After the Session shell lands, #153 materializes one-responsibility cuts until `Player` is the mutable gameplay owner and `WorldSession` is only the connection/session bridge. |
| Concrete persistence access | `persistence-access-snapshot.json` inventories exact SQLx and concrete `wow-database` syntax across application, data, instance, composition and adapter code; `persistence-boundary-policy.json` assigns every row exactly once | `wow-database` is the stable concrete adapter; each remaining group names its current capability owner, logical database(s), affinity and open removal/decision issue | handlers, lifecycle, bootstrap/loaders, runtime recovery, tests and publication paths consume concrete outcomes | statement order, connection affinity, commit/rollback/unknown-commit classification, fences, and publication order are observable; cross-database groups explicitly use independent connections and never imply distributed ACID | #186 installed the exact non-growth/stale-exception guard; #187 freezes ordered behavior; #200 earns the SQLx-free Player lifecycle port; #189 moves durable loot coordination; #153 materializes the remaining measured capability/data/auth/instance children. |
| Effective skill metadata | `wow_data::SkillLineStore` owns final `SkillLine` identity/acquisition fields; `wow_data::SkillStore` owns final `SkillLineAbility` and `SkillRaceClassInfo` rows plus their derived indexes | `world-server` bootstrap composes WDC4 → official SQL → custom SQL → final removals once; no runtime writer | spell loaders and gameplay validation read immutable stores shared with sessions | process lifetime; `SkillLine` is composed first, then dependent rows are filtered and every index is rebuilt from final records in ascending ID order | Retire the specialized acquisition projections only when the general effective DB2 authority carries the same checked payload and coverage states. Never reactivate the raw WDC-only `SkillStore::load` path in production. |
| Effective spell-acquisition metadata | `wow_data::SpellAcquisitionCatalogLikeCpp`, a compact immutable projection of the seven acquisition source families | `world-server` bootstrap composes and publishes one `Arc`; no handler or session mutates it | derived spell-learning loaders now; trainer planning in #164; sessions receive the same `Arc`, not the seven raw stores | process lifetime; exact regular SpellInfo keys seed covered/zero distinction, while server-side keys without validated acquisition payload are explicitly indeterminate | Remove the specialized catalog, or feed it from the general store, once full effective `SpellInfo` payload authority exists. This row does not authorize packet, persistence, spell, skill, money, or battle-pet mutation. |
| Immutable spell-acquisition projection and application | `wow_world::spell_acquisition` owns the pure fixed-point plan plus its validation/transaction/publication boundary; the live player and Character DB remain the runtime/durable owners | planning mutates only a private ordered copy; #158 locks one character row and commits the complete durable result; #159 extends that same transaction with guarded money and keeps the exclusion through runtime/packet publication | #157 consumes ordered primary-profession outcomes; #158 consumes the exact source/result plan and generic player `EffectLearnSpell`; #159 consumes a startup-audited cast/craft authority plus a fresh player effect mask and owns trainer charge/wrapper/visual orchestration | one acquisition operation; complete spell/skill/trait/override authority, exact slot occupancy and wrapper static/live proofs are mandatory. Unknown COMMIT outcomes reconcile money plus all spell/favorite/skill rows before publication; see [the detailed contract](spell-acquisition-plan.md) | Retire the specialized seam only when canonical `Player` methods expose the same atomic dry-run/apply contract. Never reconstruct capacity or criteria from a flat trigger list, sort profession outcomes, infer “no immunity” from missing runtime state, or publish before the durable boundary. |
| Battle-pet trainer purchase saga | `wow_world::battle_pet_purchase` owns the durable command (`character_battle_pet_purchase`), its state transitions and login recovery; Character DB money stays under the #159 exclusive per-character guard; the pet itself stays under the #160 account owner | the saga is the sole writer of the command table and the sole caller that turns a trainer offer into a #160 add; it spawns no tasks and holds no lock across `.await` other than the pre-existing async money mutex | buy handler adapts request → offer decision → saga; the #160 owner keeps fence/journal-lease/capacity authority; the #163 catalog keeps species-classification authority; the world-DB selection store keeps breed/quality/display authority | one purchase command per 128-bit request key (shared with the #160 receipt); charge+command, publication marker, completion, and refund+flip are each single Character DB transactions; `PetApplied` is derived from the Login DB receipt and a Completed row with a clear `published` marker is owed its publication by recovery | Retire only when a portable cross-pool transaction exists. Never publish before the pet is durable, never refund a durable pet, never recover from in-memory flags, never activate the `TrainerBuySpell` dispatcher arm outside #142. |
| Trusted linked module API | `wow-module-api`: validated `ModuleId`/version/descriptor, the explicit `ModuleRegistry`, the immutable `PlayerLoginSnapshot` and the typed `SendSystemMessageSelf` effect | modules only queue effects; the Session owner applies the validated batch | `WorldSession::dispatch_module_player_login_like_cpp` at the C++ `ScriptMgr::OnPlayerLogin` position | process lifetime; dispatch is deterministic in `ModuleId` order and the batch is validated before anything is applied | #228 earned the crate through one working vertical. #229-#231 add external Cargo composition, the module manager and typed configuration. No stable ABI or hot reload is promised. |
| Handler registration and dispatch-arm contract | the sole `inventory::collect!(PacketHandlerEntry)` in `wow-handler`, link-time `inventory::iter<PacketHandlerEntry>` consumed by `wow_handler`/`WorldSession`, and the concrete `WorldSession::dispatch_packet` opcode arms | unconditional module-item `inventory::submit!` declarations owned by the logical `wow-world::crate::handlers` tree, plus one dispatcher owned by `wow-world::crate::session`; both owners and their private descendants are declared in `handler-module-policy.json` | dispatch table and session update driver | compile/link lifetime; no mutable clock | #142 removed the final one-sided entries. #185 makes module ownership independent of physical filenames and fails closed on conditional, missing, duplicate, remounted, malformed, or stale ownership. #139 proves one thin capability, and #152 moves admission/dispatch without altering the exact opcode/metadata/arm contract. The terminal router inversion is re-audited by #153. |

#432 extends the Player lifecycle persistence row above with the final three
concrete `world_entry.rs` writes: two independently classified pet-talent reset
operations and the best-effort character-online mark. `wow-world` retains the
at-login decision, warning/publication behavior and existing call positions;
`wow-database` owns statement identity, binds and execution. The known C++
account-online/timing and UInt64-bind differences remain explicit fidelity work,
not hidden inside this ownership refactor.

## Non-negotiable runtime invariants

The refactor campaign must preserve:

- exactly one tick owner for each state transition; no dual session/global resolution;
- C++ update ordering, especially session/map phases and creature `Unit` → threat → AI → melee
  sequencing;
- no packet or cross-session command delivery while a map lock is held;
- exact active handler registration metadata and dispatch-arm coverage: opcode value/name,
  `SessionStatus`, `PacketProcessing`, handler name, registration, and concrete opcode arm;
- persistence mutation order and failure semantics;
- packet bytes, connection choice, and capture-diff behavior unless an intentional compatibility
  deviation is separately approved and documented;
- no new public API, state copy, synchronization primitive, or crate dependency solely for file
  movement.

## Guardrail commands

Ordinary architecture/module work uses the syntax-only ownership ratchet plus the local-first
checks:

```bash
python3 tools/architecture/check_architecture.py self-test
python3 tools/architecture/check_architecture.py check
python3 tools/architecture/check_architecture.py hotspots --limit 20
cargo run --release --locked --manifest-path tools/architecture/handler-contract-check/Cargo.toml --bin session-ownership-check -- check --syntax-only
./tools/validation-v2 final --base origin/3.4.3
```

`session-ownership-check -- check` without `--syntax-only` recomputes the exhaustive persistence
inventory as well as the syntax surface. Run it, or `./tools/validation-v2 audit`, only
for an explicitly requested persistence/architecture audit, a release/scheduled audit, or a change
that actually owns those exhaustive artifacts. A mechanical module move does not require the full
persistence scan. Preserve validator exit codes; do not pipe them through output truncation or a
trailing command that can mask failure.

Ledger schema v2 distinguishes epics from one-PR slices, validates parents and internal/external
prerequisites, rejects unknown dependencies, self-dependencies and cycles, and proves that the
documented sequence is a complete topological ordering of the slices. A closed slice cannot depend
on an open prerequisite. The checked-in state remains an offline reviewed snapshot; the guard does
not contact GitHub or silently rewrite titles, states, or higher baselines.

Because the snapshot is offline, its `state` fields are the guard's weakest point: while a slice is
recorded `open`, neither the closed-slice-prerequisite rule nor the completed-issue-exception rule
can fire against it. Issue #258 found sixteen states lagging at once, which had made both rules
inert. Two conventions keep that from reaccumulating. First, **`depends_on` records hard
prerequisites only** — the exact issues named after "Depends on" in the issue body. A conditional
reference ("required only if timing or authority changes"), a contrast ("uses focused combat/loot
contracts instead of the Player lifecycle trace") or a plain mention belongs in the issue text and
in `Refs`, never in `depends_on`; #258 removed four such over-declared edges, all of which had
recorded a conditional or explicitly rejected reference as a hard one. Second, **an `open_*` list
means open**: any slice that closes an issue resyncs the states and drops that number from every
`open_retirement_issues` and `cutover_issues` list in the same PR, so a completed issue can never
keep standing as somebody's future owner.

`handler-module-policy.json` is the offline authority for packet-dispatch and
handler-registration module ownership. Each capability declares one Cargo package, one logical
Rust module root, whether private descendants belong to that root, and an open issue that owns its
next move or terminal re-audit. The architecture checker rejects malformed policy, unknown or
completed issue ownership, duplicate capabilities, and overlapping owner trees. The Rust handler
checker consumes the same file, walks Cargo-declared production module graphs, and locates the sole
concrete `WorldSession::dispatch_packet` by logical mount rather than by filename. External modules,
inline modules, and supported `#[path]` declarations therefore preserve the contract across a
physical split; unresolved paths, production-capable `cfg` ambiguity, remounts, missing or duplicate
dispatchers, registrations outside their declared owner, and unprovable source-generation shapes
fail closed.

`self-test` pins the locked/all-features Cargo metadata command and proves a permitted downward
workspace edge (`wow-combat → wow-math`), a rejected
upward edge (`wow-map → wow-network`), a reviewed domain utility (`wow-map → rand`), concrete SQL
inside its adapter (`wow-database → sqlx`), and rejection of direct SQL, network, configuration,
process, and async-runtime additions to `wow-map`. It also exercises stale workspace exceptions,
external exceptions, external allowlist entries, and the raw-network rejection of direct SQL
(`wow-network → sqlx`) while retaining its reviewed Tokio runtime. Malformed/duplicate JSON and
Cargo identities, canonical/path/Git/alternate-registry origins, both valid Cargo Git-ID forms,
inactive target-specific dependencies, and ambiguous external identities are adversarially
covered. The hotspot classifier self-test proves exact top-level `#[cfg(test)]` item ranges across
multiple modules/items, ignores braces and false attributes in Rust literals/comments, and keeps
production after a test module outside the test range. The ratchet self-test independently rejects
production, test, and total growth and accepts a reduction. `check` evaluates the real locked
Cargo workspace, including direct
third-party `normal`/`build` dependencies, rejects stale policy entries, enforces non-growth for
the curated hotspot paths, and still prints the broader source-hotspot report.

The exact handler snapshot is `tools/architecture/world-handler-contract.tsv`. Its Rust test
enumerates the linked `inventory` registry, so macro-generated registrations are included. A
production integration test enumerates the same registry with the library compiled without
`cfg(test)`, preventing test-only submissions from entering the reviewed contract. The four
`wow-handler` registry API tests are also integration tests so their `Ping` submission is not part
of a production library source.

The standalone Rust tool under `tools/architecture/handler-contract-check` parses source without
compiling `wow-world`. Full locked Cargo metadata with all features enabled supplies the reverse
normal-dependency closure of `wow-handler`; every production `lib`/`bin` module tree in that
workspace closure is audited, including optional, renamed, transitive, and target-specific normal
edges. Development/build-only edges are excluded. An unknown dependency kind or a non-workspace
package in that reverse closure fails closed because its production source cannot be proved by the
workspace audit. Independently, every production module graph in all workspace packages is scanned
for definitions, invocations, aliases, exports, and includes capable of emitting, forwarding, or
mounting hidden registration source. The cross-package surface is a closed baseline: exactly the
23 existing declarative exports in `wow-logging`, the six generated protobuf `include!` bodies in
`wow-proto`, and the seven existing non-handler inventory calls in `wow-script` are pinned by
package and source (exported names exactly; include/inventory bodies exactly). Any new export,
include, inventory call, meta-macro, or macro body that can synthesize `mod`, `macro_rules!`,
`PacketHandlerEntry`, an inventory registration path, or one of the audited registration macros
fails. This prevents an upstream workspace crate outside the reverse closure from exporting or
mounting a hidden generator that is later invoked by a handler-linked crate.

The sibling `session-ownership-check` binary reuses that logical module walk but evaluates `cfg`
satisfiability to classify production, test-fixture, dead, and generated-input surfaces. It stores
exact normalized identities rather than line numbers, follows the three transitional registry
aliases through ordinary wrappers/imports/locals/returns, and fails closed when a macro, alias,
glob, `include!`, or malformed `cfg` would make ownership unknowable. Splitting or combining
physical `impl` blocks is therefore neutral; moving or changing an associated item, field, payload,
factory call, or direct registry operation is a reviewed baseline delta.

Ownership is logical, not a path-prefix convention: only sources mounted as
`wow_world::handlers` or descendants may submit `PacketHandlerEntry`. A file physically below
`src/handlers` but mounted as `crate::shadow` remains outside the owner. The cfg-independent module
walk follows explicit `#[path]` files, requires regular non-symlink `.rs` files inside their package
root, and rejects conditional `cfg_attr(path)`, `#[path]` in/under inline modules, and module
declarations nested in block/item bodies. These closed rules keep the ownership walk and handler
grammar on the same exact source set.

Inside the owner, direct submissions and the six audited one-entry registration macros must be
unconditional, private module items. `#[macro_export]`, reexports, unknown item macros, nested
handler-capable calls, macro-path or metavariable forwarders, aliases of inventory registration
macros, repetitions, and multi-arm generators fail closed. Outside the owner, `include!`,
handler-capable macro definitions/calls, and every `collect`, `submit`, or `__do_submit` inventory
registration-macro path/import/alias are rejected except for the exact non-handler/generated
surfaces pinned above. The only handler-registry exception is exactly one unconditional module-level
`inventory::collect!(PacketHandlerEntry)` at the `wow-handler` production Cargo `lib` target root
(not a file merely named `lib.rs` or a nested conditional module). The checker scans declarative
macro definitions and calls in all workspace production sources, but deliberately does not claim
arbitrary expansion inside third-party crates or external procedural macros; such
source-generation grammar must be made inspectable and added to the guard before use.

The checker also parses the active `dispatch_packet` method and compares its top-level
`ClientOpcodes` patterns with the snapshot; conditional or guarded arms are rejected and the sole
wildcard must be an independent final arm. Registrations and arms must now be exactly equal:
there are no one-sided exceptions, and any future mismatch fails. This is a registration-to-arm
coverage check; it intentionally does not infer correctness from names mentioned in an arm body.
The existing C++ metadata regression remains the semantic authority. Together, these guards
prevent an unreviewed registration addition, removal, rename, metadata change, conditionally
compiled registration, or dispatch-arm loss.

## Deliberate baseline updates

Do not regenerate a baseline merely to make CI green.

1. Identify the semantic owner and contrast any behavior-affecting change with C++.
2. Explain why the dependency or handler-contract change is intentional in the issue and PR.
3. Add or update focused positive/negative tests before changing the baseline.
4. For a legitimate external library in an inward package, add only that exact package/kind to
   its allowlist and explain the architectural role. For workspace or infrastructure-debt
   exceptions, provide a concrete tracking issue and reason. Remove an exception or stale
   allowlist entry in the same commit that removes its final edge.
5. For a handler snapshot or dispatch-side exception, inspect the exact added/removed/changed row
   or arm, retain the C++ metadata contrast test, and assign any temporary one-sided wiring to a
   concrete removal issue.
6. Run the architecture self-test/check, focused evidence, and
   `./tools/validation-v2 final`. Reserve `./tools/validation-v2 audit` for an explicit audit
   or release.

## Which baseline owns what

Ten checked-in files pin architecture facts. Each has exactly one producer and one enforcing
command; none of them is a second copy of another, and none may be regenerated to authorise a new
owner, public API, mirror, dependency, or direct storage operation.

| File | Pins | Enforced by | Regenerated by |
|---|---|---|---|
| `session-ownership-policy.json` | the exact Session syntax surface: WorldSession fields, impl owners and associated items, SessionResources, SessionCommand variants and direct-registry rows | `session-ownership-check check --syntax-only`, plus `check_architecture.py` for runtime syntax coverage | `session-ownership-check print-baseline` |
| `persistence-access-snapshot.json` | every production and test-fixture persistence access, keyed by package, file, enclosing item and fingerprint | `session-ownership-check check` (exhaustive, `audit` only) | `session-ownership-check print-persistence-baseline` |
| `persistence-boundary-policy.json` | how an access is classified and which boundary may hold it | the same exhaustive check | `session-ownership-check print-persistence-policy` |
| `persistence-boundary-workflows.json` | the permitted persistence workflows behind those boundaries | the same exhaustive check | reviewed by hand with the check |
| `runtime-ownership-ledger.json` | runtime ownership rows and the curated hotspot LOC ceilings | `check_architecture.py hotspot-ratchet` in `final`; the whole ledger in `check` | measured with `check_architecture.py hotspots --limit 20` |
| `dependency-policy.json` | allowed crate dependency edges and debt ownership | `check_architecture.py check` | reviewed by hand |
| `handler-module-policy.json` | handler logical-module ownership | `check_architecture.py check` and the handler contract check | reviewed by hand |
| `world-handler-contract.tsv` | the exact world handler contract rows | the handler contract check | its own checker run |
| `runtime-clock-phase-trace.json` | the traced runtime clock phases | `check_architecture.py check` | reviewed by hand |
| `architecture-issue-ledger.json` | issue numbers, kinds, parents, dependencies and the mirrored open/closed state | `check_architecture.py check` and the Session ownership check | structure by hand; `state`/`title` by `check_architecture.py refresh-issue-state` |

### Issue state is derived, not maintained

The guards read mirrored issue state: a dependency exception owned by a completed issue must fail,
as must a persistence workflow whose ownership issue has closed. A stale issue mirror disarms
those checks silently, which is how #258 and then #299 each had to repair the same class of drift by hand.

The mirror is no longer hand-maintained. `check_architecture.py refresh-issue-state` derives every
`state` and `title` from the live repository and rewrites both ledgers; `--check` reports drift and
fails without writing, and the weekly `Issue state drift` workflow runs exactly that. The guards
keep reading the checked-in file.

**Validating a ledger change takes the exhaustive ratchet.** Three separate guards read issue
state, and they do not live in one tool: dependency exceptions are checked in
`check_architecture.py`, while persistence workflow ownership is in the Rust
`session-ownership-check check`, which only runs in `audit`. #299 was validated with the Python
checker alone and left 91 workflow groups targeting a closed issue, which #341 then had to repair
on `3.4.3`. After editing either ledger, run both:

```bash
python3 tools/architecture/check_architecture.py check
cargo run --release --locked \
  --manifest-path tools/architecture/handler-contract-check/Cargo.toml \
  --bin session-ownership-check -- check
```

Deriving beats reading issue state at check time, which was the other option #299 named: `check`
runs inside `audit`, which is offline and hermetic by contract, and a validator that needs GitHub
to answer would make every architecture check depend on network and API availability. The refresh
is the only networked command in this tool, it is never called by `check`, and drift can no longer
survive a week unnoticed.

## Audited ownership and hotspot evidence

The logical-owner baseline was refreshed on branch `3.4.3` at `c2bb8a85`, after the world-server
composition split. Production and test lines remain separate:

| Logical owner root | Production | Tests | Total |
|---|---:|---:|---:|
| `crates/wow-world/src/session/mod.rs` | 73,679 | 94,442 | 168,121 |
| `crates/world-server/src/lib.rs` (crate scope) | 24,985 | 21,491 | 46,476 |
| `crates/wow-map/src/map.rs` | 15,245 | 18,273 | 33,518 |
| `crates/wow-world/src/handlers/character.rs` | 20,200 | 10,691 | 30,891 |
| `crates/wow-world/src/handlers/loot.rs` | 13,744 | 16,081 | 29,825 |
| `crates/wow-world/src/handlers/quest.rs` | 8,255 | 10,172 | 18,427 |
| `crates/wow-entities/src/player.rs` | 9,265 | 8,891 | 18,156 |

The physical table is generated live and gives every source file its own row. At this baseline,
`world-server/src/main.rs` is 15 lines, while `world-server/src/lib.rs` plus its private crate
descendants remain a 46,476-line logical composition owner. That is the intended distinction:
#136 materially improved navigation without claiming that bootstrap/runtime ownership vanished.
Standard adjacent module directories and transitional `#[path]` descendants remain charged to
their logical root; `logical_scope: crate` is used only for an explicitly reviewed composition
root. Extracted `*_tests.rs` and `tests/` descendants remain test lines in the logical view.
Issue #152 likewise keeps the complete Session owner charged to `session/mod.rs` while exposing
admission (462 lines), dispatch (1,739 lines), and their focused test modules as real physical
files. The 50-line logical increase is the reviewed module/header and explicit-import overhead of
that split, not new runtime behavior.
Issue #139 retires the former 18,637-line `handlers/misc.rs` hotspot instead of renaming it as one
logical monolith: its registrations, behavior, and tests now live in private capability modules
under `handlers/misc/`, while `misc/mod.rs` is a 164-line compatibility facade for shared packet
types, constants, and narrow helpers. Every production capability file is below 700 lines.

Issue #231 closes the module lane with namespaced typed configuration. Package defaults live in
`module.toml`, operator overrides in `conf/modules/<id>.toml` outside the checkout, and `sync`
merges, validates and embeds the typed result so a module reads its configuration once at
registration and no callback touches a file. Keys are namespaced by validated `ModuleId`, an
unread key fails registration as an operator typo rather than being ignored, and a module that
refuses its configuration is never registered — so an invalid value is caught at startup, not at a
player's login. Each configuration has a deterministic digest recorded in the lock and reported by
`list`/`doctor`, computed identically in Rust and Python with both sides pinning the same literals.
`source_api` incompatibility is refused with an actionable message before anything compiles, with a
dedicated fixture.

Issue #230 adds the author/operator workflow on top: `tools/modules/rustycore-module` with
`new`, `install`, `update`, `remove`, `list`, `sync`, `check`, `build`, `test` and `doctor`. Every
command is non-interactive, `--json` emits pure JSON on stdout so an agent parses it without
stripping prose, and the five exit codes are documented and covered by tests. Only `install` and
`update` reach the network, neither ever executes a script from the fetched repository, a rejected
install leaves nothing behind, `update` refuses a dirty checkout instead of discarding work, and
`remove` refuses any path escaping `modules/`. The official skeleton produces a module that
compiles and tests as-is, including a focused hook test.

Issue #229 makes that API installable. `modules/<checkout>/module.toml` describes an
independent trusted repository, `tools/modules/compose.py sync` validates every checkout and
regenerates both `modules.lock.toml` and the `world-modules` compositor crate, and `check` fails
when the tree has drifted. Generation is an explicit operator step: the build never fetches and no
`build.rs` discovers or rewrites the source tree. Composition order is the operator's declared
order then module id, never registration order and never linker inventory. The compositor refuses,
before compiling, a malformed id/version/package/path/registrar, a `crate_path` escaping its
checkout, a duplicate id or Cargo package, and an unsupported `source_api`. The zero-module build
stays exactly as it was: `world-server`'s binary still calls `run`, and the generated compositor
with nothing installed passes an empty registry to `run_with_modules`.

Issue #228 opens the module lane by earning a public source API rather than declaring one.
`wow-module-api` exists only to carry the `player.login -> SendSystemMessageSelf` vertical anchored
to C++ `ScriptMgr::OnPlayerLogin`, and it is classified `foundation` with an empty external
allowlist, so the checker rejects any future edge to a runtime, transport, storage or protocol
crate. A module receives an immutable snapshot and returns typed effects; it never sees a
`WorldSession`, `Player`, `Map`, pool, packet writer or raw pointer, and a manifest test asserts
that rather than leaving it to review. Dispatch is ordered by `ModuleId`, not registration order,
so the same linked set always produces the same sequence; the batch is validated whole, so one bad
effect discards it instead of half-applying; a rejected batch is logged and the login continues,
because a module must not be able to fail a player's login; and with no registry installed the
hook returns immediately, which is the zero-module no-op path.

Issue #189 retires the first field of the temporary gameplay mirror. The durable loot-money
coordinator left the Session mailbox for its own `wow_world::loot_persistence` owner — fencing
durable money persistence against admission, logout and unknown-commit outcomes is a persistence
concern, not a mailbox one — and its five focused fixtures moved with it. More importantly the
tracker is no longer a `PlayerBroadcastInfo` field: it lives beside the entry in the directory's
private storage, because it is not a gameplay projection another session may read but the
persistence handle the owning session already holds, resolved by GUID only so a remote looter can
address the recipient's coordinator. That creates no second store and no second authority, and it
takes the mirror from **80 fields to 79** — the first reduction since the ledger froze it.
`register_or_replace` now names the handle explicitly instead of smuggling it through the
projection, which cost 392 mechanical call-site updates. Transaction boundaries, SQL,
unknown-commit handling, fence and publication order are untouched.

Issue #227 finishes the physical lane on the protocol and static-data hotspots.
`wow-packet/src/packets/misc.rs` (13,185 lines) becomes `misc/` split by protocol family —
`character`, `social`, `spell`, `combat`, `movement`, `world_state`, `session` — with its
4,977-line inline test module extracted; `packets/update.rs` (9,735) becomes `update/` split by
entity domain — `player`, `unit`, `game_object`, `item`, `movement`, `block`; and
`wow-data/src/spell.rs` (7,559) becomes `spell/` with `stores`, `catalog`, `acquisition` and
`corrections`. Every public type and byte contract is unchanged, which the 724 packet and 711
data tests confirm.

Two lessons the guards taught here are worth recording. Trait-implementation methods cannot carry
a visibility qualifier, so the cross-module promotions had to distinguish inherent impls from
trait impls rather than rewrite every `fn`. And a module named `inventory` shadows the `inventory`
crate that carries handler registrations: `handler-contract-check` rejects it by name. #226 had
introduced exactly that shadow as `player/inventory`, and its harness run did not reach the
handler-contract stage; #227 renames it to `player/items`, matching what #224 had already chosen
for the character family for the same reason.

Issue #226 does the same for the canonical entities. `wow-entities/src/player.rs` (9,268 lines)
becomes `player/` with `identity`, `location`, `vitals`, `powers`-in-`vitals`, `progression`,
`collections`, `social`, `spellbook`, `visibility` and an `inventory/` subtree split into
`storage`, `equipment` and `enchantment`, because inventory alone was 4,253 lines. The former
7,071-line `unit_subsystems.rs` becomes `unit_subsystems/` with `aura`, `spell`, `combat`,
`threat`, `movement` and `control`, plus its 2,249-line inline test module extracted to
`tests.rs`. `Player` and `Unit` remain single types with single semantic owners: no storage
location, writer, mirror or runtime clock changed.

The suggested target shape named the subsystem directory `unit/`, which would have required
folding the separate 4,178-line `unit.rs` entity into it. That merge produced duplicate imports
and privately re-exported subsystem types, so the directory is `unit_subsystems/` and `unit.rs`
keeps its own module — a naming deviation, not a scope reduction, and one that leaves the Unit
entity untouched.

Issue #225 gives the two map runtimes the same treatment. `wow-map/src/map.rs` (15,250 lines)
becomes `map/` with eight private modules — `update`, `storage`, `visibility`, `spawn_groups`,
`respawn`, `relocation`, `game_object`, `scripts_weather` — and `wow-world/src/map_manager.rs`
(6,607) becomes `map_manager/` with `runtime`, `movement`, `combat` and `respawn`. No `fanout`
child was created: its 56 lines folded into `runtime` rather than becoming a near-empty file. The
Map ceiling rises 135 lines for the module headers and imports of twelve new files, and the
`impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>` header is repeated per child because Rust has
no way to continue a generic impl across modules. 32 methods became `pub(super)`. Nothing about
the two documented runtime models changed: no clock, writer, phase, bridge or scheduling was
touched, which is why this slice never needed #188.

Issue #224 turns the last three handler monoliths into feature trees without moving their logical
owner: `handlers/character.rs` (20,272 lines), `handlers/loot.rs` (13,606) and
`handlers/quest.rs` (8,274) become `character/` with eleven modules, `loot/` with ten and `quest/`
with seven. Each packet entry point is filed under the feature it serves rather than pooled into
one handler dump, which is what keeps every productive child under the 4,000-line review signal —
the largest is `character/items.rs` at 3,735. The three logical ceilings rise by 75, 0 and 134
lines respectively: module headers, docs and imports across 28 new files, with `loot/mod.rs`
actually shrinking. 178 methods crossed a feature boundary and became `pub(super)`, so the widened
visibility stops at its family instead of reaching the crate, and the handler-contract snapshot is
byte-identical because the registrations moved into descendants of the same declared owner root.
Two mechanical details were forced by the split: the character feature module is `items`, since
`mod inventory` would shadow the `inventory` crate its registrations use, and the money-publication
source scan in `character_tests.rs` now concatenates the eleven modules instead of reading one
file.

The persistence inventory still names the pre-split paths. Refreshing it needs
`print-persistence-baseline`, which aborts on this workspace — on the parent commit as much as on
the split — so the snapshot, its derived policy and the reviewed workflow annotations were left
mutually consistent rather than half-migrated. That blocked refresh is tracked with the other
unrunnable-tooling findings in #263.

Issue #140 completes the same campaign for the Session mailbox and, unlike the two moves before
it, is a net dependency reduction rather than a relocation: `wow-network` loses `wow-data`,
`wow-loot` and `wow-social` outright, so the workspace falls from 109 to 106 edges and the baseline
exceptions from 20 to 17 — including the transitional `wow-network -> wow-social` edge #137 had
just declared. Raw networking now depends only on transport crates. The 1,349 production and 390
test lines the Session logical ceiling gains are the mailbox arriving as `session/mailbox/`; the
`WorldSession` surface itself is unchanged at 738 fields and 3,341 associated items, with no
method added or removed by the move. The one real surface change is that 29 command handlers in
`handlers/loot.rs` become `pub(crate)`: the pump that calls them now lives in a sibling module, and
Rust has no narrower visibility that reaches it. They stay crate-internal and unexported.

Issue #137 gives the relocated Group owner its own logical ceiling instead of letting it land
untracked: `crates/wow-social/src/group/mod.rs` enters the table at 2,783 production and 2,511 test
lines. `wow-network` loses the whole 5,114-line `group_registry.rs`, a reduction the curated table
cannot credit because raw networking was never a curated owner root; the physical view shows it
directly. The paired increases are the mechanical cost of the move: 3 production and 2 test import
lines in `session/mod.rs`, 1 test import line in `handlers/loot.rs`, and 28 production lines in
`world-server/src/lib.rs` — 23 of them the explicit `GroupDifficultyStorePortLikeCpp` adapter that
binds `wow_social`'s loaded-difficulty port to the concrete `wow_data::DifficultyStore`, because a
`domain-runtime` owner may not depend on an `adapter-platform` store and neither type is local to
the composition root. The exact direct-registry inventory falls from 572 to 570 rows: 17 rows
relocate one-for-one and the two obsolete `wow-network` `GroupRegistry`/`PendingInvites`
re-exports disappear.

Issue #138 raises the same logical Session ceiling by exactly the relocated directory: 1,773
production and 134 test lines arrive as `session/directory.rs`, which is a reviewed private
descendant of the `session/mod.rs` owner root. Those lines are not new behavior — they are the
already-opaque connected-session directory moved verbatim out of `wow_network::player_registry`,
which stops owning it. The paired `handlers/loot.rs` (+1 production) and `handlers/quest.rs` (+1
test) increases are the explicit `use crate::session::directory::…` import lines the move
requires. The physical view records `session/directory.rs` as its own file, so the split is
visible even though the logical owner still carries it; the terminal `wow-session::directory`
crate move stays open, as do the mailbox pump (#140) and the `PlayerBroadcastInfo` retirement
(#252). No public API was added: the crate-root `wow_network::PlayerRegistry` re-export was
removed, and the exact direct-registry inventory therefore falls from 573 to 572 rows.

At the same HEAD, the syntax-aware ratchet records 731 `WorldSession` fields: 719 production and
12 `cfg(test)` fixtures. It also records all 46 logical impl owners and 3,284 exact
associated-item signatures rather than freezing the number of physical `impl` blocks. Issue #410
retires the rest-state statement builder and replaces the statement-shaped XP plan with one semantic
request, a net reduction of one signature. Private composition-side `SessionResources` has 248 fields,
of which 191 are optional;
`PlayerBroadcastInfo` is retired; and `SessionCommand` has 38 variants plus 45 transitively
reachable payload types. The factory has 252 `set_*` and one `install_*` call. The generated-input surface has 47 exact
records, and direct access to `PlayerRegistry`, `GroupRegistry`, or `PendingInvites` is frozen as
589 exact AST rows. After #410 moved the represented XP/rest transaction behind the lifecycle port,
the workspace-wide persistence inventory contains 21,655 exact rows—11,546 production and 10,109
test-fixture—with multiplicity 23,948 (13,112 production and 10,836 test). The reviewed delta
removes 35 production concrete-access rows (multiplicity 39) from the three `wow-world` XP/rest
workflow groups; the existing stable `wow-database` adapter gains 38 production rows (multiplicity
47) for statement construction and classified transaction execution, while SQL-shaped Session tests
are replaced by semantic-request and port-outcome coverage. Its workflow annotations fall from 860
to 859 (864 exact semantic groups in the exhaustive ratchet), while workflows targeting #169 fall
from 19 to 17. Six generated-source
inputs are an orthogonal subset, not a third source class. Schema
v3 covers SQLx and concrete `wow_database` types/imports, typed statements/results/errors,
prepare/query/execute/direct/raw/nonliteral/interpolated SQL, pool access, transaction construction/append/commit,
database opening, advisory locks, value flow and escapes. Statement text is read only where it is
pinned—a literal, a `concat!`, or a name bound to one of those. SQL assembled at run time (`+`
chains, `format!` templates, branches, helper returns, projections) is deliberately recorded as
interpolated or nonliteral without a content claim: deciding which string an expression produces
has no natural stopping point, so the connection-affinity and ordering facts for those call sites
come from the reviewed workflow annotation covering them. The 868 semantic workflow groups classify every
row exactly once by logical database,
capability owner, connection/transaction affinity, current order, failure/unknown-commit behavior
and open removal/decision issue; unmatched, overlapping or stale groups fail. The legacy/canonical
inventory contains 71 definition/seam rows, including eight
curated anchors; it deliberately avoids duplicating every caller of an already inventoried typed
helper. `#134` already moved `SessionResources` out of `wow-network`; #136 extracts the factory
without turning the aggregate into another public dependency bag.

The reproducible Session syntax snapshot is
`tools/architecture/session-ownership-policy.json`; persistence identities live in
`tools/architecture/persistence-access-snapshot.json`, reviewed workflow annotations in
`tools/architecture/persistence-boundary-workflows.json`, and their canonically derived policy in
`tools/architecture/persistence-boundary-policy.json`; the remaining curated owner/writer/mirror
and retirement mapping is `tools/architecture/runtime-ownership-ledger.json`. The Python guard cross-checks their
exact field/variant memberships, while `session-ownership-check` rejects added, removed, retyped,
re-visibility-scoped, re-owned, generated, factory-wiring, command-payload, broadcast, and direct
registry, persistence, and bridge surfaces. `print-baseline` only writes reviewed JSON to stdout;
`print-persistence-baseline` independently reproduces the dedicated persistence snapshot and
`print-persistence-policy` derives the policy from the reviewed workflow annotations. Because the
policy is a pure function of those annotations and the exact inventory,
`print-persistence-policy --from-snapshot PATH` derives it from an already computed snapshot
instead of scanning the workspace again: CI publishes both files as the `persistence-access-snapshot`
artifact when the ratchet moves, and a repository test rejects a checked-in snapshot and policy that
disagree, so the pair can never be updated by halves. None of these
commands updates a checked-in artifact automatically.

Workflow annotation schema v2 is the reviewed source of truth for each workflow's logical
databases, capability boundary, connection affinity, current order, and failure/unknown-commit
behavior. Policy generation copies those semantic fields exactly; it does not synthesize them
from transaction-shaped syntax.

These numbers are diagnostics, not completion criteria. #181 owns reproducible counters and
non-growth rules; each later slice must reduce or retire a named ownership smell rather than
merely moving lines.

## Refactor sequence

The ledger is the offline dependency map, not a serial execution queue. Epics #133, #169 and #99
are parents rather than PRs; external QA prerequisite #177 is separate. The four active lanes are:

- physical modules: #139, #152 and #224–#227;
- encapsulation/ownership: #150–#153, #137–#140 and #182–#184;
- runtime/persistence authority: #188–#200 plus open follow-up #204;
- stable crate/extension seams: #228–#231 under #99.

#153 is the terminal audit, not a gate that postpones known work. This deterministic topological
display is checked against the JSON ledger:

1. #131 — repository-scoped architecture skills;
2. #135 — executable boundary guardrails;
3. #143 — interaction provenance;
4. #146 — effective SpellInfo authority;
5. #148 — effective SkillLine authority;
6. #144 — validated trainer load inputs;
7. #156 — primary-profession capacity;
8. #163 — effective spell-acquisition metadata;
9. #164 — deterministic acquisition outcomes;
10. #157 — trainer offer and eligibility decisions;
11. #158 — durable prepared spell learning;
12. #159 — atomic normal trainer teaching;
13. #160 — durable account-atomic battle-pet ownership;
14. #161 — recoverable battle-pet trainer purchase;
15. #142 — dispatcher/registration reconciliation;
16. #154 — audited policy alignment;
17. #134 — listener/SessionResources decoupling;
18. #181 — ownership baseline and submodule ratchets;
19. #185 — module-aware architecture and handler guards;
20. #186 — persistence-leak inventory and ratchet;
21. #204 — active-trait `Self` constant resolution;
22. #205 — cfg-aware associated constants;
23. #206 — faster architecture gate;
24. #208 — remove gate dead weight;
25. #210 — remove persistence scan from the PR path;
26. #213 — persistence-trace gap closure;
27. #214 — root test-module extraction;
28. #218 — workflow-scoped persistence traces;
29. #220 — transitional child-charging hotspot metric;
30. #223 — roadmap, metrics and local-first synchronization;
31. #188 — world/map clock and bridge trace;
32. #136 — world-server composition split;
33. #187 — Player lifecycle persistence-order golden;
34. #150 — opaque PlayerRegistry facade;
35. #151 — opaque GroupRegistry/PendingInvites facades;
36. #139 — private misc capabilities;
37. #152 — Session admission/dispatch modules;
38. #200 — Player lifecycle persistence port;
39. #286 — represented Player character-save plan through that SQLx-free port;
40. #384 — retire obsolete Session character-save builders after the port cut;
41. #386 — account collection loads through the SQLx-free Player lifecycle port;
42. #388 — session account data and tutorial reads/writes through a dedicated SQLx-free port;
43. #390 — auxiliary Player-login reads through the SQLx-free lifecycle port;
44. #392 — canonical-map corpse hydration through a dedicated SQLx-free port;
45. #394 — Player homebind persistence through the SQLx-free lifecycle port;
46. #396 — Player spell-history reads through the SQLx-free lifecycle port;
47. #398 — Player trait-entry/config reads through the SQLx-free lifecycle port;
48. #400 — logout buyback cleanup through the SQLx-free lifecycle port;
49. #402 — realm character-count refresh through the SQLx-free lifecycle port;
50. #404 — initial world-state login reads through the SQLx-free lifecycle port;
51. #406 — transport login reads through the SQLx-free lifecycle port;
52. #408 — talent-reset transaction through the SQLx-free lifecycle port;
53. #410 — represented XP/rest persistence through the SQLx-free lifecycle port;
54. #412 — raw database-updater pools retained inside the concrete adapter;
55. #414 — core Player-login character row through the SQLx-free lifecycle port;
56. #416 — Player-login location and guild reads through the SQLx-free lifecycle port;
57. #418 — Player-login pet reads through the SQLx-free lifecycle port;
58. #420 — Player-login group lookup through the SQLx-free lifecycle port;
59. #422 — Player-login profile reads through the SQLx-free lifecycle port;
60. #424 — Player-login progression reads through the SQLx-free lifecycle port;
61. #426 — Player-login aura reads through the SQLx-free lifecycle port;
62. #428 — Player-login inventory reads through the SQLx-free lifecycle port;
63. #430 — Player-login item repair writes through the SQLx-free lifecycle port;
64. #432 — remaining Player-login pet-reset and online writes through the SQLx-free lifecycle port;
65. #434 — PacketSpoof ban persistence through a SQLx-free admission port;
66. #436 — void-storage unlock/swap through a SQLx-free capability;
67. #438 — complete mixed void-storage transfer through the SQLx-free capability;
68. #440 — social-list reads and mutations through a SQLx-free capability;
69. #442 — retire obsolete Session void-storage statement builders;
70. #444 — direct player-money and durability writes through the lifecycle port;
71. #446 — standalone player-currency saves through the lifecycle port;
72. #448 — party-invite social reads through the SQLx-free social port;
73. #450 — quest POI reads through the SQLx-free World-data port;
74. #452 — stored Item loot-money SQLx through a typed persistence port;
75. #454 — group loot-money SQLx through a distinct typed persistence port;
76. #456 — standalone no-cost item-durability write through the lifecycle port;
77. #458 — legacy bug-report insert through a typed support port;
78. #460 — next-mail query through a typed mail-read port;
79. #462 — gameobject-use template query through a typed World-read port;
80. #464 — bank-slot purchase through the SQLx-free Player lifecycle port;
81. #466 — uncaged battle-pet item-state reads through the Player lifecycle port;
82. #468 — represented Group persistence intents through a typed persistence port;
83. #470 — represented Group startup cleanup and loads through a typed persistence port;
84. #472 — represented Player spell-acquisition persistence through a typed port;
85. #474 — battle-pet account durability through a typed persistence port;
86. #476 — battle-pet trainer-purchase saga durability through a typed port;
87. #478 — character-enumeration reads through a typed persistence port;
88. #480 — creature-query catalog reads through a typed World persistence port;
89. #482 — gameobject-query catalog reads through a typed World persistence port;
90. #484 — page-text catalog reads through a typed World persistence port;
91. #487 — player-name reads through a typed Character persistence port;
92. #489 — item-template-addon reads through a typed World persistence port;
93. #491 — gossip catalog reads through a typed World persistence port;
94. #493 — respawn loads and mutations through a typed Character persistence port;
95. #495 — represented game-event loads and mutations through a typed Character persistence port;
96. #497 — instance-lock loads and mutations through a typed Character persistence port;
97. #499 — PlayerChoice startup loading through a typed World persistence port;
98. #501 — mount DB2 overlays and faction definitions through a typed startup port;
99. #503 — reputation startup tables through a typed World persistence port;
100. #505 — AreaTrigger template/create-properties tables through a typed World persistence port;
101. #507 — Vehicle/VehicleSeat hotfix overlays and World template/accessory tables through separate typed startup ports;
102. #509 — core SpellInfo DB2 overlays through a typed Hotfix persistence port;
103. #511 — exact SpellInfo key overlays through a typed Hotfix persistence port;
104. #513 — remaining standalone Spell DB2 overlays through the typed Hotfix persistence port;
105. #515 — foundational SpellMgr World catalogs through a typed persistence port;
106. #517 — remaining complex SpellMgr World catalogs through the same typed persistence port;
107. #519 — trainer startup catalogs through a typed World persistence port;
108. #521 — ChrSpecialization official/custom Hotfix overlays through a typed persistence port;
109. #523 — the effective SkillLine/ability/race-class catalog through one staged Hotfix source;
110. #526 — fishing-base and skill-tier World rules through one typed skill capability;
111. #528 — Player race and class-level base stats through one typed World capability;
112. #530 — represented Player-creation base/cast/custom catalogs through one staged World capability;
113. #532 — effective Difficulty official/custom Hotfix overlays through one typed capability;
114. #534 — Hotfix blob/data/optional delivery metadata through one staged typed capability;
115. #536 — remaining AreaTrigger World reads through one staged typed capability;
116. #538 — effective DB2 removals reuse the typed HotfixData source without merging startup stages;
117. #540 — creature display/model DB2 overlays through one two-stage typed Hotfix capability;
118. #542 — gossip startup menus/options/locales/addons through one four-stage typed World capability;
119. #544 — Phase and PhaseXPhaseGroup DB2 overlays through one ordered typed Hotfix capability;
120. #546 — World phase areas/names and terrain-swap tables through one four-stage typed capability;
103. #189 — durable loot persistence coordination;
96. #192 — runtime/fanout directory consumers;
97. #193 — combat/loot directory consumers;
98. #194 — quest/spell/movement directory consumers;
99. #197 — atomic group invite/create transitions;
100. #198 — atomic group membership/leadership transitions;
101. #199 — Group persistence/publication closure;
102. #195 — social/group session addressing;
103. #196 — PlayerRegistry storage closure;
104. #138 — opaque session-directory relocation;
105. #191 — mailbox protocol relocation;
106. #137 — encapsulated Group owner move;
107. #190 — durable creature-runtime rail relocation;
108. #140 — Session mailbox pump;
109. #252 — retire the temporary PlayerBroadcastInfo gameplay mirror;
108. #182 — logical realm/instance routing;
109. #183 — Session-only phase driver;
110. #184 — login/logout lifecycle modules;
111. #224 — character/loot/quest physical modules;
112. #225 — Map/MapManager physical modules;
113. #226 — Player/Unit physical modules;
114. #227 — packet/spell-data physical modules;
115. #228 — trusted linked external module API;
116. #229 — deterministic external Cargo composition;
117. #230 — agent-neutral module CLI and skeleton;
118. #231 — typed module configuration/fixtures;
119. #270 — retire the four PlayerBroadcastInfo transport endpoints;
120. #359 — single dispatch mechanism for every opcode;
121. #297 — promote the Session kernel to `wow-session`;
122. #378 — move the remaining five session modules into `wow-session`;
123. #153 — terminal architecture audit.

A slice may start once its declared prerequisites are merged and its branch is current. Independent
physical work remains parallel to semantic authority cuts. Mechanical moves use focused compile and
contract evidence; they do not acquire gameplay-parity claims merely by reducing a file.
