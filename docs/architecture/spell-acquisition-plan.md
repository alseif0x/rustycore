# Immutable spell-acquisition plan

Issue #164 introduces a pure prerequisite for the trainer transaction slices in #157–#161.
It does not teach a spell, mutate a player, write a row, or send a packet. Given one complete
player snapshot, effective acquisition metadata, and either `DirectLearn(spell)` or
`TrainerWrapperCast(spell)`, it returns the whole deterministic mutation graph or one typed
`Indeterminate` reason.

## Causal contract

The projection follows the legacy C++ call graph rather than flattening trainer triggers:

1. `Player::AddSpell` validates the spell, handles existing/removed/disabled state, recursively
   learns previous ranks, provisionally inserts the row, resolves active ranks, classifies its
   autocast, applies direct/fallback skills, and then expands dependencies
   (`Player.cpp:2741-3137`).
2. `Player::LearnSpell` publishes only after `AddSpell` completes, then reactivates disabled
   higher ranks and required spells (`Player.cpp:3192-3233`).
3. `Player::SetSkill` writes step/value/max before learning rewarded spells and changes the
   persistence state afterwards (`Player.cpp:5653-5870`).
4. Wrapper casts execute every HANDLE_HIT effect before the HIT_TARGET loop; each target learn
   effect completes its recursive closure before the next effect
   (`Spell.cpp:3949-3962,4001-4017,4136-4154`).

`SpellAcquisitionPlanLikeCpp::mutations` is therefore the authoritative cross-domain order.
The separate spell, skill, and override vectors are typed projections for bounded consumers; they
must not be combined to reconstruct causal order.

C++ does not require a unique `active` bit across disabled ranks, and disabled-case reactivation
can even leave more than one enabled rank active. The projection retains those exact row flags and
processes every active rank in stable spell-ID order instead of rejecting a C++-reachable snapshot.
This is fidelity to the legacy transition, not a newly invented active-rank invariant.

The same rule applies to observable work that runs after each logical mutation. `AddSpell`
updates `LearnTradeskillSkillLine` and `LearnSpellFromSkillLine` once for every effective
`SkillLineAbility` row, in row order and without deduplicating repeated SkillLine IDs, and then
updates `LearnOrKnowSpell` once (`Player.cpp:3118-3132`). `SetSkill` learns rewards first, emits
`UpdateMountCapability` only when an already represented Riding skill increases, and finally
updates `SkillRaised` followed by `AchieveSkillStep` (`Player.cpp:5707-5733,5860-5867`). These
are separate typed action intents in the plan; downstream code must not collapse them into a
generic criteria refresh.

Profession association order also follows the point where C++ writes
`ProfessionSkillLine`. A previously absent root is associated before its rewarded-spell closure,
including when its new value is zero;
an existing zero-valued root is associated only after that closure. Consequently, if activating
existing root A rewards a newly absent root B, the ordered capacity input is `[B, A]`. Issue #156
must consume that order directly rather than sorting or rediscovering it.

Future-player-condition authority is an occurrence-sensitive tape, not a map keyed by condition
ID. Each reward gate consumes the next `(condition_id, allowed)` entry at the exact causal point
where C++ calls `MeetsFutureSpellPlayerCondition`; a missing or mismatched entry fails closed.
This allows the same condition to be false before an earlier reward mutates spells/skills and true
when evaluated again afterwards. The resulting snapshot retains the original immutable tape so
#159 can repeat the same projection under the canonical owner.

Reward race gates use C++ `Trinity::RaceMask::GetMaskForRace`, including its compact mapping for
non-contiguous race IDs (for example 34 -> bit 11 and 70 -> bit 15), rather than shifting by
`race_id - 1` (`RaceMask.h:93-140`). Unknown race IDs and classes outside the C++ `Classes`
range fail closed before any closure is projected.

## Static safety and live-player cast results are different facts

A triggered self-cast is not equivalent to “all effects execute”. `TRIGGERED_FULL_MASK` still
runs the first `CheckCast` and target selection (`Spell.cpp:3411-3473`). `AddUnitTarget` applies
`IsImmunedToSpellEffect` and records the surviving effect mask (`Spell.cpp:2396-2439`);
`SpellHitResult` checks spell immunity before its positive/self shortcut
(`Object.cpp:2661-2678`, `Unit.cpp:7375-7551`).

That creates an observable split:

- `SKILL` is HANDLE_HIT on the caster and runs in `_handle_immediate_phase`
  (`SpellEffects.cpp:4572-4602`);
- `LEARN_SPELL`, `SKILL_STEP`, and `DUAL_WIELD` are HIT_TARGET handlers and run only when their
  effect bit survives (`SpellEffects.cpp:2025-2069,2176-2184,2289-2317`).

C++ can consequently apply an immediate skill while suppressing a later learn effect. Static
spell evidence proves absence of scripts, linked behavior, delayed/channeled execution,
unmodelled checks, runtime modifiers, and other graph-changing paths. Separately, the immutable
player snapshot supplies `reached_immediate_phase` and `executed_hit_target_effect_mask` for each
acquisition-bearing cast. Missing either authority returns `Indeterminate`; the planner never
assumes that an incompletely represented player has no immunity. Issue #159 must recompute and
compare these live inputs while holding the canonical mutation owner.

## Effective 3.4.3 data audit

The audit used the final effective acquisition stores composed by #163, the final trainer spell
catalog, the world script/condition/linked/disable tables, and the C++ hard-coded DUMMY dispatch:

- 255 accepted trainer offers use 90 distinct castable wrapper spells.
- Those wrappers contain only player acquisition effects: 95 `LEARN_SPELL` effects and 88
  `SKILL_STEP` effects. Their learn closure contains 93 distinct target spells.
- Only target spells `12716`, `13240`, and `22967` require C++
  `SpellMgr::IsSpellValid` crafting authority. They create items `10577`, `10577`, and `17771`;
  the effective item templates and every positive reagent exist.
- Only targets `33388` and `34090` take the passive autocast branch. Four offers reach them.
  Both contain an inert audited DUMMY plus deterministic `SKILL(762)` (steps 1 and 3), with no
  script binding, legacy `spell_scripts` command, pet aura, linked cast/hit/aura, cast/target
  condition, disable, class/label modifier, aura-195 learn path, hard-coded DUMMY case, or
  runtime-dependent `CalcValue`.
- None of the 90 wrappers or 93 learned targets is a `Mount.db2` source spell. Generic mount
  learning remains indeterminate here: C++ `CollectionMgr::AddMount` can recursively call
  `LearnSpell` for a faction counterpart, so a post-commit “reconcile mount” label would conceal
  a durable acquisition edge.

This proves that the real wrapper closure is supported when its current player-specific effect
mask is supplied. It does not turn arbitrary custom spells into trusted no-ops.

For an acquisition-bearing passive, the plan consumes every supported acquisition handler and
does not enqueue a second ordinary passive cast. The only remaining supported effects in the
audited closure are inert DUMMY/null handlers. A passive with no acquisition handler may retain
the deferred runtime refresh intent; any mixed passive with an observable unmodelled remainder
fails closed instead of relying on replay being idempotent.

## Intentional legacy repairs

Two legacy implementation defects are not protocol requirements:

- `AddSpell` demotes a newly inserted lower rank when a higher rank is active, but returns its
  stale local `active` argument. C++ can emit both superseded and learned publications for the
  same hidden lower rank. Rust returns the final row activity and omits the contradiction.
- `SetSkill` uses slot zero as both a valid position and “not found”, and recursive parent/child
  activation can select or re-enter a slot before either row claims it. Rust uses exact occupied
  slot authority, counts deleted rows whose `SkillLineID` still occupies an update-field slot,
  distinguishes root-child expansion from structural parent cycles, rechecks after recursive
  expansion, and fails atomically at capacity. Profession association inputs are restricted to
  the only C++ slots, 0 and 1; wider values cannot truncate through an `i8` conversion.

Both deviations have focused regression fixtures and are also recorded in
`docs/migration/EXISTING-CODE-DEFECTS.md`.

## Downstream rules

- #157 may consume only `root_primary_profession_skill_ids`; it must not rediscover effects.
- #158 applies the exact immutable plan and owns durable statements/publication intents.
- #159 compares the same snapshot, effective metadata identity, and cast resolutions under the
  owner before applying anything.
- Any missing complete spell/skill rows, complete per-spell trait-definition IDs, complete
  `Player::m_overrideSpells` edges, exact skill-slot occupancy, causal future-condition
  resolution, complete parentage for every effective `SkillLine` identity,
  static cast proof, live cast resolution, or non-mount proof fails before money, DB, player, or
  packet mutation.

## Atomic application boundary (#158)

The plan now carries its exact immutable source snapshot. Application first replays the single
cross-domain mutation stream, verifies both typed projections, resulting rows, profession inputs,
identity fields, provenance and every post-commit action, then compares the live authority with
that source. The separately prepared #156 capacity plan is a mandatory input: its capacity
arithmetic, ordered new-profession IDs, existing membership, normalizations and unique physical
slots are validated, then its assignments are applied to the final skill rows before persistence.
A normalized resulting snapshot is an explicit already-applied retry; it is not reported or
published as a new learn.

An action-only plan (the C++ already-known `LearnSpell` quest-objective case) has a separate
validated publication outcome. It neither opens the Character DB transaction nor normalizes or
replaces runtime persistence state. Each publication replaces the represented pending-action
batch, and changing the session's player identity clears that batch, so intents cannot accumulate
without bound or cross character lifetimes.

Durability is one Character DB transaction. It locks the character row, replaces the complete
durable `character_spell`, `character_spell_favorite` and `character_skills` sets in deterministic
order, uses strict inserts, and commits before touching runtime state. Dependent and temporary
spells remain runtime-only as in C++ `_SaveSpells`; favorite maintenance remains independent of
the dependent-spell insert gate. Skills retain their runtime step and tombstone slot while only
the C++ DB tuple `(skill, value, max, professionSlot)` is durable. A lost COMMIT response is never
guessed: the complete three-table state is reread under the same character lock and publication
continues only on exact equality.

That immediate durability boundary is for consumers whose operation itself is database-gated.
Generic player `EffectLearnSpell` preserves the distinct C++ timing: it installs the same
validated plan's dirty post-`LearnSpell` snapshot and publishes synchronously, without awaiting or
requiring Character DB. The ordinary `Player::SaveToDB` plan now consumes those exact
`NEW`/`CHANGED`/`REMOVED` spell and skill states through `_SaveSpells`/`_SaveSkills`-shaped
statements and normalizes them only after a successful full-save commit. This avoids both the old
shallow grant and making unrelated pending player state durable during a cast.

After a confirmed/reconciled commit, one non-awaiting phase replaces the complete spell, trait,
override, skill, profession and controller mirrors before processing the ordered action stream.
Learned/superseded/unlearned packets use the C++ `LearnedSpellInfo` option-bit and payload order,
including favorite and trait-definition values. Criteria, quest, passive and mount actions are
retained as an ordered represented intent log until their canonical managers own those effects;
dual wield is applied to the existing canonical player owner before packet publication, and a
missing canonical owner returns a post-commit reconciliation error without emitting packets. The
generic represented player `EffectLearnSpell` uses the same validated authority, fails closed
without complete snapshot/metadata, and defers persistence to normal `Player::SaveToDB` like C++;
pet, item and battle-pet branches remain separate owners.

Issue #159 extends the database-gated boundary for normal trainer teaching: startup audits
effective effects plus script, linked-spell, condition, disable, pet-aura, aura-restriction and
equipment blockers into immutable cast/craft authority; the buy path recomputes the current effect
mask under the money owner, then commits that exact prepared result and the guarded fee together.
Until the canonical player owns C++'s complete spell/effect-immunity maps, any active aura makes
wrapper resolution indeterminate instead of assuming no immunity. After confirmed/reconciled
commit, money, visual kits and the acquisition stream publish in C++ success order. Dispatcher
activation remains #142.
