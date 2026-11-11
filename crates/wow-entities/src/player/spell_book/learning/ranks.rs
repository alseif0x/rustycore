use super::super::{PlayerSpellState as State, SpellBookOrderError};
use super::{
    LearningError, PlayerSpellBook, SpellBookMessage, SpellLearningEffects, SpellLearningSource,
    effect, source,
};

use crate::{SpellLearningError, SpellLearningSourceError};
impl PlayerSpellBook {
    pub(super) fn replace_ranks<S: SpellLearningSource, E: SpellLearningEffects<S>>(
        &mut self,
        spell: u32,
        sources: &S,
        effects: &mut E,
    ) -> Result<bool, LearningError<E::Error, S>> {
        let keys = self
            .source_entries(|history, count| effects.book_order(history, count))
            .map_err(|error| match error {
                SpellBookOrderError::Source(error) => SpellLearningError::Effect(error),
                SpellBookOrderError::InvalidKeySet => {
                    source(SpellLearningSourceError::InvalidBookOrder)
                }
            })?
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        let new_rank = sources.spell_rank_node(spell).expect("ranked caller");
        let mut superseded_old = false;
        for id in keys {
            if self.spell(id).unwrap().state == State::Removed {
                continue;
            }
            let Some(_) = super::facts::spell(sources, id).map_err(source)? else {
                continue;
            };
            if id == spell {
                continue;
            }
            let Some(old_rank) = sources.spell_rank_node(id) else {
                continue;
            };
            if old_rank.first != new_rank.first || !self.spell(id).unwrap().active {
                continue;
            }
            let higher = new_rank.rank > old_rank.rank; // uint8 wrap is not normalized.
            if effects.is_in_world() {
                effect(effects.publish(
                    self,
                    if higher {
                        SpellBookMessage::Superseded {
                            old: id,
                            new: spell,
                        }
                    } else {
                        SpellBookMessage::Superseded {
                            old: spell,
                            new: id,
                        }
                    },
                ))?;
            }
            let target = if higher { id } else { spell };
            let current = self.entries.get_mut(&target).unwrap();
            current.active = false;
            if current.state != State::New {
                current.state = State::Changed;
            }
            if higher {
                superseded_old = true;
            }
        }
        Ok(superseded_old)
    }
}
