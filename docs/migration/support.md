# Migration: Support (SupportMgr — bugs, complaints, suggestions)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Support/`
> **Rust target crate(s):** would live in a new `crates/wow-support/` (or fold into `wow-social`); GM-command surface eventually in `wow-handler`
> **Layer:** L8 (service — opt-in feature on top of database/world/chat)
> **Status:** ❌ not started — confirmed via audit 2026-05-01 (one decoded opcode routed to a no-op `{}`; `ComplaintsEnabled = false` hardcoded; zero DB statements; zero ticket schema)
> **Audited vs C++:** ⚠️ partial (header fully read, `.cpp` ~830 lines partially read) — Rust-side absence reverified 2026-05-01
> **Last updated:** 2026-05-01

---

## 1. Purpose

The in-game player feedback system. Three ticket kinds:

- **Bug** — "this quest is broken" reports.
- **Complaint** — player-against-player report (chat spam, cheating, inappropriate name, etc.). Carries a structured `ReportType`/`ReportMajorCategory`/`ReportMinorCategory` + optional chat-log evidence.
- **Suggestion** — feature-request notes.

`SupportMgr` is a singleton that loads all open and closed tickets at startup, generates fresh ids, holds them in three `map<uint32, Ticket*>`s, persists changes to `character.gm_bug` / `gm_complaint` / `gm_suggestion`, and exposes them to GM commands and the client opcodes (`SUPPORT_TICKET_SUBMIT_*`, `GM_TICKET_GET_SYSTEM_STATUS`, `GM_TICKET_GET_CASE_STATUS`). The "GM ticket" classic system (`GMTICKET_*` opcodes) is the legacy interface; the modern Cataclysm-onwards "Submit Bug/Complaint/Suggestion" UI uses the same backing store.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Support/SupportMgr.cpp` | 808 | `prefix` |
| `game/Support/SupportMgr.h` | 312 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Support/SupportMgr.h` | 313 | All enums + `Ticket` base + `BugTicket` / `ComplaintTicket` / `SuggestionTicket` + `SupportMgr` singleton |
| `src/server/game/Support/SupportMgr.cpp` | 832 | All implementations — load/save/delete per ticket type, `Initialize`, `AddTicket`/`CloseTicket`/`RemoveTicket`/`ResetTickets` (templated on T), display formatters |

The opcode dispatch (`HandleSupportTicketSubmitBug`, `HandleSupportTicketSubmitComplaint`, `HandleSupportTicketSubmitSuggestion`, `HandleGMTicket*`) lives in `Server/WorldSession.cpp` and `Handlers/MiscHandler.cpp` — outside this folder but tightly coupled.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SupportMgr` | class (singleton) | Registry + persistence + system-status flags |
| `Ticket` | abstract class | Base: `_id`, `_playerGuid`, `_mapId`, `_pos`, `_createTime`, `_closedBy`, `_assignedTo`, `_comment`; pure virtual `LoadFromDB` / `SaveToDB` / `DeleteFromDB` / `FormatViewMessageString` |
| `BugTicket` | concrete | Adds `_note: string` |
| `ComplaintTicket` | concrete | Adds `_targetCharacterGuid`, `_reportType: ReportType`, `_majorCategory`, `_minorCategoryFlags`, `_chatLog: ChatLog`, `_note` |
| `SuggestionTicket` | concrete | Adds `_note` |
| `ReportType` | enum class : i32 | `Chat=0`, `InWorld=1`, `ClubFinderPosting=2`, `ClubFinderApplicant=3`, `GroupFinderPosting=4`, `GroupFinderApplicant=5`, `ClubMember=6`, `GroupMember=7`, `Friend=8`, `Pet=9`, `BattlePet=10`, `Calendar=11`, `Mail=12`, `PvP=13` |
| `ReportMajorCategory` | enum class : i32 | `InappropriateCommunication=0`, `GameplaySabotage=1`, `Cheating=2`, `InappropriateName=3` |
| `ReportMinorCategory` | enum class : i32 (bitmask) | `TextChat=0x1` … `Name=0x4000` (15 flags) |
| `GMTicketSystemStatus` | enum | `DISABLED=0`, `ENABLED=1` (wire value) |
| `SupportSpamType` | enum | `MAIL=0`, `CHAT=1`, `CALENDAR=2` |
| `ChatLog` typedef | = `WorldPackets::Ticket::SupportTicketChatLog` | Rich struct: lines + reported player IDs |
| `BugTicketList` / `ComplaintTicketList` / `SuggestionTicketList` | typedefs | `map<uint32, T*>` |

Note: WoLK 3.4.3 has only the *classic* GM-ticket system. The modern bug/complaint/suggestion UI in this header was back-ported from later expansions (the report categories reference Club Finder / BattlePet / Friend reporting which don't exist in 3.4.3 client). For a pure 3.4.3 server, only `Chat`, `InWorld`, `Mail`, `PvP`, and the four major categories matter — but the schema is the upstream Trinity one and the back-port is harmless.

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `SupportMgr::Initialize()` | Read `CONFIG_SUPPORT_*` config flags into the four system-status booleans | `World::getBoolConfig` |
| `SupportMgr::LoadBugTickets()` | `CHAR_SEL_GM_BUGS`; per row `BugTicket::LoadFromDB`; track `_lastBugId` and `_openBugTicketCount` | DB |
| `SupportMgr::LoadComplaintTickets()` | `CHAR_SEL_GM_COMPLAINTS`; per row `ComplaintTicket::LoadFromDB`; for each ticket, `CHAR_SEL_GM_COMPLAINT_CHATLINES` and `LoadChatLineFromDB` per line | DB |
| `SupportMgr::LoadSuggestionTickets()` | `CHAR_SEL_GM_SUGGESTIONS`; analogous | DB |
| `SupportMgr::AddTicket(BugTicket*)` / `AddTicket(ComplaintTicket*)` / `AddTicket(SuggestionTicket*)` | Insert into the right map; bump open-count; call `SaveToDB`; `UpdateLastChange` | per-ticket `SaveToDB` |
| `SupportMgr::RemoveTicket<T>(id)` | Delete from map + DB; decrement open-count if was open | `T::DeleteFromDB` |
| `SupportMgr::CloseTicket<T>(id, closedBy)` | Set `_closedBy`; persist; decrement open-count | `T::SaveToDB` |
| `SupportMgr::ResetTickets<T>()` | Wipe in-memory + DB for that type | — |
| `SupportMgr::GetTicket<T>(id)` | Typed lookup | — |
| `SupportMgr::GetComplaintsByPlayerGuid(guid)` | Linear scan filtering by author | — |
| `SupportMgr::ShowList<T>(handler, onlineOnly?)` / `ShowClosedList<T>` | GM-command formatting helper | per-ticket `FormatViewMessageString` |
| `SupportMgr::Generate*Id()` | `++_lastBugId` / `_lastComplaintId` / `_lastSuggestionId` | — |
| `SupportMgr::GetSupportSystemStatus()` / `GetTicketSystemStatus()` / `GetBugSystemStatus()` / `GetComplaintSystemStatus()` / `GetSuggestionSystemStatus()` | Master + per-feature flags AND'd | — |
| `Ticket::TeleportTo(player)` | GM command convenience: tp the GM to where the ticket was filed | — |
| `Ticket::FormatViewMessageString(handler, detailed)` | Produce the `.ticket` chat output | — |
| `Ticket::SaveToDB() / LoadFromDB(fields) / DeleteFromDB()` (per subclass) | `INSERT … ON DUPLICATE KEY UPDATE` for save; field unpacking for load; `DELETE` for delete | DB |
| `ComplaintTicket::LoadChatLineFromDB(fields)` | Append one line to `_chatLog` | — |

---

## 5. Module dependencies

**Depends on:**
- `Database` — extensive: prepared statements for all three ticket types (CRUD + chat-log child rows).
- `Entities/Object/ObjectGuid` — `_playerGuid`, `_assignedTo`, `_closedBy`, `_targetCharacterGuid`.
- `Entities/Player::TeleportTo` — for `Ticket::TeleportTo`.
- `Server/Packets/TicketPackets` — `SupportTicketChatLog` (wire-format struct reused as in-memory).
- `Chat/ChatHandler` — `FormatViewMessageString` consumers.
- `Server/World` — config flags via `CONFIG_SUPPORT_*`, `CONFIG_ALLOW_TICKETS`.
- `wow-core/Position` — `_mapId`, `_pos` for ticket location.

**Depended on by:**
- `Server/WorldSession.cpp` opcode handlers: `SupportTicketSubmitBug/Complaint/Suggestion`, `GMTicketGetSystemStatus`, `GMTicketGetCaseStatus`, `GMTicketAcknowledgeSurvey`, `Complaint`.
- `Chat/Commands/cs_ticket.cpp` (et al.) — GM commands `.ticket`, `.bug`, `.complaint`, `.suggestion` family.

---

## 6. SQL / DB queries (if any)

Prepared statements registered in `CharacterDatabasePreparedStatements`:

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_SEL_GM_BUGS` | Load all bug tickets at startup | character |
| `CHAR_SEL_GM_COMPLAINTS` | Load all complaint headers | character |
| `CHAR_SEL_GM_COMPLAINT_CHATLINES` | Load chat-log child rows for a complaint | character |
| `CHAR_SEL_GM_SUGGESTIONS` | Load all suggestion tickets | character |
| `CHAR_REP_GM_BUG` | Insert/replace bug | character |
| `CHAR_DEL_GM_BUG` | Delete bug | character |
| `CHAR_REP_GM_COMPLAINT` | Insert/replace complaint header | character |
| `CHAR_INS_GM_COMPLAINT_CHATLINE` | Insert chat-log line | character |
| `CHAR_DEL_GM_COMPLAINT` | Delete complaint header (chat lines cascade) | character |
| `CHAR_DEL_GM_COMPLAINT_CHATLOG` | Delete chat-log lines (alternative path) | character |
| `CHAR_REP_GM_SUGGESTION` | Insert/replace suggestion | character |
| `CHAR_DEL_GM_SUGGESTION` | Delete suggestion | character |

Tables: `gm_bug`, `gm_complaint`, `gm_complaint_chatlog`, `gm_suggestion` (all in `character`).

No DBC/DB2 stores used.

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_SUPPORT_TICKET_SUBMIT_COMPLAINT` | client → server | Creates `ComplaintTicket`; `AddTicket` |
| `CMSG_SUPPORT_TICKET_SUBMIT_BUG` (named `Bug`/`SubmitBug` in different builds) | client → server | Creates `BugTicket` |
| `CMSG_SUPPORT_TICKET_SUBMIT_SUGGESTION` | client → server | Creates `SuggestionTicket` |
| `CMSG_GM_TICKET_GET_SYSTEM_STATUS` | client → server | Replies with `SMSG_GM_TICKET_SYSTEM_STATUS` (ENABLED/DISABLED) |
| `CMSG_GM_TICKET_GET_CASE_STATUS` | client → server | Replies with `SMSG_GM_TICKET_CASE_STATUS` (open ticket count for player) |
| `CMSG_GM_TICKET_ACKNOWLEDGE_SURVEY` | client → server | Marks survey as acknowledged |
| `CMSG_COMPLAINT` | client → server | Spam-report shortcut (`SupportSpamType`); fast-path that does not create a `ComplaintTicket` |
| `SMSG_GM_TICKET_SYSTEM_STATUS` | server → client | One u32 = enabled-status |
| `SMSG_GM_TICKET_CASE_STATUS` | server → client | Open-tickets array |
| `SMSG_COMPLAINT_RESULT` | server → client | Echoes `Complaint` result |

The Rust opcode constants for these already exist in `crates/wow-constants/src/opcodes.rs` (e.g. `Complaint = 0x366e`, `SupportTicketSubmitComplaint = 0x3647`, `GmTicketGetCaseStatus = 0x368f`, `GmTicketGetSystemStatus = 0x368e`, `GmTicketAcknowledgeSurvey = 0x3690`, `ComplaintResult = 0x26ab`, `GmTicketCaseStatus = 0x26a3`, `GmTicketSystemStatus = 0x26a2`).

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-support` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-social` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `crates/wow-database/src/statements` | `module_dir` | 5 | 1100 | `exists_active` | directory exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/opcodes.rs` — all relevant opcode constants (see above).
- `crates/wow-world/src/handlers/misc.rs:245`/`604` — `handle_gm_ticket_get_case_status` registered and dispatched, but the body is `pub async fn handle_gm_ticket_get_case_status(&mut self, _pkt: WorldPacket) {}` — empty no-op.
- `crates/wow-packet/src/packets/misc.rs:281` — outbound `SMSG_FEATURE_SYSTEM_STATUS` advertises `ComplaintsEnabled = false` (intentional placeholder per comment "default false").
- `crates/wow-world/src/session.rs:1811` — dispatcher routes `GmTicketGetCaseStatus` to the no-op above.
- `crates/wow-database/src/statements/` — **no** support-related prepared statements registered.

**What's implemented:** opcode constants and one stub handler. Nothing else.

**What's missing vs C++:** literally everything past the wire layer — `Ticket` struct hierarchy, `SupportMgr`, prepared statements, schema migration for `gm_bug` / `gm_complaint*` / `gm_suggestion`, GM-command surface, the `SMSG_*` reply packets.

**Suspicious / likely divergent:** none — the current Rust stub is honest about being a stub.

**Tests existing:** none.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SUPPORT.WBS.001** Partir y cerrar la migracion auditada de `game/Support/SupportMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.cpp`
  Rust target: `crates/wow-support`, `crates/wow-social`, `crates/wow-handler`, `crates/wow-database/src/statements`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 808 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SUPPORT.WBS.002** Cerrar la migracion auditada de `game/Support/SupportMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.h`
  Rust target: `crates/wow-support`, `crates/wow-social`, `crates/wow-handler`, `crates/wow-database/src/statements`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#SUP.1** Add a `wow-support` crate (or module under `wow-social`). Define `enum TicketKind { Bug, Complaint, Suggestion }`, `ReportType`, `ReportMajorCategory`, `ReportMinorCategory` (bitflags), `Ticket` base struct with the shared fields (`id, player_guid, map_id, position, create_time, closed_by, assigned_to, comment`), and `BugTicket` / `ComplaintTicket` / `SuggestionTicket` value types holding their per-kind extras. (complexity: **M**)
- [ ] **#SUP.2** Add prepared statements to `crates/wow-database/src/statements/character.rs`: `SEL_GM_BUGS`, `SEL_GM_COMPLAINTS`, `SEL_GM_COMPLAINT_CHATLINES`, `SEL_GM_SUGGESTIONS`, plus the REP/DEL counterparts for each. Schema is already in TrinityCore's `character_database.sql` — port as-is. (complexity: **M**)
- [ ] **#SUP.3** Implement `SupportMgr` as `Arc<RwLock<Inner>>` holding three `BTreeMap<u32, Ticket>` and `last_*_id: u32` + `open_*_count: u32`. Add `load_all(pool)`, `add_ticket`, `close_ticket(kind, id, closed_by)`, `remove_ticket(kind, id)`, `reset_tickets(kind)`, `get<T>(id)`, `get_complaints_by_player(guid)`, plus the four system-status booleans loaded from config. (complexity: **H**)
- [ ] **#SUP.4** Implement the per-kind `SaveToDb` / `DeleteFromDb` codecs against the prepared statements; `ComplaintTicket` saves chat-log child rows in the same transaction. (complexity: **M**)
- [ ] **#SUP.5** Wire the four currently-stub opcode handlers in `wow-world/src/handlers/misc.rs`: `GmTicketGetSystemStatus`, `GmTicketGetCaseStatus`, `GmTicketAcknowledgeSurvey`, `Complaint`. Add parsers + senders for `SubmitBug`/`SubmitComplaint`/`SubmitSuggestion` and their `*Result` reply opcodes. (complexity: **M**)
- [ ] **#SUP.6** Flip the `ComplaintsEnabled` bit in `wow-packet/src/packets/misc.rs:281` (`SMSG_FEATURE_SYSTEM_STATUS`) when the corresponding `CONFIG_SUPPORT_COMPLAINTS_ENABLED` is true. Same for the other three flags (Tickets/Bugs/Suggestions). (complexity: **L**)
- [ ] **#SUP.7** GM-command surface (deferred — depends on `wow-handler`'s command framework existing): `.ticket list`, `.ticket close`, `.ticket assign`, `.ticket comment`, `.ticket teleport`. (complexity: **M**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SUPPORT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 1120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.h`. Rust target: `crates/wow-handler`, `crates/wow-social`. | `cargo test -p wow-handler && cargo test -p wow-social` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SUPPORT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 1120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.h`. Rust target: `crates/wow-handler`, `crates/wow-social`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SUPPORT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 1120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.h`. Rust target: `crates/wow-handler`, `crates/wow-social`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SUPPORT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 1120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.h`. Rust target: `crates/wow-handler`, `crates/wow-social`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] `SupportMgr::load_all` round-trips: insert fixtures via SQL; load; assert all three maps' sizes match; assert `last_*_id` is the max id loaded.
- [ ] `add_ticket` increments `open_*_count`; `close_ticket` decrements it; `remove_ticket` of a closed ticket does NOT double-decrement.
- [ ] `ComplaintTicket` with a 5-line chat log persists and reloads with all 5 lines in original order.
- [ ] `generate_*_id` is monotonic across a save-load-save cycle (no collisions after restart).
- [ ] `SMSG_FEATURE_SYSTEM_STATUS` flips `ComplaintsEnabled` correctly when the corresponding config is toggled at runtime.
- [ ] `SMSG_GM_TICKET_SYSTEM_STATUS` returns `DISABLED` when `support_system_status` is false even if the per-feature toggles are true (master flag wins).
- [ ] Submitting a complaint when complaints are disabled is rejected (no DB write, no echoed `*Result`).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 1120 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.h` | would live in a new `crates/wow-support/` (or fold into `wow-social`); GM-command surface eventually in `wow-handler` \| ❌ not started — confirmed via audit 2026-05-01 (one decoded opcode routed to a no-op `{}`; `ComplaintsEnabled = false` hardcoded; zero DB statements; zero ticket schema) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SUPPORT.DIV.001` | `crates/wow-support` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 1120 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#SUPPORT.DIV.002` | `crates/wow-social` (`exists_empty`, 0 Rust lines) | 2 C++ files / 1120 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Support/SupportMgr.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **Master vs per-feature flags**: `GetTicketSystemStatus()` returns `_supportSystemStatus && _ticketSystemStatus`. The master flag has veto. Mirror this; don't naively expose only the per-feature ones.
- **`ChatLog` is a wire packet struct reused as in-memory storage**: `using ChatLog = WorldPackets::Ticket::SupportTicketChatLog;`. Convenient in C++; in Rust, define one struct in `wow-packet` (Serialize) and reuse via `Clone`.
- **`Generate*Id` is NOT atomic** in C++ (plain `++_lastBugId`). Trinity assumes `SupportMgr` is only ever called from the world thread. RustyCore is multi-threaded — wrap the counters in `AtomicU32` or take the `RwLock` for the whole "generate id + insert" sequence.
- **`RemoveTicket` deletes the in-memory `Ticket*` via `delete`** — owning raw pointers. Rust port owns by value; no equivalent leak risk.
- **Modern report categories (`ClubFinder*`, `BattlePet`, `Friend`) are 3.4.3-client-incompatible** but kept in the schema. Don't strip them — leaving the columns and treating unknown values as opaque is forward-compatible and matches Trinity's choice.
- **Position is stored** (`SetPosition(mapId, pos)`) so a GM can `.ticket teleport` to where the player filed it. Ensure the `Position` field is populated at submit time even on bug tickets (Trinity does).
- **`UpdateLastChange()` updates a `_lastChange` timestamp** that the client polls via `SMSG_GM_TICKET_SYSTEM_STATUS` to know whether to refresh its UI. Don't forget to bump it on every mutation.
- **`CMSG_COMPLAINT`** (the spam-report shortcut, opcode `0x366e`) does NOT create a persistent `ComplaintTicket` — it's a quick "I'm muting/reporting this guy for chat spam" path that goes to `SupportSpamType` logging only. Don't confuse with the full `SupportTicketSubmitComplaint` flow.
- **`gm_complaint_chatlog`** child rows must be deleted with the parent; either rely on `ON DELETE CASCADE` in the schema or issue both deletes in the same transaction. Trinity uses both `CHAR_DEL_GM_COMPLAINT` and `CHAR_DEL_GM_COMPLAINT_CHATLOG` defensively.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class SupportMgr` (singleton) | `pub struct SupportMgr` injected as `Arc<SupportMgr>` | No global; route through `WorldSession` context |
| `class Ticket` (abstract) | `trait Ticket { fn save(...); fn load(...); fn delete(...); fn format(...); }` + value-type structs | No virtual dtor needed |
| `BugTicket : Ticket` | `pub struct BugTicket { base: TicketBase, note: String }` | Composition over inheritance |
| `ComplaintTicket` | `pub struct ComplaintTicket { base: TicketBase, target: ObjectGuid, report_type: ReportType, major: ReportMajorCategory, minor: ReportMinorCategory, chat_log: ChatLog, note: String }` | — |
| `enum class ReportType : i32` | `#[repr(i32)] enum ReportType { … }` | — |
| `enum class ReportMinorCategory : i32` (bitmask) | `bitflags! struct ReportMinorCategory: i32 { … }` | — |
| `map<u32, T*>` | `BTreeMap<u32, T>` | Ordered for `.ticket list` consistency |
| `template<typename T> T* GetTicket(u32)` | three explicit getters: `get_bug`, `get_complaint`, `get_suggestion` (or one generic over `enum TicketKind`) | Avoids monomorphic explosion |
| `SaveToDB() / LoadFromDB(Field*)` | `async fn save(&self, tx: &mut Transaction) -> Result<()>` / `fn from_row(row: Row) -> Self` | Async DB |
| `_lastChange: u64` (ms) | `last_change: AtomicU64` | Lock-free read |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

**Method:** `grep -rEi "(SupportMgr|support_mgr|support_ticket|gm_bug|gm_complaint|gm_suggestion)" crates/`. Inspected `crates/wow-world/src/handlers/misc.rs`, `crates/wow-world/src/session.rs`, and `crates/wow-packet/src/packets/misc.rs`.

**Verdict on doc claim "❌ not started": CONFIRMED with one nuance.** Of the 4 listed support opcodes, only `GmTicketGetCaseStatus` is actually wired to a stub handler (`handle_gm_ticket_get_case_status` at `handlers/misc.rs:551` — body `pub async fn handle_gm_ticket_get_case_status(&mut self, _pkt: wow_packet::WorldPacket) {}`). The other three (`GmTicketGetSystemStatus`, `GmTicketAcknowledgeSurvey`, `Complaint`) plus the three `SupportTicketSubmit{Bug,Complaint,Suggestion}` opcodes have **no registered handlers at all** — they would fall through to the unknown-opcode log path. Update §8 to reflect that (it currently implies all four are stubbed).

**Findings:**

1. **`SupportMgr` — CONFIRMED ABSENT.** Zero hits across `crates/`. No `Ticket` / `BugTicket` / `ComplaintTicket` / `SuggestionTicket` structs. No `wow-support` crate.
2. **DB layer — CONFIRMED ABSENT.** No `gm_bug` / `gm_complaint` / `gm_suggestion` strings anywhere. No `CHAR_SEL_GM_BUGS` / `CHAR_REP_GM_COMPLAINT` / etc. statement constants in `crates/wow-database/src/statements/character.rs`.
3. **`SMSG_FEATURE_SYSTEM_STATUS` flag — CONFIRMED stubbed off.** `crates/wow-packet/src/packets/misc.rs:280` writes `pkt.write_bit(false)` for `ComplaintsEnabled` with the comment `(SupportComplaintsEnabled config, default false)`. This is intentional and correct given the absence of a backing implementation; flip when #SUP.6 lands.
4. **Stub handler returns nothing** — `handle_gm_ticket_get_case_status` is empty `{}`; the client expects a `SMSG_GM_TICKET_CASE_STATUS` reply but gets silence. This is observable as a hung "tickets" panel in-game when the player opens it — not crashing, just empty. Acceptable for now.
5. **Spam-report fast-path (`CMSG_COMPLAINT`, opcode 0x366e) is unhandled** — see §11 gotcha. Not in any `inventory::submit!` block.
6. **GM commands (`.ticket *`, `.bug`, `.complaint`)** — the command-dispatcher framework does not yet exist; #SUP.7 correctly defers.
7. **No tests** for any support code path (consistent with absence of code).

**Modern-vs-classic categorization scope:** §11 notes that `ClubFinder*`, `BattlePet`, `Friend` report categories are 3.4.3-client-incompatible but kept defensively. No need to drop them; the schema is forward-compatible. The Rust port should mirror this leave-them-alone posture when #SUP.1 lands.

**Status verdict:** ❌ not started (no change). Priority: **low** — `/who` (`storages.md`) and Mail/Friends correctness (`cache.md`) are higher-value than ticket submission for a 3.4.3 Classic private server. Sub-tasks #SUP.1–#SUP.7 are well-scoped and can land in any order after a `wow-support` crate skeleton is created. Recommend doing #SUP.6 (the `ComplaintsEnabled` bit-flip) opportunistically when adjacent code touches `SMSG_FEATURE_SYSTEM_STATUS` — it's a 3-line change once #SUP.3 exists.
