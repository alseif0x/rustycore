# RustyCore — Honest Current State (single source of truth)

## #1368 UpdatePhaseShift objective completion accepted locally — 2026-11-21

Committed candidate `51bda221d641b984f51c4e3e3f8706ea77ba35ee`, based on
`1ff5297ca`, implements the objective-completion `UpdatePhaseShift` action in
all five represented objective-progress families. Following
`Player::UpdateQuestObjectiveProgress` and `PhasingHandler::OnConditionChange`
at `02245dcd245e7433e524577656177723d3e4992e`, the final interaction refresh
runs first, then the canonical Player phase conditions, phase auras/groups and
terrain swaps are recomputed. A real mutation publishes `PhaseShiftChange` and
forces visibility before the existing zone and area aura casts. Unsupported
conditions fail closed instead of forcing a transition.

Two regressions prove suppressed-phase reactivation and the complete combined
packet order: credit, both interaction refreshes, phase change, visibility
refresh, then zone and area `SpellGo` packets. The objective suite passed 30
tests. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 1ff5297ca --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 00:55:12Z to 00:58:41Z (208.609
seconds), within the 600-second target: 4,651 `wow-world` tests (one ignored)
and eight `wow-world-quest` tests passed, totaling 4,659 passed, one ignored
and zero failures. Verified manifest:
`target/validation-v2/manifests/20261121T005511.910853Z-1259835-final.json`.
Architecture policy, session syntax ownership, method-index, hotspot/census,
formatting and all affected downstream checks passed. Reviewed ledgers record
53 production and 107 test lines in Session and two new test identities. The
exhaustive persistence inventory was not rerun; it previously timed out and is
not claimed by this bounded acceptance.

This closes the objective-triggered phase mutation/publication action in the
current represented phase model, not every generic `OnConditionChange` side
effect. Controlled-unit inheritance, `Unit::OnPhaseChange`, removal of
non-own single-target auras and the `DisableMgr` phase-area gate remain outside
this slice. Full `SpellInfo::CheckLocation` removal, GameEvent and Conversation
also remain open in #1368. This is synthetic production-path
source/behavior/wire evidence, not a live quest capture or full
completion-effect parity claim. Player runtime integration and character
creation remain separate work.

## #1368 UpdateAreaDependentAuras spell-area casts accepted locally — 2026-11-21

Committed candidate `63ee53527665e244e6019e626909f7e424c11d64`, based on
`c56695871`, adds the spell-area cast half of `UpdateAreaDependentAuras`. One
shared application helper selects AUTOCAST rows by area, evaluates their
requirements and runs missing spells through the real triggered executor. Zone
casts still run first, then area casts, after the final interaction refresh.
The separate `SpellInfo::CheckLocation` removal pass is not implemented or
claimed by this slice.

The existing regression now installs distinct zone and subzone rows and requires
credit, both interaction refreshes and two `SpellGo` packets. The objective
suite passed 28 tests with the same test identities. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base c56695871 --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 23:46:38Z to 23:51:55Z (317.555
seconds), within the 600-second target: 4,649 `wow-world` tests (one ignored),
32 `wow-world-application` and eight `wow-world-quest` tests passed, totaling
4,689 passed, one ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T234636.865118Z-1114614-final.json`.
Architecture policy, session syntax ownership, method-index, hotspot/census,
formatting and all affected downstream checks passed. Reviewed ledgers record
12 production and four test lines in Session with no new test identity.

This closes only the spell-area cast half of `UpdateAreaDependentAuras` in
#1368. Area removal with full `CheckLocation`, UpdatePhaseShift, GameEvent and
Conversation remain open. This is synthetic production-path source/behavior/wire
evidence, not a live quest capture or full completion-effect parity claim.
Player runtime integration and character creation remain separate work.

## #1368 UpdateZoneDependentAuras accepted locally — 2026-11-21

Committed candidate `a10132e10a71fc1816ddd6bd3f08908cbd1f733c`, based on
`ce9e3d6d5`, implements the C++ zone-half of completion-effect aura updates.
Every represented objective family records `UpdateZoneAuras` when an objective
becomes complete. After the final interaction refresh, WorldSession asks the
application owner to select current-zone `spell_area` rows with AUTOCAST and to
run each missing spell through the real triggered executor in spell-id order.
Requirements and zone/aura authority fail closed instead of fabricating a cast.
`UpdateAreaDependentAuras` and full `SpellInfo::CheckLocation` are deliberately
outside this slice.

One regression completes a storing-value objective with a zone action and a real
`spell_area` row. It requires credit, the CompleteQuest refresh, the final
refresh and only then `SpellGo`. Zone and aura authorities are installed after
canonical Player attachment. The complete objective suite passed 28 tests. The
committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base ce9e3d6d5 --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 23:26:03Z to 23:29:04Z (181.063
seconds), within the 600-second target: 4,649 `wow-world` tests (one ignored),
32 `wow-world-application` and eight `wow-world-quest` tests passed, totaling
4,689 passed, one ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T232603.119351Z-1057856-final.json`.
Architecture policy, session syntax ownership, method-index, hotspot/census,
formatting and all affected downstream checks passed. Reviewed ledgers record
36 production and 67 test lines in Session and one new test identity.

This closes only `UpdateZoneDependentAuras` in #1368. Area-dependent removal
and casts with full `CheckLocation`, UpdatePhaseShift, GameEvent and
Conversation remain open. This is synthetic production-path source/behavior/wire
evidence, not a live quest capture or full completion-effect parity claim.
Player runtime integration and character creation remain separate work.

## #1368 completion spells across objective families accepted locally — 2026-11-20

Committed candidate `6dab45a9f70104cfe9930ca825f2d7d478eddf23`, based on
`31dfb7b91`, extends the existing deferred completion-spell boundary from
storing-value to storing-flag, money, currency and reputation objectives. One
shared selector reads the loaded action and ignores only spell id zero. Every
family stops before `CompleteQuest`, drains any earlier inline completions and
uses the same real cast/resume operation. Each family's final interaction refresh
is marked so the resumed operation neither loses nor duplicates it.

Four regressions prove storing-flag credit, SpellGo and both refreshes, plus the
same cast/completion order for money, currency and reputation without a credit
packet. The complete objective suite passed 27 tests. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 31dfb7b91 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 22:41:35Z to 22:46:50Z (314.678 seconds),
within the 600-second target: 4,648 `wow-world` tests (one ignored) and eight
`wow-world-quest` tests passed, totaling 4,656 passed, one ignored and zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T224135.270313Z-935611-final.json`.
Architecture policy, session syntax ownership, method-index, hotspot/census,
formatting and all affected downstream checks passed. The reviewed ledgers record
118 new Session test lines and four identities, with no Session production
growth in this slice.

This closes the `CompletionEffect.SpellId` divergence for all five represented
objective-progress families in #1368. GameEvent, Conversation, UpdatePhaseShift
and UpdateZoneAuras execution remain open. This is synthetic production-path
source/behavior/wire evidence, not a live quest capture or full completion-effect
parity claim. Player runtime integration and character creation remain separate
work.

## #1368 storing-value completion spell accepted locally — 2026-11-20

Committed candidate `84697bcc374c566b47e0d79350d3958d8f4e5a03`, based on
`c3bb10181`, executes a nonzero `CompletionEffect.SpellId` for storing-value
objectives. Following `Player.cpp:17161-17169` at
`02245dcd245e7433e524577656177723d3e4992e`, the domain stops after objective
credit and before `CompleteQuest`. WorldSession executes the real server-triggered
self cast with `TRIGGERED_FULL_MASK`, completes that quest and resumes the same
operation. Previously completed objectives are skipped on retry; pending
completion-state change and refresh flags preserve exactly one final interaction
refresh. Cast failure is logged without preventing completion. Session factory
installs the existing process-owned creature-spawn catalogs into
`SessionSpellState` for this executor.

One composed regression proves `QuestUpdateAddCredit`, `SpellGo`, the
`CompleteQuest` interaction refresh and the final refresh in that exact order,
with the quest complete and its counter at one. The focused objective suite
passed 23 tests. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base c3bb10181 --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 22:10:18Z to 22:19:38Z (559.594
seconds), within the 600-second target: 649 `world-server` tests (one ignored),
4,644 `wow-world` tests (one ignored), eight `wow-world-quest` and 193
`wow-world-spell` tests passed, totaling 5,494 passed, two ignored and zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T221018.369755Z-870611-final.json`.
Architecture policy, session syntax ownership with 2,688 associated items,
method-index, hotspot/census, formatting and all affected downstream checks
passed. The reviewed ledgers and method-index reason record the exact execution
boundary and test growth without a new WorldSession field.

This closes only the storing-value spell-effect divergence in #1368. Other
objective families and GameEvent, Conversation, UpdatePhaseShift and
UpdateZoneAuras execution remain open. This is synthetic production-path
source/behavior/wire evidence, not a live quest capture or full completion-effect
parity claim. Player runtime integration and character creation remain separate
work.

## #1368 objective completion-effect catalog accepted locally — 2026-11-20

Committed candidate `0b128860cfcad72fd24db1f5ae0a52b76664a201`, based on
`8dd7f1eda`, represents C++ `QuestObjectiveAction` on every objective and loads
`quest_objectives_completion_effect` through a `LEFT JOIN` on `ObjectiveID`.
Following `ObjectMgr.cpp:4669-4671` and `QuestDef.cpp:289-297` at
`02245dcd245e7433e524577656177723d3e4992e`, all five fields are preserved:
GameEvent, Spell, Conversation, UpdatePhaseShift and UpdateZoneAuras. No action
exists when every joined value is NULL; nonnull zero/false values still define
an action. The SQL-free persistence row and world-server conversion retain the
same values in the canonical `QuestStore`; existing fixtures explicitly use no
completion effect.

Focused tests prove the query/join shape, NULL-versus-present classification,
world-server conversion of all actions and absence, and QuestStore retention.
`cargo check -p wow-world --tests` also passed for every updated consumer. The
committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 8dd7f1eda --architecture --require-changes --timings --logs`
campaign passed all ten commands from 21:05:12Z to 21:17:27Z (735.350 seconds),
exceeding the 600-second target by 135.350 seconds: 649 `world-server` tests
(one ignored), 1,081 `wow-data` (six ignored), 635 `wow-database` (nine ignored),
1,309 `wow-entities`, 36 `wow-persistence`, 4,643 `wow-world` (one ignored) and
eight `wow-world-quest` tests passed, totaling 8,361 passed, seventeen ignored
and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T210512.323218Z-690798-final.json`.
Architecture policy, session syntax ownership, method-index, hotspot/census,
formatting and all affected downstream checks passed. Reviewed ledgers record
the exact conversion and fixture deltas without new runtime Session authority.

This is catalog implementation and production integration only. The five
completion actions are not yet executed by objective progress, so #1368 remains
open for that runtime behavior. No live database read, effect execution, quest
capture or full parity is claimed. Player runtime integration and character
creation remain separate work.

## #1368 quest-tracker completion accepted locally — 2026-11-20

Committed candidate `e0fccb14b4dcf563fb5043e48cccfcc92f88a4d8`, based on
`00f75b1cad67a0a96c2e03dd2280d8d0dade20bc`, implements the quest
tracker write at the end of represented objective completion. Following
`Player::CompleteQuest` (`Player.cpp:15169-15196`) at
`02245dcd245e7433e524577656177723d3e4992e`, the write runs after the possible
tracking-event reward and uses `CHAR_UPD_QUEST_TRACK_COMPLETE_TIME` with quest id
then Player guid. `Quests.EnableQuestTracker` is composed through the production
runtime policy with C++'s disabled default. Tracker failure is logged without
reverting completion, and the separate quest-status row is not saved immediately.

Three focused tests prove the SQL statement and parameter order, the
configuration key/default, and enabled versus disabled tracker calls with zero
status-persistence requests. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 00f75b1ca --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 19:29:03Z to 19:37:01Z (477.715
seconds), within the 600-second target: 648 `world-server` tests (one ignored),
633 `wow-database` tests (nine ignored), 36 `wow-persistence`, 4,643 `wow-world`
(one ignored), 297 `wow-world-core` and eight `wow-world-quest` tests passed,
totaling 6,265 passed, eleven ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T192903.701217Z-562141-final.json`.
Architecture policy, session syntax ownership with 2,686 associated items,
method-index, hotspot/census, formatting and all affected downstream checks
passed. The reviewed ledgers record the exact policy/test growth, including 74
new Session test lines and one test identity.

The additional exhaustive `session-ownership-check check` was attempted
separately because persistence inputs changed. It did not produce a verdict:
the first run timed out after 1,200 seconds and the explicit extended run timed
out after 2,400 seconds. Both consumed CPU throughout and emitted no ownership
mismatch. No exhaustive persistence-inventory acceptance is claimed or hidden;
the verified bounded final campaign above is the publication evidence.

This closes only the quest-tracker completion-write divergence in #1368.
Objective completion effects and the exhaustive inventory verdict remain open.
This is synthetic source/behavior/SQL evidence, not a real database update,
restart/relogin durability check, live quest capture or full quest parity
acceptance. Player runtime integration and character creation remain separate
work.

## #1368 deferred objective-status persistence accepted locally — 2026-11-20

Committed candidate `851ac19ed9b44306930d44765a2186e738b83a69`, based on
`1413926ee`, removes the five objective-progress families' immediate quest-status
save queue and its end-of-call writes. Objective changes retain the represented
status mutation and dirty tracking, including the `Default` save type assigned by
`SetQuestStatus` for non-turn-in quests. This follows
`Player::UpdateQuestObjectiveProgress` (`Player.cpp:16980-17194`) and
`Player::SetQuestStatus` (`Player.cpp:16230-16246`) at
`02245dcd245e7433e524577656177723d3e4992e`: progress does not issue a separate
immediate status write; ordinary Player saving consumes the dirty state.

One composed canonical-Player regression reduces a required money objective on a
non-turn-in quest and requires `INCOMPLETE`, `Default` dirty state, zero immediate
persistence-port requests and exactly one interaction refresh. The focused
`cargo test -p wow-world s2_quest_objective_owner_drain` check passed 21 tests.
The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 1413926ee --architecture --require-changes --timings --logs`
campaign passed all ten commands from 19:05:05Z to 19:07:40Z (154.859 seconds),
within the 600-second target: 4,642 `wow-world` tests (one ignored) and eight
`wow-world-quest` tests passed, totaling 4,650 passed, one ignored and zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T190505.619084Z-521084-final.json`.
Architecture policy, session syntax ownership, method-index, hotspot/census,
formatting and all affected downstream checks passed. The reviewed ledgers record
exactly 56 new Session test lines and one test identity, with no production
growth in that logical owner.

This closes only the immediate objective-status save divergence in #1368.
Completion effects remain open. This is synthetic source/behavior/wire evidence,
not a real database save/restart/relogin durability check, live quest capture or
full quest parity acceptance. Player runtime integration and character creation
remain separate work.

## #1368 inline quest completion accepted locally — 2026-11-20

Committed candidate `6a30657cd00949c6ad777c8f6a21b33253a23be2`, based on
`d59c27a4e`, completes eligible quests inside the objective loop instead of
deferring every completion until after it. Two small helpers drain the existing
completion queues, so each iteration drops the access before `await`, completes
and then recreates it. Storing-value/storing-flag and threshold paths all use the
inline order; their distinct completed-ID persistence behavior is preserved, as
is the final interaction refresh from `Player.cpp:17178-17194` at
`02245dcd245e7433e524577656177723d3e4992e`.

One composed two-quest regression runs in both storage families and requires
first credit, first completion refresh, second credit and final refresh in that
exact order, while the second quest remains incomplete. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base d59c27a4e --architecture --require-changes --timings --logs`
campaign passed all ten commands from 18:33:40Z to 18:36:22Z (162.045 seconds),
within the 600-second target: 4,641 `wow-world` tests (one ignored) and eight
`wow-world-quest` tests passed, totaling 4,649 passed, one ignored and zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T183340.016883Z-481797-final.json`.
Architecture policy, session syntax ownership, method-index, hotspot/census,
formatting and all affected downstream checks passed. The reviewed ledgers record
exactly 174 new Session test lines and one test identity, with no production
growth in that logical owner.

This closes only the inline completion-order divergence in #1368.
Immediate-save/persistence fences and completion effects remain open. This is
synthetic source/behavior/wire evidence, not a live quest capture or full quest
parity acceptance. Player runtime integration and character creation remain
separate work.

## #1368 effective-team PvP credit accepted locally — 2026-11-20

Committed candidate `6a2075da3b227e77a4c0ac4afe671a2747dad028`, based on
`0be460bd5`, replaces race-only comparison for `KILL_PLAYERS_SAME_FACTION` with
`Player::GetEffectiveTeam`. The shared rule maps race through the existing team
helper and swaps Alliance/Horde only for `PLAYER_FLAGS_EX_MERCENARY_MODE`
(`0x0002`). The victim's flags are resolved through `PlayerRegistry`'s canonical
Player owner, while the actor uses the selected canonical Player directly,
matching `Player.cpp:17044-17047` and `Player.h:GetEffectiveTeam` at
`02245dcd245e7433e524577656177723d3e4992e`. No Session identity projection or
second team authority is added.

One composed regression covers a mercenary victim, a mercenary actor and an
opposite-effective-team rejection: matching effective teams progress despite
different races, while different effective teams do not progress even with the
same race. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 0be460bd5 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 18:14:08Z to 18:17:08Z (180.156 seconds),
within the 600-second target: 4,640 `wow-world` tests (one ignored), 297
`wow-world-core` tests and eight `wow-world-quest` tests passed, totaling 4,945
passed, one ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T181408.754101Z-448688-final.json`.
Architecture policy, session syntax ownership, method-index, hotspot/census,
formatting and all affected downstream checks passed. An earlier campaign stopped
when registry projection added nine forbidden Session production lines; the
accepted candidate instead resolves those flags in the existing registry owner.

This closes only the effective-team divergence in #1368. Inline completion
ordering, immediate-save/persistence fences and completion effects remain open.
This is synthetic source/behavior/wire evidence, not a live quest capture or full
quest parity acceptance. Player runtime integration and character creation
remain separate work.

## #1368 raid objective admission accepted locally — 2026-11-20

Committed candidate `bcae7a3cc26007aa622ab730a72e7c6c59ffe582`, based on
`f4dc4da19`, enforces C++ raid admission across storing-value, storing-flag,
money, currency and reputation progress. `QuestTemplate` now owns the objective
exemption list and the `QuestInfoID`/difficulty/`QUEST_FLAGS_RAID_GROUP_OK`
decision, while `Quests.IgnoreRaid` is loaded through the production policy with
its default `false`. Current raid membership comes from the authoritative group
registry and map difficulty from the selected canonical Player, matching
`Player.cpp:17021-17028` and `QuestDef.cpp:IsRaidQuest/IsAllowedInRaid` at
`02245dcd245e7433e524577656177723d3e4992e`.

Two composed regressions cover both storage families rejected in raid,
RAID_GROUP_OK and non-raid completion, plus money regressing in raid without a
raid flag. A catalog matrix covers all quest-info/difficulty/config branches and
a world-server test covers the configuration key. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base f4dc4da19 --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 17:22:59Z to 17:35:12Z (732.631
seconds), exceeding the 600-second target by 132.631 seconds: 647
`world-server` (one ignored), 1,081 `wow-data` (six ignored), 4,639 `wow-world`
(one ignored), 297 `wow-world-core` and eight `wow-world-quest` tests passed,
totaling 6,672 passed, eight ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T172259.917639Z-333726-final.json`.
Architecture policy, session syntax ownership with 2,685 associated items and
752 direct-registry rows, formatting and all affected downstream checks passed.
Reviewed ledgers record the exact production/test growth, 105 new Session test
lines and two test identities.

This closes the raid-admission divergence in #1368. Inline completion ordering,
immediate-save/persistence fences, effective team and completion effects remain
open. This is synthetic source/behavior/wire evidence, not a live quest capture
or full quest parity acceptance. Player runtime integration and character
creation remain separate work.

## #1368 threshold negative transitions accepted locally — 2026-11-20

Committed candidate `94460fac95e0f8eb15e8faf4b6f5b3e1d29a543b`, based on
`2ef7286b8`, completes negative completion-state handling for money, currency and
reputation objectives. The shared threshold planner now carries objective flags;
all three consumers preserve quest completion for optional regressions, invoke
`IncompleteQuest` for required ones and run the end-of-call interaction refresh
on every completion-state change. Following `Player.cpp:17161-17194` at
`02245dcd245e7433e524577656177723d3e4992e`, positive transitions retain both the
`CompleteQuest` refresh and the final refresh.

Three composed canonical-Player regressions reduce each resource from 100 to 90:
required objectives mark the quest incomplete, optional objectives preserve its
complete status and both publish exactly one visibility update without credit or
reward. Existing positive-money expectations now prove two updates. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 2ef7286b8 --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 16:37:13Z to 16:46:04Z (531.483
seconds), within the 600-second target: 1,309 `wow-entities` tests, 4,637
`wow-world` tests (one ignored) and eight `wow-world-quest` tests passed,
totaling 5,954 passed, one ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T163713.022551Z-235496-final.json`.
Architecture policy, session syntax ownership, formatting and all affected
downstream checks passed. The reviewed ledgers record exactly 205 new Session
test lines and three test identities, with no production growth in that logical
owner. Initial fixture-only failures installed the missing item GUID generator
and canonical reputation owner before the accepted run; no assertion or required
suite was narrowed.

This closes the threshold negative-transition divergence in #1368. Inline
completion ordering, raid admission, immediate-save/persistence fences, effective
team and completion effects remain open. This is synthetic source/behavior/wire
evidence, not a live quest capture or full quest parity acceptance. Player
runtime integration and character creation remain separate work.

## #1368 completed storing-flag repeat guard accepted locally — 2026-11-20

Committed candidate `8b5358e93e224260f5eb3d3005819c5dee502b11`, based on
`6cc386e83`, skips nonnegative progress events for an already complete
storing-flag objective before calling its mutator. This applies the existing C++
guard at `Player.cpp:17034-17036` (`02245dcd245e7433e524577656177723d3e4992e`)
to the flag path, matching the guard already used by storing-value objectives.
Repeated events cannot duplicate credit, completion or visibility publication.

One composed canonical-Player regression covers both `addCount = 0` and
`addCount = 1`, requiring unchanged status/count and no packets. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 6cc386e83 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 16:01:31Z to 16:03:54Z (142.459 seconds),
within the 600-second target: 4,634 `wow-world` tests (one ignored) and eight
`wow-world-quest` tests passed, totaling 4,642 passed, one ignored and zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T160131.908571Z-166449-final.json`.
Architecture policy, session syntax ownership, formatting and all affected
downstream checks passed. The reviewed ledgers record exactly 20 new Session test
lines and one test identity, with no production growth in that logical owner.

This closes only the completed storing-flag repeat divergence in #1368. Inline
completion ordering, raid admission, other objective families' negative
transitions, immediate-save/persistence fences, effective team and completion
effects remain open. This is synthetic source/behavior/wire evidence, not a live
quest capture or full quest parity acceptance. Player runtime integration and
character creation remain separate work.

## #1368 objective completability accepted locally — 2026-11-20

Committed candidate `e8c5a97c367b16484b6becd94c3ed4497dadae0b`, based on
`881edb4bc`, applies the existing represented `IsQuestObjectiveCompletable` rule
to storing-value and storing-flag objective selection. Following
`Player.cpp:17025-17039,17376-17431` at
`02245dcd245e7433e524577656177723d3e4992e`, a sequenced objective now waits for
its prerequisite before any counter mutation, credit, completion or visibility
publication. The shared rule retains its optional/progress-bar handling and
recursive sequence checks; no second sequence authority was added.

Two composed canonical-Player regressions cover both storage families with an
incomplete prerequisite (unchanged status/count and no packets) and a completed
prerequisite (credit, completion and both interaction refreshes). The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 881edb4bc --architecture --require-changes --timings --logs`
campaign passed all ten commands from 15:50:54Z to 15:54:24Z (209.193 seconds),
within the 600-second target: 4,633 `wow-world` tests (one ignored) and eight
`wow-world-quest` tests passed, totaling 4,641 passed, one ignored and zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T155054.902050Z-152561-final.json`.
Architecture policy, session syntax ownership, formatting and all affected
downstream checks passed. The reviewed ledgers record exactly 73 new Session test
lines and two test identities, with no production growth in that logical owner.

This closes only the represented completability/sequencing divergence in #1368.
Inline completion ordering, raid admission, other objective families' negative
transitions, immediate-save/persistence fences, effective team and completion
effects remain open. This is synthetic source/behavior/wire evidence, not a live
quest capture or full quest parity acceptance. Player runtime integration and
character creation remain separate work.

## #1368 NO_CREDIT_FOR_PROXY admission accepted locally — 2026-11-20

Committed candidate `01eced0fbf98cd3635055a8c8eb466f52710f6a7`, based on
`bee14fd01`, applies `QUEST_FLAGS_EX_NO_CREDIT_FOR_PROXY` (`0x4000`) before any
storing-value objective mutation or publication. Following
`Player.cpp:17025-17039` at `02245dcd245e7433e524577656177723d3e4992e`, a
`MONSTER` objective with that quest flag now ignores an empty victim GUID. A real
victim GUID remains eligible, and an empty GUID still progresses when the quest
does not carry the flag.

Two composed canonical-Player regressions prove the rejected proxy preserves its
incomplete status and zero count without packets, while both admitted controls
complete with credit and the expected two visibility refreshes. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base bee14fd01 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 15:34:39Z to 15:39:26Z (287.877 seconds),
within the 600-second target: seven `wow-constants` tests, 4,631 `wow-world`
tests (one ignored) and eight `wow-world-quest` tests passed, totaling 4,646
passed, one ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T153439.013291Z-135005-final.json`.
Architecture policy, session syntax ownership, formatting and all affected
downstream checks passed. The reviewed ledgers record exactly 75 new Session test
lines and two test identities, with no production growth in that logical owner.

This closes only the NO_CREDIT_FOR_PROXY divergence in #1368. Inline objective
ordering, raid/completability admission, other objective families' negative
transitions, immediate-save/persistence fences, effective team and completion
effects remain open. This is synthetic source/behavior/wire evidence, not a live
quest capture or full quest parity acceptance. Player runtime integration and
character creation remain separate work.

## #1368 HIDE_CREDIT_MSG publication accepted locally — 2026-11-20

Committed candidate `3f951bb6cda4367f2e526438bf52cecb4a254f86`, based on
`11c4baa3b`, applies `QUEST_OBJECTIVE_FLAG_HIDE_CREDIT_MSG` (`0x0010`) to
storing-value and storing-flag objective progress. Following
`Player.cpp:17049-17079` at `02245dcd245e7433e524577656177723d3e4992e`, positive
progress no longer publishes the corresponding credit packet when that flag is
present. Objective counts, quest completion, rewards and interaction refreshes
are unchanged.

Two composed canonical-Player regressions prove that both storage families still
complete and run their two visibility refreshes while `QuestUpdateAddCredit` or
`QuestUpdateAddCreditSimple` is suppressed; neither `QuestUpdateComplete` nor a
reward is invented. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 11c4baa3b --architecture --require-changes --timings --logs`
campaign passed all ten commands from 15:20:41Z to 15:25:49Z (308.629 seconds),
within the 600-second target: seven `wow-constants` tests, 4,629 `wow-world`
tests (one ignored) and eight `wow-world-quest` tests passed, totaling 4,644
passed, one ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T152041.242691Z-109152-final.json`.
Architecture policy, session syntax ownership, formatting and all affected
downstream checks passed. The reviewed ledgers record exactly 46 new Session test
lines and two test identities, with no production growth in that logical owner.

This closes only the HIDE_CREDIT_MSG divergence in #1368. Inline objective
ordering, the remaining admission filters, other objective families' negative
transitions, immediate-save/persistence fences, effective team and completion
effects remain open. This is synthetic source/behavior/wire evidence, not a live
quest capture or full quest parity acceptance. Player runtime integration and
character creation remain separate work.

## #1368 objective completion transitions accepted locally — 2026-11-20

Committed candidate `f908ef40e2e5cfcbe66109a8cf393d72380957cf`, based on
`787e7d77a`, removes the remaining item-added `QuestUpdateComplete` emissions and
implements completion-state transitions for storing-value and storing-flag
objectives. Following `Player.cpp:16980-17194` at
`02245dcd245e7433e524577656177723d3e4992e`, both families now visit active
`INCOMPLETE` and `COMPLETE` quests, call the existing `IncompleteQuest` owner when
a required objective regresses, preserve quest completion for an optional
objective and refresh visible interactions after every objective completion-state
change. A positive transition retains the `CompleteQuest` refresh and the
end-of-call refresh; `AreaExploredOrEventHappens` remains the only
`QuestUpdateComplete` emitter.

Six composed canonical-Player regressions cover positive, negative and
optional-negative transitions for both storage families. They prove final status
and count, positive-only credit, no `QuestUpdateComplete` or reward, and the exact
one/two visibility-refresh counts. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 787e7d77a --architecture --require-changes --timings --logs`
campaign passed all ten commands from 15:08:42Z to 15:11:38Z (176.316 seconds),
within the 600-second target: 4,627 `wow-world` tests (one ignored), 297
`wow-world-core` tests and eight `wow-world-quest` tests passed, totaling 4,932
passed, one ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T150842.176658Z-86174-final.json`.
Architecture policy, session syntax ownership with 2,684 exact associated items,
formatting and all affected downstream checks passed. The reviewed ledgers record
exactly 287 new Session test lines and six test identities; production remains
unchanged in that logical owner. Earlier campaigns stopped on those unrecorded
hotspot and census deltas before running the library suites, then were rerun with
the reviewed records rather than narrowing acceptance.

This closes only item-added packet duplication and storing-value/storing-flag
completion transitions in #1368. Inline objective ordering and filters, the other
objective families' negative transitions, immediate-save/persistence fences,
effective team and completion effects remain open. This is synthetic
source/behavior/wire evidence, not a live quest capture or full quest parity
acceptance. Player runtime integration and character creation remain separate
work.

## #1368 QuestUpdateComplete publication corrected locally — 2026-11-20

Committed candidate `9e3bf55cc46efe29dc3e88c00ca44dc20d0ce532`, based on
`0b1634767`, removes `QuestUpdateComplete` from all five objective-progress
families and the tracking-event reward coordinator. Following
`Player.cpp:15169-15195,16795-16813,16980-17194` at
`02245dcd245e7433e524577656177723d3e4992e`, that packet is now emitted only by
`AreaExploredOrEventHappens`. Ordinary objective completion and auto-reward still
run their existing status, reward and visibility phases; `QuestGiverQuestComplete`
and the area-explored packet remain distinct.

All 4,621 `wow-world` library tests pass with one ignored, including the
objective/reward expectations updated to this contract and the preserved
`AreaExplored` case. Eight `wow-world-quest` tests also pass with zero failures.
The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 0b1634767 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 14:35:19Z to 14:37:55Z (155.128
seconds), within the 600-second target: 4,629 passed, one ignored and zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T143519.979752Z-53871-final.json`.
Architecture policy, session syntax ownership with 2,684 exact associated items,
formatting and all affected downstream checks passed.

This closes only the packet-publication divergence of #1368. Inline objective
ordering and filters, negative-completion transitions, refresh/persistence
fences, effective team and completion effects remain open in that issue. This is
synthetic source/behavior/wire evidence, not a live quest capture or full quest
parity acceptance. Player runtime integration and character creation remain
separate work.

## #1367 LootList fanout unified and accepted locally — 2026-11-20

Committed candidate `d9c25cadefb1f18f9e453d5c7bd8471e29308d98`, based on
`0f70f7985`, retires the shell's duplicate `NotifyLootList` implementation onto
the existing loot-release owner. Following `Loot.cpp:736-754` at
`02245dcd245e7433e524577656177723d3e4992e`, the owner now sends the typed packet
to the current session unconditionally and the registry fan-out excludes that
same current player rather than the loot object's GUID. The old group-only gate
is removed, so every connected `_allowedLooters` member on the map receives the
same bytes on open and round-robin release.

One ungrouped round-robin regression proves local and remote delivery on open
and release, including the winner being cleared. Existing master-loot and AE
expectations now include the mandatory `LootList` frames, and their bounded
packet queues hold the real response/list/ACK sequence instead of deadlocking.
The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 0f70f7985 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 13:48:01Z to 13:50:53Z (171.554
seconds), within the 600-second target. Suites passed 4,621 `wow-world` tests
(one ignored), 297 `wow-world-core` and seven `wow-world-loot-ops`: 4,925
passed, one ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T134801.899415Z-8677-final.json`.
Architecture policy, session syntax ownership with 2,684 exact associated items
and 747 direct-registry rows, formatting and all affected downstream checks
also passed.

The first campaign stopped on two obsolete shell-registry baseline rows; their
retirement was recorded before the full rerun. This is synthetic
source/behavior/wire evidence, not a live multi-client loot capture or a new
whole-port loot audit. Player runtime integration and character creation remain
separate work.

## #1473 creature melee damage school accepted locally — 2026-11-20

Committed candidate `6cfb90d6420e6bb2bfc3535c69c6f8371bbdf37e`, based on
`4a5fd2498`, propagates the creature's `melee_damage_school_mask` through the
complete represented white-swing path. Following `Creature.cpp:657,2321` and
`Unit.cpp:1335-1534,1683-1690,8045-8155` at
`02245dcd245e7433e524577656177723d3e4992e`, one captured attacker fact now
selects immunity, attacker/victim school modifiers, critical bonuses, absorb,
split/share gates and `AttackerStateUpdate` bytes for both Player and Creature
victims. Non-physical swings skip armor as C++ requires; physical swings keep
their existing mitigation and threat modifier behavior. The typed mailbox
command carries the same mask to session publication, including native-death
prefixes and the admitted-execution compatibility path.

A Fire Elemental regression (entry 575, mask `0x04`) combines physical immunity,
+10% attacker Fire damage, +5 Fire taken and four points of Fire absorb. It
proves 12 resulting damage without armor and decodes the Fire mask from the
serialized attacker-state packet. Existing command fixtures and the recorded
F6-8D3a expectations explicitly carry physical mask 1. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 4a5fd2498 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 13:13:59Z to 13:16:16Z (137.043
seconds), within the 600-second target. Suites passed 646 `world-server`
(one ignored), 4,620 `wow-world` (one ignored), 297 `wow-world-core`, 37
`wow-world-creature`, two `wow-world-entities` and 193 `wow-world-spell`
tests: 5,795 passed, two ignored and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T131359.379046Z-4171610-final.json`.
The manifest records the clean candidate SHA above. Architecture policy,
session syntax ownership with 2,684 exact associated items, formatting and
all affected downstream checks also passed.

Earlier local attempts stopped on a missing ownership-baseline field, one
300-second runner timeout and five recorded Debug expectations that lacked the
new physical mask; those inputs were corrected without narrowing the final
campaign. This is synthetic source/behavior/wire evidence, not a live Fire
Elemental combat capture or a new whole-port combat audit. Player runtime
integration and character creation remain separate work.

## #1487 starting loadout accepted locally — 2026-11-20

Committed candidate `f9c3614878c2e39e73216a97ac7ab86c758a0b36`, based on
`0b877b4bd`, projects the initial character loadout and persists its item
instances, inventory rows and 34-slot `equipmentCache`. The creation path
selects purpose-9 DB2 loadouts by class and race mask, applies the positive and
negative `playercreateinfo_item` overrides, allocates unique item GUIDs and
places equipment, equipped bags and backpack items without duplicating slots.
Each projected row retains its count and maximum durability; equipped entries
publish inventory type, display ID, subclass and the remaining visual fields in
the same five-integer-per-slot cache format consumed by character enumeration.

A combined regression covers shirt, weapon, backpack, equipped bag, negative
weapon removal and positive weapon readmission, including complete cache
contents, quantities, durability, slots and GUID uniqueness. The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base 0b877b4bd --architecture --require-changes --timings --logs`
campaign passed all thirteen commands from 12:02:00Z to 12:12:33Z (633.299
seconds), exceeding the 600-second target by 33.299 seconds. Suites passed 646
`world-server`, 1,080 `wow-data`, 632 `wow-database` (nine ignored), 36
`wow-persistence`, 4,619 `wow-world` (one ignored), four
`wow-world-character` and 297 `wow-world-core` tests: 7,314 passed, 16 ignored
and zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T120200.334569Z-4094830-final.json`.
The manifest records the clean candidate SHA above; architecture policy,
session syntax ownership, formatting, downstream checks and SQL composition
also passed.

This is synthetic catalog/persistence/wire evidence, not real MariaDB
durability or a live character-create/select capture. The remaining
`CharacterCreator` phases and Player runtime integration stay open separately.

## #1486 character-enum stored customizations accepted locally — 2026-11-20

Committed candidate `b8e79d342c598be917928a2542487fffc42bf94e`, based on
`d318a15e2`, publishes stored character customizations in the enum response.
Following `CharacterHandler.cpp:391–455` and `CharacterPackets.cpp:219–245` at
`02245dcd245e7433e524577656177723d3e4992e`, the persistence adapter now runs
`SEL_ENUM_CUSTOMIZATIONS` after the character query and decodes option/choice
IDs as `u32`. The typed outcome carries those rows, the handler groups them by
character GUID, and `CharacterInfo::Write` emits their count and `int32` pairs
immediately after `SexID`. Missing rows remain an empty list; query failure
still returns the bounded failure response.

A packet regression proves both pairs and the following level field, while the
real handler regression publishes both stored pairs through the typed port.
The committed
`RUSTYCORE_CPP_REFERENCE_ROOT=target/publication-20261111/legacy-reference ./tools/validation-v2 final --base d318a15e2 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 11:08:51Z to 11:10:45Z (114.236 seconds),
within the 600-second target. Suites passed 632 `wow-database` tests (nine
ignored), 820 `wow-packet`, 36 `wow-persistence`, 4,619 `wow-world` (one ignored)
and all three `wow-world-character` tests, with zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T110851.486850Z-4053726-final.json`.
The character-owner ledger records exactly 44 new test lines and no production
growth.

Earlier attempts failed because the physical C++ files were empty and then
because the Forever SQL reference did not match the 3.4.3 statement contracts.
The accepted run uses the preserved legacy reference instead; no failing test
was removed or weakened. This is synthetic source/persistence/wire evidence,
not real MariaDB durability or a live character-select capture. Character
creation and Player runtime integration remain separate work.

## #1495 Forever InspectResult layout accepted locally — 2026-11-20

Committed candidate `1c13e72c446614f90da2dd9c079950ec7463c444`, based on
`e3cf737e2`, aligns `SMSG_INSPECT_RESULT` with the complete Forever writer in
`InspectPackets.cpp:96–150,216–250` and `ItemPacketsCommon.cpp:151–188` at
`02245dcd245e7433e524577656177723d3e4992e`. The serializer now places item
count after display data, writes ItemInstance before item index and Azerite
counts, retains both talent counts plus the four PvP slots, emits the empty
`ClassicTalentInfoUpdate`, uses nine PvP brackets with their exact field types,
and orders honor and trait data before the glyph/talent arrays and optional
guild/Azerite bits. It no longer invents random-property fields inside
ItemInstance.

One binary regression walks every field through one item, the classic talent
block, all brackets, honor, traits and optional flags, then requires complete
packet consumption. The committed
`./tools/validation-v2 final --base e3cf737e2 --architecture --require-changes --timings --logs`
campaign passed all eight commands from 10:34:11Z to 10:36:57Z (165.691
seconds), within the 600-second target. All 819 `wow-packet` tests passed with
zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T103411.935031Z-4011244-final.json`.
The initial binary attempt exposed the ItemModList width/order mismatch; the
corrected writer and reader-aware bit resets pass without offset-based masking.

This intentionally does not implement #1495's 3.4.3-only layout: Forever keeps
`Talents.Count`, adds `PvpTalents.Count`, and uses the different ordering above.
That issue remains open for its own version. No Inspect capture-diff flow or
live 1.60.1 client capture exists here, so this is synthetic source/wire
evidence rather than live acceptance. Player runtime integration and character
creation remain separate work.

## #1492 quest-accept close gossip accepted locally — 2026-11-20

Committed candidate `57ddce2e329711bdf5c12065c91f0de25bee8737`, based on
`9cf6270fa`, replaces the unconditional `QuestGiverQuestComplete` after a
successful quest accept with the C++ close-gossip transition. Forever
`QuestHandler.cpp:89–177` calls `PlayerMenu::SendCloseGossip` after
`AddQuestAndCheckCompletion`; `GossipDef.cpp:292–298` resets interaction data
and replies `GossipComplete`. The quest host now exposes that existing
`InteractionState` operation through its narrow trait and the accept handler
calls it after the established durability and hook phases. An immediate reward
still emits its own `QuestGiverQuestComplete` through the committed reward path;
failure paths retain their existing bounded behavior.

The accept regression now requires `GossipComplete` before the quest-log map
update, and the hook-order regression requires Chat, close gossip, then the map
update. The committed
`./tools/validation-v2 final --base 9cf6270fa --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 10:19:44Z to 10:22:01Z (136.607
seconds), within the 600-second target. The `wow-world` suite passed 4,618
tests with one existing ignored and `wow-world-quest` passed all eight tests,
with zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T101944.487460Z-3996088-final.json`.
The exact WorldSession method-surface entry, four-line quest-owner delta and
54-method census are reviewed. Earlier attempts stopped on those missing
metadata entries and were not accepted as final evidence.

Related #1490 was not changed: Forever `HandleQuestgiverHelloOpcode` calls
`PrepareQuestMenu`/`SendPreparedQuest`, whose empty-menu branch returns without
a packet (`Player.cpp:14669–14699`). Its proposed always-reply behavior belongs
to the 3.4.3 issue, not this target. No live-client capture is claimed for this
synthetic source/wire acceptance. Player runtime integration and character
creation remain separate work.

## #1354 live Pet alternate-minion retention accepted locally — 2026-11-20

Committed candidate `5fb622307fde36e1a07b15bfb6886ce8c66fc87a`, based on
`a66052059`, extends bounded live `Player::RemovePet` to owners with an
alternate current minion. Forever `Unit::SetMinion` (`Unit.cpp:6271–6440` at
`02245dcd245e7433e524577656177723d3e4992e`) erases only the removed Pet when
it is not the current minion. The Rust admission now permits that case and the
existing cleanup removes only the selected Pet's controlled identity. An
inconsistent alternate identity, or a selected current minion with another
controlled unit that could require guardian promotion and spell-bar
initialization, still fails before mutation.

One real release regression proves both saves and corpse creation while the
alternate GUID remains current and controlled, and the removed Pet disappears
from those sets. The negative scenario proves no writes, clear-spells packet or
identity mutation when promotion would be required. The committed
`./tools/validation-v2 final --base a66052059 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 09:55:20Z to 09:58:41Z (201.921 seconds),
within the 600-second target. The `wow-world` suite passed 4,618 tests with one
existing ignored and `wow-world-application` passed all 32 tests, with zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T095520.056239Z-3974055-final.json`.
No audited logical production owner grew; the two-test census increase is
reviewed under #1354.

This is synthetic source/lifecycle evidence, not a live-client capture.
Selected-minion promotion, Pet-owned control cleanup, combat continuations and
arbitrary runtime auras without persisted provenance remain unadmitted. #1354
remains open; Player runtime integration and character creation remain separate
work.

## #1354 live Pet group-update flag accepted locally — 2026-11-20

Committed candidate `5db0b73b6828445c0f9d5901059ec3a920383780`, based on
`2afd2e26b`, admits grouped owners in the bounded live `Player::RemovePet`
operation. `PlayerGameplayState` now owns the C++ `m_groupUpdateMask` beside its
existing membership and update-sequence state. Following `Player.cpp:22322–22408`
and `Player.h:2832–2834` at
`02245dcd245e7433e524577656177723d3e4992e`, the operation evaluates current
membership after `ClearPetSpells` and ORs `GROUP_UPDATE_FLAG_PET` without
discarding existing bits. No map lock is held during packet delivery, and group
membership is deliberately re-evaluated at the C++ point rather than frozen at
admission.

The real release regression proves both ordered saves, corpse creation, pet
detachment, one clear-spells packet and preservation of an unrelated mask bit;
the existing ungrouped scenario now also proves a zero final mask. The committed
`./tools/validation-v2 final --base 2afd2e26b --architecture --require-changes --timings --logs`
campaign passed all ten commands from 09:46:10Z to 09:47:54Z (104.310 seconds),
within the 600-second target. Suites passed 1,309 `wow-entities` tests, 4,616
`wow-world` tests (one existing ignored) and all 32 `wow-world-application`
tests, with zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T094610.537477Z-3961526-final.json`.
The Player ownership ledger records the measured nine-line production delta;
its initial textual estimate was corrected before this accepted run.

This proves canonical flag state, not a live group-update publication or client
capture. Arbitrary runtime auras without persisted provenance, secondary-minion
and combat/control continuations remain unadmitted and fail closed. #1354
remains open; Player runtime integration and character creation remain separate
work.

## #1354 live Pet persisted-aura removal accepted locally — 2026-11-20

Committed candidate `5323e5e3779f30225635173f31fd0a0bb3876699`, based on
`69d9e0ad7`, extends the bounded live `Player::RemovePet` path to persisted
native auras. Following Forever `Player.cpp:22322–22408`, `Pet.cpp:452–546,
1301–1345` and `Unit.cpp:4401–4436` at
`02245dcd245e7433e524577656177723d3e4992e`, the save projection orders auras
by their C++ key, clears a self-cast Pet GUID, and captures exact aura/effect
rows before native cleanup. Only auras with complete loaded provenance and
matching durable input are admitted; unknown, divergent or unsupported runtime
state fails before mutation. Cleanup removes applied, owned, visible and
per-effect amount state after CombatStop and before the first persistence
await. Both ordered saves and every later revalidation retain the existing
cancellation and detachment fences.

An application regression covers two auras, C++ ordering, self-caster clearing,
effect amounts and complete container cleanup. Two real release scenarios prove
exact persisted rows followed by cleanup/detachment and reject a divergent
loaded duration without writes or aura mutation. The committed
`./tools/validation-v2 final --base 69d9e0ad7 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 09:27:09Z to 09:28:53Z (104.775 seconds),
within the 600-second target. Suites passed 1,309 `wow-entities` tests, 4,615
`wow-world` tests (one existing ignored) and all 32 `wow-world-application`
tests, with zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T092709.216221Z-3931842-final.json`.
An intermediate Session accounting misattribution was corrected and the complete
campaign rerun; no production behavior changed during that metadata correction.

This is synthetic source/persistence evidence, not real database durability or
a live-client capture. Arbitrary runtime auras without persisted provenance,
group, secondary-minion and combat/control continuations remain unadmitted and
fail closed. #1354 remains open; Player runtime integration and character
creation remain separate work.

## #1357 AE loot candidate phase and combat reach accepted locally — 2026-11-20

Committed candidate `b2196c0108e6f90071002bb9165fa7c17edb582f`, based on
`b1109ee11`, aligns the bounded AE loot candidate query with the target C++
visitor. It snapshots the canonical Player's phase and combat reach, fails
closed when that Player is unavailable, visits cells within `radius + viewer
reach`, and admits only dead creatures visible in that phase within `radius +
viewer reach + creature reach`. The caller retains its existing main-target and
loot-permission checks.

The new regression admits a 32-yard corpse, rejects a 33-yard corpse and a
corpse in another phase, and excludes a living target without relying on cell
iteration order. The original candidate case and four existing AE integration
scenarios pass; those integration fixtures now install and adopt their canonical
Player explicitly. The committed
`./tools/validation-v2 final --base b1109ee11 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 08:43:28Z to 08:45:27Z (118.708 seconds),
within the 600-second target. The `wow-world` suite passed 4,613 tests with one
existing ignored and `wow-world-core` passed all 297 tests, with zero failures.
Verified manifest:
`target/validation-v2/manifests/20261120T084328.358306Z-3878681-final.json`.

The first final attempt exposed four AE fixtures without a canonical Player;
only their fixture setup and the order-independent candidate assertion changed
before the green campaign, with the exact test-ceiling reviews updated. That
failed attempt is not behavioral evidence. `CONFIG_ENABLE_AE_LOOT` remains
disabled by default; this is synthetic source evidence, not a live-client
capture or a global #1357 closeout. Player runtime integration and character
creation remain separate work.

## #584 Unit VALUES Empath visibility accepted locally — 2026-11-20

Committed candidate `d9ff480fca8100ef8b7c1e62e6466718859707b9`, based on
`45fa2c4a2`, implements the target's remaining Empath receiver branch for Unit
VALUES. Matching `Unit::GetUpdateFieldFlagsFor`, a Creature or Pet exposes its
Empath-only fields only when UNIT_DYNFLAG_SPECIALINFO (0x20) is set and its
SPELL_AURA_EMPATHY (121) was cast by the receiver. Owner filtering, public field
selection and PartyMember behavior are unchanged; an update with no visible
fields emits no UnitData section.

A focused regression covers no flag, flag without the viewer aura, an unrelated
caster, the qualifying viewer aura, and flag removal. It also proves that field
190 survives only with Empath while an unrelated owner-only field is removed.
The committed `./tools/validation-v2 final --base 45fa2c4a2 --architecture --require-changes --timings --logs`
campaign passed all nine commands from 08:01:34Z to 08:04:25Z (170.560 seconds),
within the 600-second target. All four `wow-world-visibility` tests passed with
zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T080134.908541Z-3843642-final.json`.
The earlier attempt stopped only on rustfmt after fixture repairs and was not
used as behavioral evidence. No production dependency or policy changed.

The target's `Unit::GetUpdateFieldFlagsFor` has no UNIT_ALL branch, so no such
visibility is invented. This is synthetic source evidence, not a live capture.
Remaining lifecycle work, character creation and their required acceptance stay
open under #584 and the separate Forever objective.

## #584 Player CREATE PartyMember visibility accepted locally — 2026-11-20

Committed candidate `988aeebd9e331678478710468e34b501643e8a2e`, based on
`d860f59d3`, completes the receiver-specific PartyMember half of Player CREATE.
Following `Player::GetUpdateFieldFlagsFor`, canonical create snapshots compare
source and viewer raid GUIDs and carry the result through every existing
projection constructor into UpdateBlock. Self remains Owner|PartyMember (0x03),
another member of the same raid receives PartyMember (0x02), and other receivers
remain 0x00. Existing VALUES receiver filtering is unchanged. QuestLog rows are
serialized only when that visibility is present; ActivePlayerData remains
self-only.

A direct packet regression pins all three flags and the 25 quest rows, while a
canonical-registry regression proves same-raid inclusion and different-raid
rejection through the real snapshot-to-CREATE path. The committed
`./tools/validation-v2 final --base d860f59d3 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 07:38:38Z to 07:43:05Z (266.771 seconds),
within the 600-second target. Suites passed 818 `wow-packet` tests, 4,612
`wow-world` tests (one existing ignored), and all 297 `wow-world-core` and 14
`wow-world-lifecycle` tests, with zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T073838.418008Z-3822421-final.json`.
The one-test census raise is reviewed under #584; no production hotspot grew.

This is synthetic source/wire evidence, not a live client capture. UNIT_ALL,
EMPATH, remaining lifecycle work and their exact capture requirements remain
open under #584. Character creation and the separate Pet lifecycle gaps are not
closed by this checkpoint.

## #1354 live Pet summon-reagent return accepted locally — 2026-11-20

Committed candidate `60a7bc3b61d4a23d33b24176fcf186c50a38f47b`, based on
`207d5a11b`, extends the bounded live Player::RemovePet operation to summon
spells. The captured removal input now freezes CreatedBySpell and the actual
nonzero battleground-instance state. Outside map guards and before CombatStop
or either pet-save transaction, the existing inventory/quest operation returns
reagents under the same cancellation fence. The complete pet/save/BG input is
revalidated after callbacks and after every save. Battleground removal skips
the refund and preserves the temporary-unsummoned marker; ordinary removal
clears it. The saved pet row retains CreatedBySpell.

Two regressions prove refund-before-ClearPetSpells and saved spell identity,
plus nonzero-BG refund suppression and marker preservation. Existing reagent
(six) and live-release (five) scenarios also pass. The committed
`./tools/validation-v2 final --base 207d5a11b --architecture --require-changes --timings --logs`
campaign passed all ten commands from 07:05:31Z to 07:08:04Z (152.973 seconds),
within the 600-second target. The `wow-world` suite passed 4,611 tests with one
existing ignored and `wow-world-application` passed all 31 tests, with zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T070531.612638Z-3793839-final.json`.

The first BG=0 fixture failed because zero is the correct no-battleground
sentinel; changing only the fixture to instance 31 exposed the intended branch.
The first final campaign then stopped on the quest hotspot's unreviewed +232
test lines, which were reconciled alongside the two-test census raise. No failed
attempt is claimed as behavioral evidence. Recording ports do not establish real
DB durability, restart or live-client behavior.

Group, secondary-minion, non-empty-aura and combat/control continuations remain
unadmitted and fail closed. #1354 therefore remains open rather than claiming
full RemovePet parity. Player runtime integration and character creation remain
separate work.

## #1354 unloaded-move Pet removal accepted locally — 2026-11-20

Committed candidate `53e848cafd09a565f462084cabb4c83a6f00a3ab`, based on
`a680f365c`, connects the unloaded-grid Pet fallback to the existing bounded
Player::RemovePet operation. Map now emits exact pet/owner GUIDs, the canonical
manager transfers each request once, and world-server attaches map/instance
identity before publishing through the owner's durable mailbox outside map
locks. The session ignores stale pet or residence identities, then revalidates
and executes the two ordered pet-save transactions before clearing the active
pet/stable/minion links, queueing remove-list and publishing empty pet spells.
Execution or retention failure disconnects rather than silently dropping the
obligation. No SQL, persistence await or session call occurs under a map guard.

Five new regressions plus the strengthened existing Map case cover exact request
identity, one-shot handoff, current/wrong/missing-owner delivery, ordered saves
and detachment, stale-command refusal, and disconnect on save failure. The
committed `./tools/validation-v2 final --base a680f365c --architecture --require-changes --timings --logs`
campaign passed all twelve commands from 06:34:08Z to 06:42:58Z (529.872
seconds), within the 600-second target. Suites passed 646 `world-server` tests,
788 `wow-map` tests and 4,609 `wow-world` tests (one existing ignored in each),
plus all 31 `wow-world-application` and 297 `wow-world-core` tests, with zero
failures. Verified manifest:
`target/validation-v2/manifests/20261120T063408.850088Z-3759666-final.json`.
Architecture retains 43 exact command variants and 749 direct-registry rows;
the three-test census increase is explicitly reviewed under #1354.

Earlier attempts stopped without acceptance: one lacked pinned `protoc` (exit
64), then command/policy metadata was incomplete, its canonical order was wrong,
and the new tests were not censused. Those inputs were corrected before the
green candidate; none is presented as behavioral evidence. The recording port
proves operation order, not real database durability, restart or live-client
behavior.

The live operation still admits only quiescent pets with complete empty aura
input and no summon-spell, group, secondary-minion or combat/control
continuation. Other pets fail and disconnect instead of completing full C++
RemovePet behavior. Consequently #1354 remains open for those lifecycle gaps;
Player runtime integration and character creation remain separate work.

## #1354 direct and lent Creature respawn relocation accepted locally — 2026-11-20

Committed candidate `f471c9aee88fcd93f4caae1bcf32ebce87477873`, based on
`1e570cf20`, extends the production respawn fallback to direct in-place and
lent-return relocation. Both entry points now report the requested relocation
and an optional fallback separately. Successful requested moves and invalid
coordinates need no fallback; an unloaded destination preserves the original
non-relocated result while attempting the creature's actual home with
combat/motion cleanup and direct reinitialization on success. A normal Creature
queues remove-list when home is also blocked. Origin and actual-home observers
are marked through the deferred rail, never the refused destination, and the
session consumer no longer logs a fallback report as remaining at origin.

New direct and lent regressions cover requested relocation, blocked requests,
home success and origin/home/refused visibility. The committed
`./tools/validation-v2 final --base 1e570cf20 --architecture --require-changes --timings --logs`
campaign passed all ten commands from 05:31:21Z to 05:35:30Z (249.207 seconds),
within the 600-second target. Library suites passed 787 `wow-map` tests (one
existing ignored), 4,606 `wow-world` tests (one existing ignored) and all 297
`wow-world-core` tests, with zero failures. Verified manifest:
`target/validation-v2/manifests/20261120T053121.260976Z-3706433-final.json`.

A Pet whose requested move and home are both blocked still reports only
`PetRemoveRequested`: no canonical deletion, remove-list insertion, SQL or
persistence operation is invented. The equivalent Pet::Remove lifecycle remains
open, so #1354 is not closed by this checkpoint. Player runtime integration and
character creation remain separate work.

## #1354 production Creature respawn relocation accepted locally — 2026-11-20

Committed candidate `f75a9578cbc2596380d00fa5299b9b6a91ab21aa`, based on
`9594502da`, implements the production move-list fallback for #1354. A valid
move to an unloaded destination now tries the creature's actual home position
with combat/motion cleanup and direct reinitialization on success. If home also
fails, a normal Creature queues remove-list; a Pet records only a removal
request, without deleting the canonical record, invoking persistence or claiming
an unimplemented Pet::Remove. Refused destinations are never marked as successful
walks; origin and actual-home observers use the existing deferred rail.

Seven new tests cover home success, Creature removal, Pet request-only and
invalid coordinates, including origin/home/refused visibility. The committed
`./tools/validation-v2 final --base 9594502da --architecture --require-changes --timings --logs`
campaign passed all nine commands from 05:07:42Z to 05:11:29Z (228.043 seconds),
within the 600-second target. The `wow-map` library passed 785 tests with zero
failures and one existing ignored test. Verified manifest:
`target/validation-v2/manifests/20261120T050741.776808Z-3685572-final.json`.

Direct in-place and lent-return still snap back after MONSTER_MOVE and require
their own explicit fallback result; #1354 remains open until those paths are
covered. The pre-production focused attempt failed compilation because the new
`respawn_relocated` field did not yet exist, not a behavioral regression claim.

## #1354 map-rebind wording accepted locally — 2026-11-20

Committed candidate `33596380857e527484301c2c401af5cbfb87f2d9`, based on
`0b3610ff2`, resolves the map-rebind wording review item in #1354. The mutation
root documentation, `BindingRestored` comment and error log now state that the
mutation result is discarded after the original map binding is restored. The
`canonical_application_refused` outcome, log fields and severity are unchanged.

The committed
`./tools/validation-v2 final --base 0b3610ff2 --architecture --require-changes --timings --logs`
campaign passed all nine commands from 04:40:14Z to 04:43:49Z (214.841 seconds),
within the 600-second target. The `wow-world-core` library passed all 297 tests
with zero failures or ignored tests. Verified manifest:
`target/validation-v2/manifests/20261120T044014.507070Z-3666869-final.json`.

Only unloaded-grid respawn relocation remains open in #1354; no runtime behavior
or issue closeout is claimed from this wording correction.

## #1354 inaccessible chase target retention accepted locally — 2026-11-20

Committed candidate `9317820b50c3628dd756b14770a7aea7899b0cfa`, based on
`93d1b60e1`, fixes the inaccessible-chase review item in #1354. The Rust branch
now retains `last_target_position` after `CannotReachTarget`, matching C++'s
assignment before its early return; a second identical update returns `Continue`
instead of rebuilding and rejecting the same path every tick. The unrelated
lost-target/immobilized reset and all other branches are unchanged.

The strengthened existing regression failed before the fix with `None` rather
than the target position, then passed with retention and `Continue` on the next
update. The committed
`./tools/validation-v2 final --base 93d1b60e1 --architecture --require-changes --timings --logs`
campaign passed all eight commands from 04:29:57Z to 04:34:24Z (266.860
seconds), within the 600-second target. The `wow-movement` library passed all
128 tests with zero failures or ignored tests. Verified manifest:
`target/validation-v2/manifests/20261120T042957.407476Z-3660047-final.json`.

Unloaded-grid respawn relocation and the map-rebind wording item remain open in
#1354; no live client capture or issue closeout is claimed.

## #1354 creature relocation visibility accepted locally — 2026-11-20

Committed candidate `59ceb11c674f4e855607e99a093d85fe1c897ffb`, based on
`c6c57166e`, completes the visibility-notify half of #1354 for all three existing
creature relocation entry points: direct in-place, lent-return and Creature
move-list drain. Valid destinations mark observers at the origin before the
operation and at the destination only when `relocated=true`, using the existing
deferred visibility rail without packets under the map guard. Invalid coordinates
produce no marks; blocked/error outcomes add no destination mark. Existing
outcomes and summary counters are unchanged. The C++ contract is
`Map::CreatureRelocation` and `Unit::UpdateObjectVisibility(false)`, cross-checked
against the issue's target anchors and the complementary AzerothCore source.

Four new Map tests prove direct, lent-return and move-list success plus the NaN
negative. Before the production fix, the three success cases failed on the
old-range observer; afterward all six matching cases passed. The committed
`./tools/validation-v2 final --base c6c57166e --architecture --require-changes --timings --logs`
campaign passed all nine commands from 04:21:46Z to 04:24:41Z (175.597 seconds),
within the 600-second target. The `wow-map` library passed 778 tests with zero
failures and one existing ignored test, including all four new identities. The
session syntax check retained 747 exact direct-registry rows. Verified manifest:
`target/validation-v2/manifests/20261120T042146.096709Z-3653679-final.json`.

Unloaded-grid respawn relocation and the two smaller review items in #1354 remain
open; no live client capture or #1354 closeout is claimed. Player runtime
integration and character creation remain separate work.

## Forever skill/book/item checkpoint qualified locally — 2026-11-17

Code candidate `920a32e0b1cb8bcb653b5222b5019a6033f81b6a`, based on published
`2cb94768d`, passes the complete committed-candidate final with architecture:
16/16 commands, twelve affected library suites, 9,459 passing tests and 18
ignored. The green manifest is
`target/validation-v2/manifests/20261117T214132.853555Z-1784580-final.json`;
its checkout is clean and the final profile is independently verified.
The actual Forever binary with all four native features passes 116 tests
(one ignored); the private nine-table Item loader passes its separately
requested ignored case after all input hashes match the retained manifest.
The extracted-source C++ oracle passes, including 129 distinct loaded-value
histories, alongside its preserved skill-status/book/override/cast contrasts.

The checkpoint includes isolated native skill/book load/save/field operations,
enchantment and item-bonus kernels/data sources, and captured Item query and
persistence inputs. It deletes both InventoryState copied Item-map producers.
Canonical Player skill/book/Item owner integration, remaining actor/runtime
copies, load/save/effects/publication consumers and actual character creation
remain unfinished. CharacterCreator is still None. This is checkpoint
qualification, not full macro, runtime, durability or target-wire acceptance.

Earlier final attempts remain failed: the first found format/policy drift;
the second found a nondeterministic failure-fence fixture. Its corrected
landed-hit input preserves every existing assertion and changes no gameplay.
Exact source/physical/logical reviews and failure boundaries are in the owning
Forever runbook. Local checks ran from 21:09:04Z to 21:49:08Z (2,403.829 seconds),
before documentation/publication closeout; repairs are recorded separately.
The complete 600-second ordinary target is not met. No campaign clock or
earlier failed evidence is reset, and warnings are not claimed baseline-free.
No service, client or real database operation was performed.

## Forever canonical inventory capture accepted locally — 2026-11-16

Resumed from published d91650024. The complete inventory query/publication
projection now captures owned facts instead of copying Player, preparing the
later non-Clone skill/book integration.
The complete read-only query preparation moves position validation, traversal,
use, bank, equipment,
two-hand and swap-preflight rules to private children of that same view: thirteen
public Player queries delegate, and two traversal helpers move with their owner.
Bank-slot count and Titan Grip are explicit inputs rather than new view state.
Parent comparison preserves all fifteen bodies after the reviewed receiver
substitutions and canonical call formatting. The incorrect profession-slot C++
comment is corrected; the actual duplicate-index Rust behavior remains a named
target-version gap, not a hidden gameplay repair.

Worker coding feedback passed the affected Entity checks, 124 existing
`player::tests::item_` cases and three new position/traversal cases. The four
use and four bank cases are included in those 124, not additional cases. The
explicit out-of-bag-size fixture slot is populated and asserted before checking
traversal. Equipment feedback first failed on test-only imports, then passed
after their cfg(test) retention. Parent subsequently applied canonical import
formatting; those dirty-tree coding results are not final evidence at a later
commit. These worker results did not establish acceptance. The later Core
producer replacement and committed-candidate campaign are recorded below. The owning
runbook's November 16 contract preserves observation timing, source-version
gaps and the disabled runtime constructor.

Sixteen shared value operations now serve canonical money, ordinary flags,
counts, bank flags, inventory/visible slots and buyback through borrowed
data/mask references. A boolean requests
the existing Object update admission; one private Player adapter owns that call.
Equal setters preserve existing masks, while explicit money marking always marks
and requests admission. No copied Player/Unit, new state or Core producer change
is introduced. Worker coding feedback passed the Entity check and nonzero money,
flag and receiver-split filters; final acceptance remains pending.

Count and bank-flag value rules now share the same borrowed owner. The flag
setter's validity result remains separate from update admission: invalid returns
false, valid unchanged returns true without new masks/queue, and changed values
mark the existing array bits. Its explicit marker retains the original unguarded
index and parent-before-index-addition order; that overflow path is source-reviewed,
not separately tested. The two newly dead scalar helpers are removed. Worker
feedback passed the Entity check, two existing count filters and two bank-flag
cases, including one new public validity/mask/queue regression. The eight
slot/visible/buyback rules also passed the Entity check and four existing
one-case filters for visible bits, inventory bits, buyback and forced markers.
They retain guards, integer/GUID widths, value-before-mask order and array bits
without adding scalar root bits.

Shared Player/Active resets and quest/stat projectors now retain their actual
operation order, complete predicate bit sets, cached/filtered Mana index and
receiver filtering. A new nested regression preserves quest/rest values,
clears their nested masks, checks both `Some(empty)` and a two-word zero self-
resurrection mask, and retains an unrelated toy mask. Worker check and six
nonzero focused cases passed; the added empty branch reran only its affected
case. Unit virtual-item setters/markers now also share pure value/mask rules,
with the original public guards and one Unit-owned admission adapter. The Entity
check and three nonzero filters passed, including a new equal/invalid no-mask/
no-queue regression. Raw aura capture now retains the mask and every per-state
caster list without computation or owner references; shared projectors keep the
old borrowed canonical path and deferred snapshot calculation. Frozen-owner and
debug-only empty-invalid-state regressions passed with the Entity check (four
focused executions). Parent contrasted Unit.cpp:6159–6167 and the two caster-
dependent states at 02245dcd; this is not full target-wire acceptance. The detached
Unit payload now captures only owned values, masks and raw aura facts, with no
Unit actor. Its lazy Unit branch preserves deferred aura projection; complete
DTO/frozen-owner and debug invalid-state tests both passed with the Entity check.
The detached Player inventory payload now captures complete represented values,
nested masks, native topology and cached publication inputs without an actor.
Its setters share canonical pure rules and its clear/update phases preserve the
old ordering. Four focused snapshot cases passed after correcting two test-
fixture power calls; the first test attempt failed to compile and ran no tests.
Coverage includes independent seeded owners, nested masks, receiver filtering,
frozen values/Mana inputs and deferred debug aura failure. The cached Mana
setter normalizes the out-of-range test input before capture. Entity check
passed. Query overlays now delegate through ItemStorageView using those captured
facts, and disposable/canonical top-level placement share narrow native store/
take primitives while retaining field/visible/bag-clear order. Seven snapshot
cases passed; three strengthened query cases passed again after correcting test
index types. They cover ordered/early-stop traversal, distinct native/field GUIDs,
count/Titan overlays, invalid removal and visible clearing even with an empty
native equipment slot.

The Core facade now owns only the Entity inventory snapshot. Its producer
captures facts under the existing generation-checked owned read and wraps them
after the manager guard is released; the whole-Player Clone producer is removed.
The unused projection Dual Wield setter and Titan Grip penalty argument are
retired with their single valuation caller; canonical capability APIs stay
unchanged. A shared pure tuple conversion retains visible-item input widths and
the all-zero default rule. Worker Core and player-conditions checks passed, as
did the affected consistent-bag Entity case and four Core regressions. The first
Core test compile failed on a missing test-only trait import and ran no tests;
the corrected/final filter passed. Core coverage compares independent owners'
complete models and bytes, actual NaN/-0 fields, active/detached and stale
generations, frozen values and the deferred aura panic outside an unpoisoned
manager mutex. Player, PlayerGameplayState and PlayerGameplayLoadRecord no longer
derive Clone. Their only aggregate gameplay-copy fixture now moves one state
and compares independently built expected buckets; its focused case and Entity
check passed. Narrow state/DTO clones remain unchanged. Root/storage physical ceilings
are reviewed at 4,699/1,904 lines; the logical Player row's reviewed source delta
is 24,021 production + 19,432 tests = 43,453 lines, confirmed by the final
parser-backed campaign. No gameplay/architecture debt or target parity is closed.

Code c59d5b36b82b27eb3ebdc38151aaa38488cedc6b passed the single final campaign
against published d91650024 on this x86_64 host, with one Cargo job, clean
provenance and the pinned read-only d8f4044 extractor overlay. Final
`--architecture --timings --logs --keep-going --require-changes` passed 11/11
commands in 552.364 seconds; its manifest is
target/creation-continuation-20261116/final-c59d5b36b.json and was verified green
with `--require-profile final`. It checks test targets in 35 affected/reverse-
dependent packages and runs Entity's 1187 and Core's 294 cases; player-conditions has no
library tests and passes its compile checks. No previous Entity/Core identity or
ignored state is lost: 1170 -> 1187 and 290 -> 294, with 17/4 additions.

The same campaign then passed three actual World inventory position/publication
regressions and the stale same-GUID committed-swap regression, in 201.366 and
0.252 seconds. These are disjoint from final's changed-library suites, not a
duplicate World campaign. The code-check window was 10:26:24.150–10:40:23.960 UTC
(839.810 seconds); post-run source/reference/clean-tree provenance review ended
at 10:43:31 UTC, making the complete evidence window 1026.850 seconds. The ordinary
600-second performance target is NOT met. Timings, identities and exact commands
are retained in inventory-capture-closeout.json, test-identity-review.json and
extra-consumer-results-c59d5b36b.json under the same directory. Coding/repair was
outside that window; no repeat or reduced acceptance is used to claim speed.

This closes the copied-actor prerequisite, not character creation. Native
skill/book canonical load/save/ack/publication/effect integration, the remaining
bulk-load/PartialEq boundary and target field-format/history evidence are still
unfinished; CharacterCreator stays None. No service/client/DB operation, runtime
installation, manual-test readiness or issue/architecture closeout is claimed.
Any documentation-only publication closeout retains this tested code SHA and
requires its own quick manifest at its actual later SHA.

## Forever publication candidate — 2026-11-11

The operator requested a commit and push of the preserved experimental worktree.
This is a publication checkpoint, not completion of character creation. The
composition still supplies no CharacterCreator, and no schema, service or live
database operation is authorized by this publication. The four loaded-grid fact
bindings now use primary_facts; normal formatting reduces spawn_groups.rs to
2,380 lines, below its existing 2,384 ceiling. The World test census is updated
after reviewing all 135 relocated test names at unique Entity/World destinations,
89 added World identities and the dependency/classification delta. The exact
current test metrics and formatter-only storage root adjustment are recorded
in their owning policies; neither grants general growth or retires core debt.

The old /tmp unit artifacts are no longer available. Their historical command
results below remain dated reports, not fresh evidence for this candidate.
Publication final passed at bf1cfaf9c281d3204c80903b42980b6ca79edd21; its
manifest and logs are preserved under target/publication-20261111. Additional
binary/native evidence retains its c5ece0ba5 SHA as detailed below. DB/restart
recovery and runtime acceptance remain absent; creation stays disabled.

The first publication campaign at bc8a33a6c passed physical limits, syntax
ownership, net-move, the test census and formatting, but reported growth in four
logical-owner aggregates. Their exact Map/composition/quest-fixture/Entity-family
deltas now have bounded reviews in runtime-ownership-ledger.json; the existing
#584 retirement conditions remain. No generic baseline or inherited-debt waiver
is used for this new growth. That campaign was interrupted with exit 143 after
another host campaign started and inspection found unpublished changes in the
sibling extractor. Replacement acceptance uses a read-only process-local overlay
of a clean d8f4044f63f4d47fcb44d655d11cea90889ec8ea extractor checkout, preserving
the real sibling worktree and the checkout-local Cargo target. Prior partial
results do not establish publication acceptance for this corrected candidate.

The pinned-sibling campaign at 3eb3038ea passed architecture and syntax
ownership, then its workspace all-target check returned 101: the two new
loaded-grid consumer tests imported private loader aliases for public persistence
row types. The dependent test compilation was stopped (143) so the same known
compile error is repaired before further acceptance. The repair uses the
original public persistence types, with fixture values/assertions and production
visibility unchanged. The recorded host wait was 3,088.14 seconds; the attempted
campaign itself ran 295.34 seconds. Those costs and the earlier failed campaign
remain separate recorded attempts, not a green ten-minute acceptance result.

At 4dd0ccd81 the full workspace/all-target check passed. The complete selected
library suites and contract checker reported 10,632 passing cases and six
Database failures: five legacy SQL contrasts were incorrectly pointed at the
sparse Forever reference, and one ambient recording test received events from
the new parallel SQL fixtures. The corrected setup uses CI's legacy reference
92796557f9b0ba3d2d1c7c770f535153154cf83e with both published CI SHA256 checks;
Forever source evidence remains separately pinned at 02245dcd. The 49 new
creation-fixture constructors now use one private helper with their own recorder;
all SQL, parameters, assertions, identities and production tracing stay unchanged.
The QA-bot check also required missing OpenSSL development headers; these are
prepared privately from the package matching the installed runtime, without a
system package change. That campaign ended 101 after 853.244 seconds, exceeding
the 600-second target. A corrected committed-candidate campaign remains required.

The 18939b2ee campaign used the corrected legacy references and private OpenSSL
headers: all reference contrasts and the QA-bot compilation passed. It ended
101 after 197.304 seconds with one remaining recording-test failure. Three
creation tests call production transaction builders internally (journal cleanup
and two recovery projections); these must use the existing capture-test guard
for the complete batch lifetime. The pure planning fixtures retain private
recorders. This is test isolation only; production tracing and all SQL/assertions
remain unchanged, and publication acceptance is still pending.

At c5ece0ba5 the Database suite passed (631 passed, nine ignored), including
the recording guards, and the remaining final checks passed except the existing
native split-death ordering fixture. A diagnostic reproduced a zero-damage MISS
packet at index 1, split logs at indices 2/3, and the normal 2,500-damage primary
packet at index 4: the original selector picked the earlier miss. Production
ordering was correct in that witness. The fixture now requests a normal swing
using its existing hit-chance aura and creature no-crit/no-crushing flags, asserts
zero avoidance, and retains the original selector, damage and ordering checks.
Packet-prefix diagnostics are test-only; no gameplay implementation is changed.
The first fixture repair failed to compile because it used the wrong flag owner;
the correction uses the existing lifecycle getter and runtime setter. That failed
attempt executed no tests and is not acceptance evidence. Corrected coding
feedback passed one exact case and 100/100 direct binary repetitions, each
executing one case. The tested fixture SHA256 is
4dcf42c1e22c18cbde432f79f0e2f76422e332890e93b1e317f854ed17249f7f;
commands, binary hash and preserved failure witnesses are recorded in
target/publication-20261111/native-death-diagnostic/fixture-normal-accessor/.

Additional acceptance at c5ece0ba5 passed: the feature-enabled Forever binary
(110 passed, one ignored), bnet-server binary (134 passed), synthetic source
container traversal oracle, and C# client-key probe compilation (five existing
nullable-context warnings, no errors). Logs and exact commands are retained in
target/publication-20261111/additional-results.json. The fixture-only follow-up
does not change any of those production inputs; their evidence retains its
original SHA. No client, service, schema or database operation was performed.
The c5ece0ba5 final attempt ended 101 after 182.983 seconds. The complete
publication elapsed time includes host waits and coding/error repair, measured
separately from acceptance commands. The 853.244-second attempt alone exceeded
the 600-second acceptance target; a later green rerun cannot erase that overrun.
Corrected committed-candidate acceptance is pending.

The corrected committed candidate bf1cfaf9c281d3204c80903b42980b6ca79edd21
passed `validation-v2 final --base 03eb9b5320af5cf6cbca79388179f73384d8b1a9
--architecture --timings --logs --keep-going --require-changes` with one Cargo
job and a clean worktree, from 13:21:35.652Z to 13:24:41.666Z (186.013 seconds).
All 19 declared commands passed, including workspace/all-target compilation,
10,174 selected library tests (17 ignored), 464 contract-checker tests and QA-bot
compilation. `verify --require-profile final` verified the green manifest
target/publication-20261111/final-bf1cfaf9c.json. The private dependency/header
setup and separate C++ reference pins above remained in effect. The test fixture
follow-up has no production/binary/native input delta from c5ece0ba5, so its
additional evidence is reused at that original SHA, without relabelling it.
This subsequent documentation-only closeout does not change the tested code;
its hygiene evidence and exact publication SHA are recorded separately. The
earlier 853.244-second failure and missing live/durability evidence remain.

## Forever character creation resumed — 2026-11-06

The active worktree at `03eb9b532` contains uncommitted login/build-70450 and
character-creation changes. Creation remains disabled at runtime because no
composition-owned constructor is attached. Source-reviewed local corrections
and authored regressions cover creation cancellation/unknown-commit fencing,
canonical ownership, SQL binding and GUID allocation, plus startup parsing
and diagnostic privacy. Further authored work requires explicit primary-row
state, preserves source scalar/vector/version and item-primary projections,
resolves container/child-creator references, and retains a repository recovery
fence across cancelled/uncertain saves. Full dependent save groups, canonical
Player/item construction and durable restart reconciliation remain open.
The base item save now represents gems, transmog, artifact/powers and ordinary
modifiers with explicit snapshot inputs and source-ordered auxiliary rows.
Explicit Azerite/empowered subtype saves now precede the base row; actual
template-dependent subtype selection and item construction remain unfinished.
An owned creation operation now reserves character/item identities before
construction and retains one writer gate through consumed save. A journal
marker in the character transaction, separate post-Auth cleanup and a
pending-marker/schema guard are authored; the explicit schema migration is
not applied, and real durability/reconciliation evidence is still absent.
The explicit projection-only reconciler and a connection-bound advisory writer
are now authored. Auth account/last-played locks precede character writes;
normal save/recovery share the projection encoders and use the existing SQL
commit classifier. Startup recovery wiring and real cross-process/DB/restart
acceptance remain open; no runtime creator is enabled.
The represented customization, spell-favorite and selection-cache groups now
include their source-mandated deletes in the correct order.
Fishing and battleground rows now use explicit constructor inputs in source
order around the primary/customization/inventory prefix.
Quest status/reward persistence now preserves ordered dirty queues, abandoned
retention and explicit-delete policies, objective/spawn rows and source widths.
Periodic quest groups preserve changed-empty semantics, source ordering and
integer conversions. Fishing state now retains the canonical uint8 range and
widens only at SQL binding.
Glyph/normal/PvP talent snapshots now preserve five groups, source iteration,
glyph narrowing and all five PvP rows even when every slot is zero.
Trait persistence now preserves dirty-state policies, key/ID distinctions,
nullable kind-specific fields and the source defaults for unknown kinds.
Spell-history projection now preserves cooldown/charge save order, held-state
filtering and signed epoch-second bindings before action persistence.
Aura persistence now preserves frozen eligibility/key outcomes, full binary
GUIDs, raw doubles and parent/effect order before skills; canonical aura
eligibility and mask construction remain open.
Stored aura locations now preserve source state/order/raw coordinates,
including its unassigned Spell-delete parameter behavior.
Achievement/reputation/quest-criteria projection now preserves dirty policies,
counter/date widths, map/ID distinctions and source order before the cache.
Saved equipment and legacy transmog sets now preserve all four dirty states,
type-specific parameter orders, value GUIDs and ordered map traversal.
Modern transmog outfits now preserve ordered dirty IDs, missing-lookup deletes,
parent/situation/slot order and exact signed/unsigned field bindings.
Session tutorial/restriction snapshots now preserve changed/loaded and
empty-map policies, account identity and source order after the selection cache.
Currency snapshots now preserve frozen lookup/state decisions, zero writes,
source iteration, uint16 key narrowing and distinct REPLACE/UPDATE orders.
CUF snapshots now preserve all five slot operations and the source's 27-bit
boolean mask, with exact DELETE/REPLACE field widths after currency.
Character-data snapshots now preserve ordered dirty element/word keys,
delete/nonzero-insert policies, typed nulls and raw float bindings after CUF.
Character bank-tab snapshots now always delete and reinsert the owner's set after
character-data writes, preserving vector indexes, private strings and signed flags.
Garrison snapshots now preserve absence/empty policies, source delete/insert
order, optional plot filtering and supplied follower/ability identities.
Statistics snapshots now preserve the frozen eligibility decision, complete
setter layout and the source's resilience/mastery overwrite/default behavior.
Mail snapshots now preserve the updated/state gate, checked-bit narrowing,
ordered link/attachment cleanup and all eleven source item-deletion statements.
The construction contract now retains a guarded owner across save and local
confirmation/publication, with cleanup on cancellation and failure. The target
GUID helper preserves sixteen realm bits for selection/creation, and starting
actions are preparation requests rather than fabricated SQL trait/spec rows.
Format-1 recovery remains limited to its represented Auth projection; complete
account effects need durable intents and Battle.net-owner exclusion first.
The source pet tail has independent commits outside that journal; conditional
pet creation also needs an explicit durability/recovery contract before activation.
Worker feedback now has a passing `wow-database` check and all 171 focused
`forever::create::` library cases. This work remains **NOT VALIDATED** for
completed-delivery acceptance. Subsequent worker feedback also has a passing
`wow-world` check and all 470 focused `forever::` cases, including 14 creation
lifetime/fencing cases. This remains coding feedback under level 1; the
accepted November 1 integration below does not qualify these new inputs.
The subsequent signed-XP correction has a passing `wow-world` check and all
nine focused vitals cases after correcting one zero-XP fixture failure. XP and
NextLevelXP are signed int32 in the target source as well as the entity model;
the unsupported `INT32_MIN - 1` boundary remains guarded without a native-parity
claim. The earlier 470-case run predates this correction. The existing entity
is the selected base-state owner where its target contract is represented;
complete birth initialization and one canonical skill/spell authority remain open.
The shared PowerType model now uses target ComboPoints ID 4 and BurningEmbers
ID 14, with one decoder for kinds 0–25 and preserved invalid-byte policies.
Worker coding feedback has a passing `world-server` check, two constants
cases and fourteen entity power cases. It does not establish native/runtime
acceptance or enable creation.
The follow-up armor preparation fix preserves the source float create-agility
rounding before signed conversion without changing current integer stats.
Its `wow-world` check and all eleven focused vitals cases pass as worker coding
feedback; undefined C++ conversion boundaries remain explicitly guarded.
The complete target skill algorithms now use separate basic-catalog and full
learning-source interfaces, preserving weak-query admission and mandatory
reentrant effects. Their first World-owned form has a passing `wow-world`
check and all 48 focused skill cases. Interface extraction is not replacement
of the canonical skill vector, its 256-slot publication or load/save consumers.
Those interfaces, metadata, input and generic error now belong to Entities;
the actual BirthCatalog adapter belongs to Data, using existing dependencies.
The lower-layer extraction also has a passing `wow-world` check and all 48
focused cases. The complete private skill state/algorithm/effect family now
lives in Entities, with World retaining the real-source adapter and facade.
Its check passes; 38 Entity and 13 World cases pass after repairing two missing
World test imports. All 48 original cases remain, with three source-adapter
splits added. This remains coding feedback, not completed acceptance. The
ordinary Player still uses its old skill vector: consumer/load/save/publication
replacement and production effect/factory integration remain open, and no
parallel skill field or runtime creator was added.
A separate C++-backed book correction now uses category 11 for primary
professions, preserving the unranked-first fallback and point-effect ordering.
Its check and all 62 focused learning cases pass after repairing a new test's
rank-supersession return-value expectation. Complete Add/Remove cases cover
secondary/child exclusion and higher ranks. Those results predate the source
and family extraction below; complete creation acceptance remains open.
The lower definition-source/view boundary and complete generic cast resolver
now have a passing `wow-world` check and all 22 focused cast cases. The first
test build exposed a borrowed-source GAT lifetime requirement; it was made
explicit before the passing check/test rerun. All 19 original cases remain,
with three boundary cases added. Cast still requires no learning readiness;
the subsequent complete learning source boundary also has a passing
`wow-world` check, 63 learning cases and
the same 22 cast cases. One missing test import was repaired before those
passing runs. All 62 original learning cases remain, plus a real-view item-fit
case. Add/Learn/Remove and private phases now use the lower source contracts;
the complete book/state/effect family has subsequently moved to Entities.
World keeps the admitted adapter, concrete aliases and real-source integration
cases. Its check passes; 52 Entity and 45 World cases pass, preserving all 93
original identities and adding four source projections. Two missing test
helper/import failures were repaired, then one unused test import was removed
and the World filter rerun. The operation bodies and private state remain
unchanged; ordinary Player spell collections have not yet been replaced.
Canonical load/save/publication consumers, production effects/factory and
complete creation acceptance remain open. These are worker coding results on
the dirty tree, not completed acceptance.
The skill owner now records only successful full-width status insertions and
admits provider-ordered borrowed status/field views. Its `wow-world` check and
all 46 Entity skill cases pass, preserving the original 38 cases and adding
eight insertion/admission regressions. No failed feedback loop occurred.
Exact-set admission does not establish native iteration order. The separate
skill-specific numeric adapter is now implemented; its Forever binary check
and five focused adapter cases pass with all four required features. The
independent extracted-source oracle is authored but uncompiled/unexecuted.
These remain coding results. Load/save/acknowledgement, live publication and canonical
Player replacement remain open; no runtime creator is enabled.
The subsequent borrowed item-storage view preserves seven complete query
bodies and the existing Player API, preparing trade planning to stop copying
whole Players. After repairing extraction imports/lifetimes its Entity check
passes; 26 existing item cases pass through their real mounted modules. The
first requested filter executed zero cases and is not credited. Actual trade
snapshot/consumer replacement is now implemented across Inventory and Social:
the pair read captures only inventory and original SQL facts, then plans
outside the map lock. All three whole-Player planning clone statements are
retired. Its repaired `wow-world` check passes, with 124 Entity item cases,
3 transfer cases, 9 inventory-planning cases, 6 Social capture/SQL cases and
all 9 original World trade-commit cases passing. The initial private-module
compile failure and the interrupted tool wait are recorded; the original
Cargo process completed normally without a duplicate run. Parent reviewed
the patch/current hashes and unchanged async fence/application functions.
These remain level-1 coding results, not final or target inventory acceptance.
The direct quest/gameplay capture, emptiness predicate and quest-log fixture
copies are now retired. Their World check and 26 Entity misc, 3 Core quest
snapshot and 252 World quest cases pass as worker coding feedback; all 72
unaffected function texts remain unchanged. Eleven new regressions cover
narrow read facts, emptiness and actual fixture-owner transfers/rejections.
The initially prescribed World filter ran zero cases and is not credited;
the corrected mounted family supplies the 252 passing cases. Parent reviewed
all seven tested source hashes and the actual unit patch. The indirect
map-record output copies are replaced in the partial local implementation,
but this next unit is unfinished: its binary check and 69 focused cases pass;
the attempted server binary test ran zero cases because tests live in the
composition library. The library test and World trade regression remain
unexecuted, as does a one-line physical-ratchet repair. Luna stopped on its
usage limit; no fallback model or parent routine implementation replaced it.
Parent recovered the exact source/feedback and closed the stopped worker;
no Cargo process remains active. Canonical load/save/effects/factory
integration remains unfinished. The renewed trace
also found the shared inventory query/values-publication projection's
`Clone::clone` producer; that separate cross-crate boundary must be replaced
before removing the aggregate derives.
The [owning runbook checkpoint](../operations/forever-login.md#character-creation-worktree-checkpoint--2026-11-06)
records exact source anchors, the intentional acknowledgement contract and
remaining construction/recovery/native-evidence gates. No new runtime or
publication result is claimed.

## Forever modular integration accepted — 2026-11-01

Code candidate `1d5dd370cf5d337f606263b20bda92a38e8913d8` replays all 49 non-merge
Forever commits onto `origin/3.4.3` at `a680ab4328fab903bff5cd17cabd750d3ff716b3`.
The legacy modular owners remain intact: navigation and terrain adapters live in
`wow-world-core`, shared handler vocabulary stays in `wow-handler`, and Forever's
six account-phase handlers use one explicit static registration list without
`inventory`. Package admission remains available for v23, v56 and v57.

`./tools/validation-v2 final --base origin/3.4.3 --architecture --timings --logs`
passed on Linux x86_64 and its manifest was verified green for the final profile:
`target/validation-v2/manifests/20261101T155505.499900Z-759752-final.json`.
Focused checks also passed: 12 navigation tests, three terrain tests, one
feature-disabled regression, 57 Python protocol tests, 460 active `wow-database`
tests, and 464 handler-contract tests. The timing-sensitive split-death case
passed independently. The reviewed architecture, net-move and test-census gates
are green; the 449 new source-local tests remain explicit placement debt, not a
completed test migration.

This is local structural and synthetic acceptance, not a fresh real-client,
full-world or extractor-completion claim. No live runtime was activated and no
private client-data probes were run during this integration.

## Earlier evidence

The [archived checkpoint at this revision](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/migration/STATE.md) contains the dated C++ references, capture and QA records, commands, tested SHAs and limits from earlier deliveries. Use the [port plan](PORT_PLAN.md) and the open owner issue when deciding what to implement next.

## 5. Definition of "done" going forward (replaces `represented-complete`)

A capability is **done** only when: (1) it runs in the **live runtime**, (2) its wire output
is **capture-clean vs a C++ capture** of the same action — byte/opcode exact unless a narrowly
reviewed comparator omits only an intrinsically runtime-allocated identifier or canonicalizes
only proven unordered C++ collection order, while retaining every stable value/count and failing
malformed input — (3) it has been **exercised on a running server/client**, and (4) C++ refs +
the validating capture/test are cited.
`represented-complete` is no longer a closure state — it means "logic drafted, not yet live".

This gameplay-capability definition does not require a fresh capture for every behavior-preserving
structural commit. Refactors follow the proportional gates in AGENTS.md and their explicit issue
acceptance: preserve bytes, metadata, connection and order with focused evidence; obtain the
required action-specific capture/live evidence when the change or acceptance calls for it.
Report a bounded proof as bounded, and distinguish code tests, historical golden regression,
fresh runtime evidence and full functional parity. Architecture completion requires the complete
technical gate: #584 core, the #583 product (including the mandatory Rust/Wasm/C mixed delivery
with optional operator activation), and the independent #153 audit. #133 is already closed
administratively and is not a future gate. Do not substitute a favorable field count or test
total. Neither architecture closure nor playable M6 closes the full Part-2 parity ledgers.
