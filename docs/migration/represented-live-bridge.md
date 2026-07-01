# Represented -> Live Bridge Convention

Issue #19 defines the minimal bridge convention for converting represented
handler work into live runtime mutations without ad-hoc per-handler wiring.

## Convention

1. Packet handlers still own decoding and C++ validation.
2. After validation, the handler builds one `RepresentedLiveIntentLikeCpp`.
3. The handler calls `WorldSession::record_and_apply_represented_live_intent_like_cpp(intent)`
   exactly once.
4. The bridge records the represented evidence and applies the live mutation.
5. The handler must not also mutate the same live state or send the same packets directly.

The bridge is intentionally narrow. Add one intent variant at a time, with the
C++ anchor, live owner, packet side effects, and a focused test proving one
record + one application. Do not move DB writes, map locks, or broad runtime
ownership through this bridge unless that slice explicitly owns those concerns.

## First Converted Example

`RepresentedLiveIntentLikeCpp::DuelAccepted` converts the accepted-duel path.

- C++ source: `src/server/game/Handlers/DuelHandler.cpp`, `WorldSession::HandleDuelAccepted`.
- Rust handler validation: `WorldSession::handle_duel_accepted_like_cpp`.
- Rust bridge application: `WorldSession::apply_represented_duel_accepted_live_like_cpp`.

The handler verifies the challenged duel state and arbiter, then emits one
`DuelAccepted` intent. The bridge records the intent, switches both canonical
players to `PlayerDuelStateLikeCpp::Countdown`, and sends one `DuelCountdown`
packet to each participant. Tests cover the direct bridge path and the packet
handler path.
