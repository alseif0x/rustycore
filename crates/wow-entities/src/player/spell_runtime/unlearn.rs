//! Ordered Player::RemoveSpell operation.
//!
//! TrinityCore a5f8da2e Player.cpp:3236-3462. This preserves the represented
//! Rust subset, including its row, reactivation and omitted-aura gaps.
//! Only frames and the original visited-ID set live here; runtime authority
//! stays in PlayerSpellRuntimeState. Application resolves each requested fact
//! and performs each external effect at that request's original point.

use std::collections::HashSet;

mod owner;
mod protocol;
pub use protocol::{
    SpellUnlearnEdge, SpellUnlearnInput, SpellUnlearnOwnerOutcome, SpellUnlearnOwnerStep,
    SpellUnlearnStep,
};

#[derive(Clone, Copy)]
enum Phase {
    Known,
    Rows,
    Invalidate,
    Visit,
    Next,
    Talent,
    NextKnown,
    Requiring,
    RequiredNext,
    RequiredKnown,
    Forget,
    Skill,
    Learned,
    LearnedNext,
    LearnedOverride,
    Previous,
    Ranked,
    PreviousKnown,
    Reactivate,
    Superceded,
    DropTrait,
    Trait,
    TraitOverride,
    TitanGrip,
    DualWield,
    Offhand,
    Unlearned,
    Done,
}

struct Frame<I> {
    spell_id: i32,
    learn_low_rank: bool,
    suppress_messaging: bool,
    phase: Phase,
    preserve_complete: bool,
    was_dependent: bool,
    next_is_talent: bool,
    target: u32,
    edge: Option<SpellUnlearnEdge>,
    requiring: std::vec::IntoIter<i32>,
    learned: Option<I>,
    prev_activate: bool,
}

impl<I> Frame<I> {
    fn new(spell_id: i32, learn_low_rank: bool, suppress_messaging: bool) -> Self {
        Self {
            spell_id,
            learn_low_rank,
            suppress_messaging,
            phase: Phase::Known,
            preserve_complete: false,
            was_dependent: false,
            next_is_talent: false,
            target: 0,
            edge: None,
            requiring: Vec::new().into_iter(),
            learned: None,
            prev_activate: false,
        }
    }
}

/// A synchronous continuation for exactly one spell-unlearning call.
/// Its iterator holds the application's existing learned-node snapshot,
/// without allocating another catalog or copying Player runtime.
pub struct SpellUnlearnOperation<I> {
    frames: Vec<Frame<I>>,
    seen: HashSet<i32>,
}

impl<I: Iterator<Item = SpellUnlearnEdge>> SpellUnlearnOperation<I> {
    pub fn new(spell_id: i32, suppress_messaging: bool) -> Self {
        Self {
            frames: vec![Frame::new(spell_id, true, suppress_messaging)],
            seen: HashSet::new(),
        }
    }

    /// Return the next read, owner write or application effect. Internal
    /// recursion resumes its caller, including after a failed child owner.
    pub fn step(&mut self) -> SpellUnlearnStep {
        loop {
            let Some(frame) = self.frames.last_mut() else {
                return SpellUnlearnStep::Done;
            };
            let signed_id = frame.spell_id;
            let unsigned_id = u32::try_from(signed_id).ok();
            return match frame.phase {
                Phase::Known => SpellUnlearnStep::Known(signed_id),
                Phase::Rows => SpellUnlearnStep::RowsComplete,
                Phase::Invalidate => SpellUnlearnStep::InvalidateRows,
                Phase::Visit => {
                    if !self.seen.insert(signed_id) {
                        self.frames.pop();
                    } else {
                        frame.phase = if unsigned_id.is_some() {
                            Phase::Next
                        } else {
                            Phase::Forget
                        };
                    }
                    continue;
                }
                Phase::Next => SpellUnlearnStep::NextRank(unsigned_id.unwrap()),
                Phase::Talent => SpellUnlearnStep::Talent(frame.target),
                Phase::NextKnown => SpellUnlearnStep::Known(frame.target as i32),
                Phase::Requiring => SpellUnlearnStep::Requiring(unsigned_id.unwrap()),
                Phase::RequiredNext => {
                    if let Some(required) = frame.requiring.next() {
                        frame.target = required as u32;
                        frame.phase = Phase::RequiredKnown;
                    } else {
                        frame.requiring = Vec::new().into_iter();
                        frame.phase = Phase::Forget;
                    }
                    continue;
                }
                Phase::RequiredKnown => SpellUnlearnStep::Known(frame.target as i32),
                Phase::Forget => SpellUnlearnStep::Owner(SpellUnlearnOwnerStep::Forget {
                    spell_id: signed_id,
                    preserve_complete: frame.preserve_complete,
                }),
                Phase::Skill => SpellUnlearnStep::DowngradeSkill(unsigned_id.unwrap()),
                Phase::Learned => SpellUnlearnStep::Learned(unsigned_id.unwrap()),
                Phase::LearnedNext => {
                    if let Some(edge) = frame.learned.as_mut().and_then(Iterator::next) {
                        if let Ok(learned_id) = i32::try_from(edge.spell_id) {
                            frame.edge = Some(edge);
                            frame.phase = Phase::LearnedOverride;
                            self.frames.push(Frame::new(learned_id, true, false));
                        }
                    } else {
                        frame.learned = None;
                        frame.phase = if frame.learn_low_rank {
                            Phase::Previous
                        } else {
                            Phase::DropTrait
                        };
                    }
                    continue;
                }
                Phase::LearnedOverride => {
                    let edge = frame.edge.expect("the child has one learned edge");
                    if edge.overrides_spell_id != 0 {
                        if let Ok(overridden) = i32::try_from(edge.overrides_spell_id) {
                            return SpellUnlearnStep::RemoveOverride {
                                overridden,
                                replacement: edge.spell_id as i32,
                            };
                        }
                    }
                    frame.phase = Phase::LearnedNext;
                    continue;
                }
                Phase::Previous => SpellUnlearnStep::PreviousRank(unsigned_id.unwrap()),
                Phase::Ranked => SpellUnlearnStep::Ranked(unsigned_id.unwrap()),
                Phase::PreviousKnown => SpellUnlearnStep::Known(frame.target as i32),
                Phase::Reactivate => SpellUnlearnStep::Reactivate {
                    spell_id: frame.target as i32,
                    dependent: frame.was_dependent,
                },
                Phase::Superceded => SpellUnlearnStep::Superceded {
                    spell_id: signed_id,
                    previous_spell_id: frame.target as i32,
                },
                Phase::DropTrait => {
                    SpellUnlearnStep::Owner(SpellUnlearnOwnerStep::DropOverridesAndTrait {
                        spell_id: signed_id,
                    })
                }
                Phase::Trait => SpellUnlearnStep::TraitOverride(frame.target),
                Phase::TraitOverride => SpellUnlearnStep::RemoveOverride {
                    overridden: frame.target as i32,
                    replacement: signed_id,
                },
                Phase::TitanGrip => SpellUnlearnStep::TitanGrip(signed_id),
                Phase::DualWield => SpellUnlearnStep::DualWield(signed_id),
                Phase::Offhand => SpellUnlearnStep::Offhand,
                Phase::Unlearned => {
                    if let Some(spell_id) = unsigned_id.filter(|_| !frame.prev_activate) {
                        SpellUnlearnStep::Unlearned {
                            spell_id,
                            suppress_messaging: frame.suppress_messaging,
                        }
                    } else {
                        frame.phase = Phase::Done;
                        continue;
                    }
                }
                Phase::Done => {
                    self.frames.pop();
                    continue;
                }
            };
        }
    }

    /// Supply only the fact/effect result requested by step().
    pub fn advance(&mut self, input: SpellUnlearnInput<I>) {
        let frame = self
            .frames
            .last_mut()
            .expect("an outstanding removal frame");
        match (frame.phase, input) {
            (Phase::Known, SpellUnlearnInput::Known(true)) => frame.phase = Phase::Rows,
            (Phase::Known, SpellUnlearnInput::Known(false)) => {
                self.frames.pop();
            }
            (Phase::Rows, SpellUnlearnInput::RowsComplete(complete)) => {
                frame.preserve_complete = complete;
                frame.phase = if complete {
                    Phase::Visit
                } else {
                    Phase::Invalidate
                };
            }
            (Phase::Invalidate, SpellUnlearnInput::Applied) => frame.phase = Phase::Visit,
            (Phase::Next, SpellUnlearnInput::Rank(next)) => {
                frame.target = next;
                frame.phase = if next != 0 && i32::try_from(next).is_ok() {
                    Phase::Talent
                } else {
                    Phase::Requiring
                };
            }
            (Phase::Talent, SpellUnlearnInput::Talent(talent)) => {
                frame.next_is_talent = talent;
                frame.phase = Phase::NextKnown;
            }
            (Phase::NextKnown, SpellUnlearnInput::Known(known)) => {
                let child = frame.target as i32;
                let recurse = known && !frame.next_is_talent;
                frame.phase = Phase::Requiring;
                if recurse {
                    self.frames.push(Frame::new(child, false, false));
                }
            }
            (Phase::Requiring, SpellUnlearnInput::Requiring(required)) => {
                frame.requiring = required.into_iter();
                frame.phase = Phase::RequiredNext;
            }
            (Phase::RequiredKnown, SpellUnlearnInput::Known(known)) => {
                let child = frame.target as i32;
                frame.phase = Phase::RequiredNext;
                if known {
                    self.frames.push(Frame::new(child, true, false));
                }
            }
            (
                Phase::Forget,
                SpellUnlearnInput::Owner(Some(SpellUnlearnOwnerOutcome::Forgotten {
                    was_dependent,
                })),
            ) => {
                frame.was_dependent = was_dependent;
                frame.phase = if u32::try_from(frame.spell_id).is_ok() {
                    Phase::Skill
                } else {
                    Phase::DropTrait
                };
            }
            (Phase::Forget, SpellUnlearnInput::Owner(None)) => {
                self.frames.pop();
            }
            (Phase::Skill, SpellUnlearnInput::Applied) => frame.phase = Phase::Learned,
            (Phase::Learned, SpellUnlearnInput::Learned(learned)) => {
                frame.learned = Some(learned);
                frame.phase = Phase::LearnedNext;
            }
            (Phase::LearnedOverride, SpellUnlearnInput::Applied) => {
                frame.phase = Phase::LearnedNext
            }
            (Phase::Previous, SpellUnlearnInput::Rank(previous)) => {
                frame.target = previous;
                frame.phase = if previous != 0 && i32::try_from(previous).is_ok() {
                    Phase::Ranked
                } else {
                    Phase::DropTrait
                };
            }
            (Phase::Ranked, SpellUnlearnInput::Ranked(ranked)) => {
                frame.phase = if ranked {
                    Phase::PreviousKnown
                } else {
                    Phase::DropTrait
                };
            }
            (Phase::PreviousKnown, SpellUnlearnInput::Known(known)) => {
                frame.phase = if known {
                    Phase::Reactivate
                } else {
                    Phase::DropTrait
                };
            }
            (Phase::Reactivate, SpellUnlearnInput::Applied) => frame.phase = Phase::Superceded,
            (Phase::Superceded, SpellUnlearnInput::Applied) => {
                frame.prev_activate = true;
                frame.phase = Phase::DropTrait;
            }
            (Phase::DropTrait, SpellUnlearnInput::Owner(outcome)) => {
                let trait_id = match outcome {
                    Some(SpellUnlearnOwnerOutcome::TraitDefinition(id)) => id,
                    None => None,
                    _ => panic!("trait cleanup must return a trait definition"),
                };
                if let Some(id) = trait_id.and_then(|id| u32::try_from(id).ok()) {
                    frame.target = id;
                    frame.phase = Phase::Trait;
                } else {
                    frame.phase = Phase::TitanGrip;
                }
            }
            (Phase::Trait, SpellUnlearnInput::TraitOverride(overridden)) => {
                frame.target = overridden as u32;
                frame.phase = if overridden > 0 {
                    Phase::TraitOverride
                } else {
                    Phase::TitanGrip
                };
            }
            (Phase::TraitOverride, SpellUnlearnInput::Applied) => frame.phase = Phase::TitanGrip,
            (Phase::TitanGrip, SpellUnlearnInput::Applied) => frame.phase = Phase::DualWield,
            (Phase::DualWield, SpellUnlearnInput::Applied) => frame.phase = Phase::Offhand,
            (Phase::Offhand, SpellUnlearnInput::Applied) => frame.phase = Phase::Unlearned,
            (Phase::Unlearned, SpellUnlearnInput::Applied) => frame.phase = Phase::Done,
            _ => panic!("input must match the current spell-removal phase"),
        }
    }
}

#[cfg(test)]
mod tests;
