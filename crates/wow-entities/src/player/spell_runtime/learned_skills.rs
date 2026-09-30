//! Complete represented SpellLearnSkill gain and rank-downgrade valuation.
//! Application lookups and writers answer individual ordered steps.

mod protocol;
pub use protocol::*;

#[derive(Clone, Copy)]
enum Phase {
    GainRoot,
    GainNode(u32),
    InitialPrevious(u32),
    PreviousNode(u32),
    SearchPrevious(u32),
    FirstRank(u32),
    Value,
    Maximum,
    Range,
    Level,
    Tier(i16),
    Write(LearnedSkillWrite),
    Done(bool),
}

pub struct LearnedSkillOperation<'a> {
    roots: &'a [i32],
    index: usize,
    gain: bool,
    original: Option<LearnedSkillNode>,
    previous: u32,
    node: Option<LearnedSkillNode>,
    value: u16,
    maximum: u16,
    new_maximum: u16,
    always_max: bool,
    phase: Phase,
}

impl<'a> LearnedSkillOperation<'a> {
    pub fn gain(roots: &'a [i32]) -> Self {
        Self {
            roots,
            index: 0,
            gain: true,
            original: None,
            previous: 0,
            node: None,
            value: 0,
            maximum: 0,
            new_maximum: 0,
            always_max: false,
            phase: Phase::GainRoot,
        }
    }

    pub fn downgrade(node: LearnedSkillNode, current_spell_id: u32) -> Self {
        Self {
            roots: &[],
            index: 0,
            gain: false,
            original: Some(node),
            previous: 0,
            node: None,
            value: 0,
            maximum: 0,
            new_maximum: 0,
            always_max: false,
            phase: Phase::InitialPrevious(current_spell_id),
        }
    }

    pub fn step(&mut self) -> LearnedSkillStep {
        loop {
            return match self.phase {
                Phase::GainRoot => {
                    if self.index == self.roots.len() {
                        self.phase = Phase::Done(true);
                        continue;
                    }
                    let Ok(spell_id) = u32::try_from(self.roots[self.index]) else {
                        self.phase = Phase::Done(false);
                        continue;
                    };
                    self.index += 1;
                    self.phase = Phase::GainNode(spell_id);
                    continue;
                }
                Phase::GainNode(id) => LearnedSkillStep::GainNode(id),
                Phase::InitialPrevious(id) | Phase::SearchPrevious(id) => {
                    LearnedSkillStep::PreviousRank(id)
                }
                Phase::PreviousNode(id) => LearnedSkillStep::PreviousNode(id),
                Phase::FirstRank(id) => LearnedSkillStep::FirstRank(id),
                Phase::Value => {
                    LearnedSkillStep::Value(self.node.expect("learned skill node").skill_id)
                }
                Phase::Maximum => {
                    LearnedSkillStep::Maximum(self.node.expect("learned skill node").skill_id)
                }
                Phase::Range => {
                    LearnedSkillStep::Range(self.node.expect("learned skill node").skill_id)
                }
                Phase::Level => LearnedSkillStep::LevelMaximum,
                Phase::Tier(tier_id) => LearnedSkillStep::TierMaximum {
                    tier_id,
                    index: u32::from(
                        self.node
                            .expect("learned skill node")
                            .step
                            .saturating_sub(1),
                    ),
                },
                Phase::Write(write) => LearnedSkillStep::Write(write),
                Phase::Done(complete) => LearnedSkillStep::Done(complete),
            };
        }
    }

    pub fn advance(&mut self, input: LearnedSkillInput) {
        match (self.phase, input) {
            (Phase::GainNode(_), LearnedSkillInput::GainNode(lookup)) => match lookup {
                LearnedSkillLookup::Present(node) => self.accept_node(node),
                LearnedSkillLookup::Absent => self.phase = Phase::GainRoot,
                LearnedSkillLookup::Unavailable => self.phase = Phase::Done(false),
            },
            (Phase::InitialPrevious(_), LearnedSkillInput::Rank(previous)) => {
                self.previous = previous;
                if previous == 0 {
                    self.clear_original();
                } else {
                    self.phase = Phase::PreviousNode(previous);
                }
            }
            (Phase::PreviousNode(_), LearnedSkillInput::PreviousNode(node)) => {
                if let Some(node) = node {
                    self.accept_node(node);
                } else if self.previous != 0 {
                    self.phase = Phase::SearchPrevious(self.previous);
                } else {
                    self.clear_original();
                }
            }
            (Phase::SearchPrevious(_), LearnedSkillInput::Rank(previous)) => {
                self.previous = previous;
                self.phase = Phase::FirstRank(previous);
            }
            (Phase::FirstRank(_), LearnedSkillInput::Rank(first)) => {
                self.phase = Phase::PreviousNode(first);
            }
            (Phase::Value, LearnedSkillInput::Value(value)) => {
                let Some(value) = value else {
                    self.phase = Phase::Done(false);
                    return;
                };
                self.value = if self.gain {
                    value.max(self.node.expect("learned skill node").value)
                } else {
                    value
                };
                self.phase = Phase::Maximum;
            }
            (Phase::Maximum, LearnedSkillInput::Maximum(maximum)) => {
                let Some(maximum) = maximum else {
                    self.phase = Phase::Done(false);
                    return;
                };
                self.maximum = maximum;
                let node = self.node.expect("learned skill node");
                self.new_maximum = node.max_value;
                if self.new_maximum == 0 {
                    self.phase = Phase::Range;
                } else {
                    if !self.gain && self.value > node.value {
                        self.value = node.value;
                    }
                    self.finish_math();
                }
            }
            (Phase::Range, LearnedSkillInput::Range(range)) => {
                match range {
                    LearnedSkillRange::Unavailable => {
                        if self.gain {
                            self.phase = Phase::Done(false);
                        } else {
                            self.finish_math();
                        }
                        return;
                    }
                    LearnedSkillRange::None { always_max } => {
                        if self.gain {
                            self.phase = Phase::Done(false);
                            return;
                        }
                        self.always_max = always_max;
                    }
                    LearnedSkillRange::Language { always_max } => {
                        self.value = 300;
                        self.new_maximum = 300;
                        self.always_max = always_max;
                    }
                    LearnedSkillRange::Level { always_max } => {
                        self.always_max = always_max;
                        self.phase = Phase::Level;
                        return;
                    }
                    LearnedSkillRange::Mono { always_max } => {
                        self.new_maximum = 1;
                        self.always_max = always_max;
                    }
                    LearnedSkillRange::Rank {
                        always_max,
                        tier_id,
                    } => {
                        self.always_max = always_max;
                        self.phase = Phase::Tier(tier_id);
                        return;
                    }
                }
                self.finish_range();
            }
            (Phase::Level, LearnedSkillInput::LevelMaximum(maximum)) => {
                self.new_maximum = maximum;
                self.finish_range();
            }
            (Phase::Tier(_), LearnedSkillInput::TierMaximum(maximum)) => {
                if let Some(maximum) = maximum {
                    self.new_maximum = maximum.try_into().unwrap_or(u16::MAX);
                } else if self.gain {
                    self.phase = Phase::Done(false);
                    return;
                }
                self.finish_range();
            }
            (Phase::Write(_), LearnedSkillInput::Applied) => {
                self.phase = if self.gain {
                    Phase::GainRoot
                } else {
                    Phase::Done(true)
                };
            }
            _ => unreachable!("input must answer the current learned skill step"),
        }
    }

    fn accept_node(&mut self, node: LearnedSkillNode) {
        self.node = Some(node);
        self.always_max = false;
        self.phase = Phase::Value;
    }

    fn clear_original(&mut self) {
        self.phase = Phase::Write(LearnedSkillWrite {
            skill_id: self.original.expect("downgrade source").skill_id,
            step: 0,
            value: 0,
            max_value: 0,
        });
    }

    fn finish_range(&mut self) {
        if self.always_max {
            self.value = self.new_maximum;
        }
        self.finish_math();
    }

    fn finish_math(&mut self) {
        let node = self.node.expect("learned skill node");
        let maximum = if self.gain {
            self.maximum.max(self.new_maximum)
        } else {
            if self.maximum > self.new_maximum {
                self.maximum = self.new_maximum;
            }
            if self.value > self.new_maximum {
                self.value = self.new_maximum;
            }
            self.maximum
        };
        self.phase = Phase::Write(LearnedSkillWrite {
            skill_id: node.skill_id,
            step: node.step,
            value: self.value,
            max_value: maximum,
        });
    }
}
