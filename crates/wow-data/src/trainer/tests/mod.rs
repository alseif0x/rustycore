//! Trainer store regressions.
//!
//! Separated from trainer.rs under #683.

use std::cell::RefCell;

use super::*;

fn spell_row(trainer_id: u32, spell_id: u32) -> TrainerSpellRowLikeCpp {
    TrainerSpellRowLikeCpp {
        trainer_id,
        spell: TrainerSpellLikeCpp {
            spell_id,
            money_cost: 100,
            req_skill_line: 0,
            req_skill_rank: 0,
            req_ability: [0, 0, 0],
            req_level: 1,
        },
    }
}

fn trainer_row(id: u32) -> TrainerRowLikeCpp {
    TrainerRowLikeCpp {
        id,
        trainer_type: TRAINER_TYPE_TRADESKILL_LIKE_CPP,
        greeting: format!("Hello {id}"),
    }
}

fn from_rows_with_existing_references(
    trainer_rows: impl IntoIterator<Item = TrainerRowLikeCpp>,
    trainer_spell_rows: impl IntoIterator<Item = TrainerSpellRowLikeCpp>,
    trainer_locale_rows: impl IntoIterator<Item = TrainerLocaleRowLikeCpp>,
    creature_trainer_rows: impl IntoIterator<Item = CreatureTrainerRowLikeCpp>,
) -> TrainerLoadOutcomeLikeCpp {
    TrainerStoreLikeCpp::from_rows_like_cpp(
        trainer_rows,
        trainer_spell_rows,
        trainer_locale_rows,
        creature_trainer_rows,
        |_| true,
        |_| true,
        |_| true,
        |_, _| true,
    )
}

mod scenarios;
