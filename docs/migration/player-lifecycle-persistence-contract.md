# Player lifecycle persistence contract

Issue #187 freezes the narrow contract that the Player persistence port in #200 must preserve.
It is not a claim that Rust already implements all of `Player::LoadFromDB` or `Player::SaveToDB`.
The executable fixture is
`crates/wow-world/tests/fixtures/player-lifecycle-contract.json`.

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

These discrepancies are inputs to #200 and later parity work. Changing them in the architecture
move requires an explicit behavior PR rather than silently changing the fixture.
