# Player lifecycle persistence contract

Issue #187 freezes the narrow contract that the Player persistence port in #200 must preserve.
It is not a claim that Rust already implements all of `Player::LoadFromDB` or `Player::SaveToDB`.
The executable fixture is
`crates/wow-world/tests/fixtures/player-lifecycle-contract.json`.

This remains a bounded compatibility contract, not a current full-persistence
audit. The #187/#200/#286 notes below retain their implementation history;
current ownership and remaining work follow [STATE.md](STATE.md) and
[#578 C0–C4](../architecture/session-578-checkpoint.md). Do not recompute the full
persistence inventory for a metadata-only refresh of this document.

## C++ anchors

- Load: `Player::LoadFromDB` and the `LoginQueryHolder` path in
  `src/server/game/Handlers/CharacterHandler.cpp`.
- Save: both `Player::SaveToDB` overloads in
  `src/server/game/Entities/Player/Player.cpp`. The Character transaction is committed before the
  Login transaction; runtime dirty state may only be cleared after the relevant commit succeeds.
- Logout: `WorldSession::LogoutPlayer` in
  `src/server/game/Server/WorldSession.cpp`. The player is saved while it is still present in its
  map/session context, then removed and the login boundary is released.

The fixture records semantic groups and parameter classes rather than values or the incomplete
Rust statement inventory. It therefore contains no account name, credential, session key, or raw
SQL.

## Deliberately frozen boundaries

- Player load reads belong to CharacterDatabase on a pooled connection and runtime publication
  follows successful loading.
- Periodic and manual saves share one CharacterDatabase transaction for the represented snapshot.
- A definite pre-COMMIT failure rolls back and keeps runtime dirty state unpublished.
- A successful commit precedes dirty-state publication.
- An unknown COMMIT outcome is not treated as rollback or success; it fences further mutation and
  requires a fresh login.
- Logout drains pending durable work before save, persists account collections in LoginDatabase,
  publishes offline/removal state, and only then releases the sole-login claim so reconnect may
  proceed.

## Known Rust/C++ discrepancies (not corrected by #187)

- C++ loads the Player through a prepared `LoginQueryHolder`; Rust still issues a sequence of
  individual load queries from `handle_continue_player_login`.
- C++ appends account collection writes to one LoginDatabase transaction inside `SaveToDB`. Rust
  currently commits mounts, toys, heirlooms, appearances, and illusions as separate transactions
  after the CharacterDatabase save.
- Rust's represented Player save is partial and its extra offline writes do not yet equal the full
  C++ save inventory.
- Rust explicitly fences an indeterminate COMMIT and forces relogin. The fixture preserves that
  current safety behavior without claiming it is a completed C++ parity implementation.

These discrepancies were inputs to #200 and remain bounded historical findings
until re-contrasted. Intentionally changing behavior still requires explicit
approval and separate behavior evidence; do not silently change the fixture in
a refactor. An already approved behavior change need not acquire a second
approval or a micro-PR solely because it is part of a larger authorized macro.

## #286 character-save port

The represented Character-database save now crosses the lifecycle boundary as
`PlayerCharacterSaveRequestLikeCpp`, one SQLx-free semantic snapshot grouped into character
scalars, spells, skills, talents/glyphs, action bars, cooldowns/charges, equipment/void storage,
tutorials, instance restrictions, played time, reputation and CUF profiles. It contains no
statement identifier or generic parameter bag. MariaDB SQL text, prepared parameters, statement
decomposition, transaction and driver error remain private to
`wow_database::player_lifecycle_adapter::MariaDbPlayerLifecycleAdapterLikeCpp`.

The executable contracts are split deliberately:

- `session::tests::save_plan_order::the_character_save_request_is_semantic_and_deterministic_like_cpp`
  pins the real Session's deterministic semantic request without exposing SQL;
- `player_lifecycle_adapter::tests::save_order::character_save_adapter_preserves_the_frozen_statement_order_like_cpp`
  expands a semantic request and compares the exact SQL run order with
  `player-save-plan-order.json`; the fixture now covers every represented group plus equipment,
  transmog, void-storage and CUF insert/update/delete branches rather than the former twelve-group
  subset;
- `player_lifecycle_adapter::tests::save_steps::every_private_character_save_operation_maps_to_the_existing_mariadb_statement_like_cpp`
  covers every private adapter operation and proves it still selects the previous MariaDB statement;
- the lifecycle persistence tests drive production orchestration through a fake port and prove
  `Applied` clears dirty state, definite rollback preserves it, and unknown COMMIT both preserves
  it and closes the mutation fence.

The adapter returns the semantic dirty groups alongside the three-way outcome; Session consumes
them only after `Applied`. This is an architecture move, not a claim of wider `Player::SaveToDB`
parity. The represented statement inventory and known Rust/C++ discrepancies above are unchanged.
