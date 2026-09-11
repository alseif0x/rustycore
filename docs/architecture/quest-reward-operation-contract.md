# Quest reward — operation contract

Owner document for the quest-reward operation delivered under
[#718](https://github.com/alseif0x/rustycore/issues/718), P1 of the
[architecture completion plan](refactor-completion-plan.md).

## Versioned C++ anchors

`/home/server/woltk-trinity-legacy` at `a5f8da2e`:

| Behavior | Anchor |
| --- | --- |
| The operation | `src/server/game/Entities/Player/Player.cpp:14625` `Player::RewardQuest` |
| Package rewards | `Player.cpp:14582` `Player::RewardQuestPackage` |
| Reward mail, own transaction | `Player.cpp:14794` |
| Closing save | `Player.cpp:14867` `SaveToDB(false)` |
| Character transaction begin/commit | `Player.cpp:19312` |
| Save group order | `Player.cpp:19630`–`19655` |
| Turn-in entry | `QuestHandler.cpp:398` |

## Durable contract

C++ applies every grant in memory and reaches the database once, in the
closing `SaveToDB(false)`. The reward's durable participants inside that
character transaction are `_SaveInventory`, `_SaveQuestStatus`, the
daily/weekly/seasonal/monthly quest-status groups and `_SaveCurrency`, plus the
money carried by the character row. Reward mail is the single deliberate
exception and keeps its own transaction.

RustyCore commits exactly those participants as one transaction:

1. Every removal and grant records what it needs durable into the operation's
   plan. Nothing is written while the operation is still deciding.
2. `_SaveCurrency` is projected once at the end, from a copy, so the change
   flags are cleared only after a known COMMIT.
3. The batch is committed once, in the C++ group order.
4. Publication precedes the commit, as it does in Classic.

Failure or a crash before the commit persists nothing: the quest stays
retryable and no grant is duplicated. This replaces the earlier per-grant
writes, under which a reward could be durable in parts.

### Ambiguous COMMIT

The witness is written by the same transaction, so either form proves the whole
batch's fate:

- **Money**, when the reward changes it. This reuses the established
  exclusive-money reconciliation unchanged.
- **The rewarded quest's active status row** otherwise. Both durable shapes of
  the operation delete it, so its absence proves the commit and its presence
  proves the rollback.

When neither can be observed the session is quarantined and kicked, matching
the existing unknown-COMMIT money fence.

### Definite rollback

Nothing durable changed, so the quest stays retryable and no grant is
duplicated. The operation did mutate the in-memory inventory while it planned,
exactly as C++ `StoreNewItem` does before its save, so the session is
quarantined rather than left showing items and quest progress the database
does not have. Classic leaves that mismatch standing until relog; here it ends
immediately, and the reward is never published.

### Recorded departures

- C++ applies money to memory before its save and publishes before it. This
  server keeps its exclusive-money fence, so money becomes visible only after a
  known COMMIT. The fence is stricter than Classic, never weaker.
- For the same reason the quest leaves the log only after a known COMMIT. C++
  mutates first and leaves memory ahead of a failed save until relog; here the
  two stay in step, and a reward that does not commit is not published.
- The transaction carries the reward's participants rather than every group of
  the full character save. The groups the reward does not modify are unchanged,
  so the durable outcome for the operation is the same. Widening it to a global
  save is a separate decision, not a silent one.

## Participants confirmed unimplemented

These are absent subsystems rather than misplaced code. They remain open port
work and must not be reported as implemented because the operation now owns its
boundary:

`Battleground::HandleQuestComplete`, which `QuestHandler.cpp:400` runs before
the operation; `UpdateSkillPro`; `SetTitle`; mail (`MailDraft`, `SendMailTo`,
`SendItemRetrievalMail`); honor (`CalculateHonorGain`, `RewardHonor`); criteria
updates and `SendDisplayToast`; `SetQuestCompletedBit`; the max-level
XP-to-money conversion (`GetRewMoneyMaxLevel() * RATE_DROP_MONEY`); the
next-quest offer; phase shift and `UpdateObjectVisibility`; the creature and
game-object AI `OnQuestReward` hooks and `ScriptMgr::OnQuestStatusChange`.

Reward skill, spell casts, title and mail exist today only as `#[cfg(test)]`
recordings. Rust also applies money before XP where C++ applies XP first; that
ordering is retained for now and belongs to the same open port.
