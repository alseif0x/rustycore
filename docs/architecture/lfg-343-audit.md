# Automatic Dungeon Finder 3.4.3 — bounded audit and proposed ownership

Date: 2026-09-13. Historical design audit at `13c984a6`; bounded decoder delivery
revalidated on current `3.4.3` at `115eb699`/`fbf5664b` (aarch64 development host).
Status: protocol decoder slice implemented and ready for integration; this remains
not full LFG parity or client acceptance.
C++ root for relative anchors below: `/home/server/woltk-trinity-legacy/src/server/game`.

## Verdict and scope

Automatic Dungeon Finder has substantial C++ machinery but no complete live Rust path.
Rust already has immutable catalogs, informational packets and partial LFG group state; do not
replace those with duplicate stores. Manual LFG List is a separate incomplete feature.
The user prioritized this audit after the repaired login smoke. It does not close #578,
supersede the entire port plan, or authorize publishing the current branch.

## Evidence and acceptance gaps

| Surface | C++ anchor | Current Rust / remaining boundary |
|---|---|---|
| Catalogs and locks | `DungeonFinding/LFGMgr.cpp:119,188`; `Handlers/LFGHandler.cpp:150` | `wow-data/src/lfg.rs`, world-server catalog loaders and `handlers/misc/lfg.rs` exist. Seasonal activity is hard-coded false; party lock response lacks a live LFG manager. |
| Join, leave, roles, vote, teleport | `Handlers/LFGHandler.cpp:31-104`; `Server/Protocol/Opcodes.cpp:425-430` | `wow-packet/src/packets/misc/lfg_client.rs` now decodes the six client packets. No handler registration, queue or matchmaking path is claimed. |
| Queue and proposals | `DungeonFinding/LFGQueue.cpp:288,358,576`; `LFGMgr.cpp:286,397,719,1052,1160` | No live queue, role-check/proposal state machine or deadline driver found. Information/status handlers do not constitute matchmaking. |
| Group formation | `LFGMgr.cpp:945-1034` | Existing `wow-social::group::GroupRegistry` is canonical. It already restores LFG dungeon/state, owns assigned member roles and gates direct kicks; it does not execute accepted DF proposals. |
| Teleport and completion | `LFGMgr.cpp:1357,1449,1472` | Must invoke canonical Player/Map transfer and existing rewards/persistence; a successful proposal is not dungeon completion. |
| Login, logout, membership | `DungeonFinding/LFGScripts.cpp:40,51,151,207` | Need explicit LFG lifecycle hooks, cancellation, continuation and reconnect handling. Ordinary group APIs alone do not provide these. |
| Manual listings | `Handlers/LFGHandler.cpp:584-632` | C++ search returns zero results and apply returns explicitly unimplemented. Keep out of the automatic-DF acceptance claim. |

Issues #550 and #552 are CLOSED catalog-capability extractions, not gameplay closeouts.
On the audit date no open issue with LFG in its title was returned; this is a bounded title
search, not proof that no related issue exists. Index #49 and #47 still leave the Part-2
transition gate open. Do not create a large Part-2 issue campaign from this document.

## QA defects and C++ uncertainty found before implementation

1. **Bot listens on the wrong connection for LFG results.**
   `tools/wow-test-bot/src/main.rs:6230-6325` reads only the current instance stream in its LFG
   loop; the realm stream is retained separately after ConnectTo. C++ routes JOIN_RESULT,
   PROPOSAL_UPDATE and UPDATE_STATUS on REALM (`Opcodes.cpp:1623,1638,1645`). The repaired
   login-only drain does not repair this separate LFG scenario. Read both streams with
   cancellation-safe framing and record the receiving connection.
2. **Bot treats party lock information as group-formation proof.**
   Its `SMSG_LFG_PARTY_INFO` branch sets `group_formed`; C++ `SendLfgPartyLockInfo` sends this
   informational response on a system-info query. Require an actual group update with the
   expected five members, assigned roles and LFG identity, plus server/persistence evidence.
3. **Bot assumes an additional DF ready-check exchange.**
   It sends opcode 0x361C; C++ registers DF_READY_CHECK_RESPONSE as STATUS_UNHANDLED/Handle_NULL
   (`Opcodes.cpp:428`). Do not make this exchange a prerequisite for C++-anchored automatic DF.
   Distinguish party ready checks, role checks and proposal acceptance.
4. **Potential inherited multi-proposal publication defect.**
   `LFGMgr::Update` saves the previous ID, lets queues generate proposals, then starts publication
   at `ProposalsStore.find(m_lfgProposalId)` (`LFGMgr.cpp:342-351`). `AddProposal` increments that
   ID for each insertion (`1038-1042`), while `FindGroups` can produce multiple matches (`LFGQueue.cpp:288`).
   Static inference: if two proposals are created in one update, the earlier new proposal is
   skipped by that publication loop. This is not a live reproduction. Add a deterministic
   two-proposal regression and document an intentional C++ defect correction separately from
   fidelity work; do not copy it silently or claim a tested fix.

Existing bot comments referencing `main_srp6_complete.rs` are not protocol authority. Audit its
helpers against `Server/Packets/LFGPackets.cpp:20-62,362-394` before reuse. Current DFJoin Roles
is uint8 in the C++ header, not an inferred uint32. No new wire layout is approved by this audit.

## Proposed canonical ownership and dependency direction

Start with private implementation modules and narrow public contracts in existing crates, not
a new `wow-lfg` crate or a second global world simulation loop.

| State / responsibility | Proposed owner and boundary |
|---|---|
| Tickets, selected slots, queued role preferences, queue order, compatibility, role checks, proposals and vote deadlines | One process-wide DF authority in `wow-social`, with synchronous commands and explicit time/RNG inputs. No Player copies, sockets, SQL or packet DTOs. |
| Group membership, assigned roles, LFG dungeon/run state and remaining kicks | Existing `GroupRegistry` / `GroupInfo`. Existing `lfg_db_state` and `lfg_kicks_left_like_cpp` must be consumed through this owner, not copied into a second authoritative LFG-group map. |
| Character restrictions, auras, instance locks, position and return destination | Existing canonical Player/Map owners. Application obtains bounded snapshots and revalidates before effects; no persistent Session mirror. |
| Catalogs and rewards definitions | Existing immutable `wow-data` catalogs, supplied as narrow values/queries. Do not add a production `wow-social -> wow-data` edge merely for convenience. |
| Cross-owner join, accepted proposal, transfer, completion and publication | Private application integration in `wow-world`; handlers only decode, enforce admission, invoke it and present results. Persistence goes through existing typed ports/adapters. |
| Runtime construction and supervision | `world-server`; exactly one LFG driver, no per-session/per-map matching ticks. |

`cargo metadata` confirms wow-social production dependencies are only dashmap, wow-core and
wow-constants; wow-world/world-server already consume wow-social. Preserve that direction.
Queue role preferences and assigned group roles have different meanings: distinguish their types
and transition rather than synchronizing interchangeable mutable copies.

For proposal acceptance, reserve one proposal transition, release its lock, apply the group
operation through the canonical owner, then finalize/reject with identity/generation checks.
The exact transaction and failure/retry contract must be frozen before runtime implementation.
Never hold an LFG/Group/Map guard across SQL, await or network publication.

Clock integration remains an explicit design gate. C++ runs GroupMgr::Update then LFGMgr::Update
in `World.cpp:2790-2796`. Rust actually has a group-ready-check driver at
`world-server/src/runtime/map.rs:1535`, started by `app.rs:5357`, in addition to map/legacy clocks.
Evaluate extending that social driver while preserving its ordering before choosing a separate
task. Do not attach realm-wide matching to every map tick or claim the old two-clock snapshot
describes all current periodic tasks. No driver or cadence was changed here.

Mirror retirement: existing group fields remain their authoritative state. Any temporary queue
membership index is derived, invalidated on leave/disconnect/group change, and retired when
the authority can answer the query directly. Do not duplicate GroupRegistry membership.

## Incremental execution and verification

These are dependency-ordered scope candidates. The first bounded decoder subset was subsequently
selected as #582; the other candidates are not newly created issues or completed implementations.

1. **Protocol and QA boundary:** C++-anchored decoders/encoders and a two-socket LFG bot scenario.
   Test truncation, bit alignment, optional PartyIndex, packed tickets and routing. No fake JOIN_OK.
2. **Join/leave and lifecycle authority:** permissions, roles/class filters, party leader/member
   restrictions, dungeon intersection and stale tickets. Test cancellation, logout, duplicates,
   no selected roles, invalid slots and queued party changes. Status reads query actual state.
3. **Matching and proposals:** deterministic role assignment, queue order, timeout/decline/requeue,
   existing-group continuation and multiple simultaneous matches. One proposal per participant;
   repeated responses cannot form a group twice. Preserve and test the C++ timeout boundaries.
4. **Group commit and dungeon transfer:** integrate canonical group persistence and Player/Map
   entry/return paths; revalidate disconnects, instance capacity/locks and failed transfers.
   Five bots must prove one group, correct roles and destination, not just a success packet.
5. **Completion, rewards and lifecycle edges:** encounter-driven completion, first/repeat reward,
   cooldown/deserter, boot vote, replacement, reconnect and restart persistence. Test duplicate
   completion events and failed commits; do not invent durable exactly-once claims from an
   in-memory finished flag.

First implementation candidate is the protocol/QA boundary; the full runtime owner/clock choice
must be resolved before activation. Use a dedicated issue and linked branch into 3.4.3 when the
scope is selected; do not add LFG gameplay to #578. Audit the chosen base because the successful
login build currently includes unpushed #578 changes. Do not push/merge those as an implied prerequisite.

## Data and remaining limits

The prior checkpoint records suspicious local Hotfix IDs 256/258 and an exact-build export with
empty descriptions. No SQL rows were read or changed in this audit. Before enabling queues,
audit the effective DB2+Hotfix catalog and all references; do not invent descriptions or rewrite
IDs from names alone. Seasonal support, instance completion and reward dependencies need focused
source audits; their presence elsewhere in Rust is not proof of complete LFG integration.

Validation performed here: read-only source/manifest/issue inspection and documentation diff check.
No server restart, bot traffic, fresh capture-diff, gameplay implementation or issue closeout.

## First implementation follow-up — issue #582

The user authorized continuation into the protocol boundary. Branch
`582-lfg-client-packet-decoders` was rebased from its original `80b9e682` base onto current
`3.4.3` `6311d48e`; the rebased implementation commit is `115eb699` and the evidence commit
is `60c11527`. It contains no unpushed #578 gameplay changes. The original audit above remains
a snapshot of `13c984a6`, not a claim of full LFG parity.

`crates/wow-packet/src/packets/misc/lfg_client.rs` adds six ClientPacket decoders: DFJoin,
DFLeave, DFProposalResponse, DFSetRoles, DFBootPlayerVote and DFTeleport. Existing packet types
and RideTicket serialization remain unchanged. C++ `LFGPackets.h:46-47` defines uint8 roles and
an Array<uint32,50>; `PacketUtilities.h:195-200` rejects excess capacity before the remaining
fields are read. C++ `LFGPackets.cpp:20-62` supplies field/bit order;
`LFGPacketsCommon.cpp:20-31` and `PacketUtilities.h:241-279` anchor the packed ticket and int64
timestamp. Eligibility, selected-role sanitization and ticket ownership are not decoder rules.

Ten focused tests cover each truncated byte prefix, independent flags and optional PartyIndex branches,
zero/50/51/u32::MAX counts, typed dungeon entries, signed 64-bit ticket time, proposal alignment,
and every possible byte for the MSB-only vote/teleport flags. This is a represented wire-decoding
increment, not LFG runtime parity. All ten focused tests and the full 734-test wow-packet library
suite pass on the aarch64 host. The preliminary quick gate also passed; final validation is
recorded separately from these focused checks.

Current revalidation on `fbf5664b` passed with one Cargo job: the ten focused decoder tests and
the full `wow-packet` library suite (738 tests, zero failures), plus `cargo fmt --all -- --check`
and `git diff --check`. The applicable validation-v2 quick gate is run for the publication
candidate and passed in 28.2s with manifest
`target/validation-v2/manifests/20260913T034550.555683Z-3329863-quick.json`. This evidence is
source-anchored wire decoding, not capture-diff equivalence.

No handlers are registered and no packets are emitted by production: queue ownership, server
responses, bot two-socket routing/group proof, runtime clocks, group creation, teleport execution,
rewards, live acceptance and data repair remain pending. No inventory-wide progress percentage or
full-LFG closeout is claimed. Fresh live captures become required when these handlers are activated.
