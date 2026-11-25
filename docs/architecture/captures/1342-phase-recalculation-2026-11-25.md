# 3.4.3.54261 phase recalculation QA

This record covers the area eligibility portion of #1342. The port
evaluates the destination area and its parents against the canonical
Player and active game-event state. It retains the area condition on each
phase reference. Quest mutations and live aura changes reevaluate those
conditions before publishing the new phase and refreshing visibility.
Loaded grid spawns and game-event spawns retain their typed phase data.

The target source at `a5f8da2e` is `PhasingHandler::OnAreaChange` and
`PhasingHandler::OnConditionChange` (`PhasingHandler.cpp:299–482`),
`PhaseShift::AddPhase` (`PhaseShift.cpp:25–50`), the quest callers in
`Player.cpp:14883–14885,15821–15823,16333–16335`, and direct aura-phase
effects in `SpellAuraEffects.cpp:1744–1768`. The port also reevaluates
area predicates after ordinary aura changes. `AuraApplication::_InitFlags`
(`SpellAuras.cpp:110–161`) supplies the positive and cancelable flags
for the visible, non-passive Player-owned aura used by the test.

## Two observers at area entry

The prepared 54261 QA clients used the project QA trust configuration.
They joined an isolated copy of map 0, area 9 at `(-8948.0, -80.0, 88.78)`
during the configured event 86 window. The aarch64 debug world was built
at source revision `540014ebe`, SHA-256
`c6972b16894bc65f5e4e797d96dd9bf2031bb007eba737ae8493a0083fb30351`.

| Observer | Phase evidence | Visibility evidence |
| --- | --- | --- |
| Mage | Frames 147 and 151 grant 198 and 199 | Frame 153 creates the restricted GameObject and AreaTrigger. Frame 185 creates the restricted creature. |
| Paladin | Frames 432 and 437 contain an empty phase list | Frames 433 and 565 create the unphased controls. The restricted entities are absent. |

## Same-area quest and aura changes

The mage remained at the same map, area and coordinates through all four
actions. The verifier found no movement start, jump or world transfer
after admission. At source revision `cac8c6169`, the aarch64 debug world
SHA-256 was
`4ba9c0f9eb97b7823182b6b1d56d782f83a72253a260820febfba08b6205215a`.

| Action | Client and aura frames | Phase frame | Visibility frame |
| --- | --- | --- | --- |
| Accept fixture quest 3510000 | 348 `QuestGiverAcceptQuest` | 351 grants 198 | 352 creates the restricted creature and GameObject |
| Abandon the quest | 368 `QuestLogRemoveQuest` | 369 clears 198 | 370 removes both entities |
| Cast Arcane Intellect | 418 `CastSpell`, 423 `AuraUpdate` for spell 1459 with flags `0x0103` | 425 grants 199 | 426 creates the restricted AreaTrigger |
| Cancel Arcane Intellect | 428 `CancelAura` for spell 1459, 429 clears the same aura slot | 430 clears 199 | 431 removes the AreaTrigger |

The server removed the aura 741 ms after application and published the
empty phase list 18 ms after receiving the cancellation. Frame 428 has
SHA-256 `7cdddd4a600fac3e7c0e56249671c841ae662df2a5804b50c07261bb944d04fa`.
Frame 431 has SHA-256
`7f92d80658f49b33af073accf1bc480037c5b8a9749f8e9be50bbf345da5bcaf`.
The earlier `7702a796c` run also verified natural aura expiration.
The temporary services, QA client, four cloned schemas, and temporary
database role were removed after testing.

The real-client runs used development revisions that also contained work
from #1514 and separate spawn-placement fixes. The phase PR carries the
phase implementation and its regressions onto `3.4.3`. The capture
applies to the tested development revisions above. Terrain swaps, personal
spawn materialization, stock client login, and the remaining #1342
scenarios retain their own acceptance.
