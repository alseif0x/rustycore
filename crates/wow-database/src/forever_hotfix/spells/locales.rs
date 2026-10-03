//! Captured esES locale overlays, independent of main-table enUS values.
//! 02245dcd DB2DatabaseLoader::LoadStrings/AddString and locale SQL statements.
use super::{ForeverHotfixRepository, LoadError, PreparedStatement, SpellRow};
use crate::SqlResult;
use wow_persistence::forever::spells::{
    BattlePetSpeciesLocaleRow, DifficultyLocaleRow, SpellCategoryLocaleRow, SpellLocaleOverlays,
    SpellLocaleRows, SpellNameLocaleRow, SpellRangeLocaleRow, SpellShapeshiftFormLocaleRow,
};
use wow_persistence::forever::spells::{SpellItemEnchantmentLocaleRow, TalentLocaleRow};

pub(super) const QUERIES: [&str; 8] = [
    "SELECT ID, Name_lang FROM spell_name_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
    "SELECT ID, Name_lang FROM difficulty_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
    "SELECT ID, DisplayName_lang, DisplayNameShort_lang FROM spell_range_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
    "SELECT ID, Name_lang FROM spell_shapeshift_form_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
    "SELECT ID, Description_lang, SourceText_lang FROM battle_pet_species_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
    "SELECT ID, Name_lang FROM spell_category_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
    "SELECT ID, Description_lang FROM talent_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
    "SELECT ID, Name_lang, HordeName_lang FROM spell_item_enchantment_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
];
impl ForeverHotfixRepository {
    pub async fn load_spell_es_es_overlays(&self) -> Result<SpellLocaleOverlays, LoadError> {
        let mut rows = SpellLocaleOverlays {
            official: SpellLocaleRows::default(),
            custom: SpellLocaleRows::default(),
        };
        for (table, sql) in QUERIES.iter().enumerate() {
            for custom in [false, true] {
                let mut statement = PreparedStatement::new(*sql);
                statement.set_bool(0, !custom);
                statement.set_string(1, "esES");
                let mut result = self
                    .0
                    .query(&statement)
                    .await
                    .map_err(|_| LoadError::Database)?;
                if result.is_empty() {
                    continue;
                }
                let target = if custom {
                    &mut rows.custom
                } else {
                    &mut rows.official
                };
                loop {
                    match table {
                        0 => target.spell_names.push(spell_name(&result)?),
                        1 => target.difficulties.push(difficulty(&result)?),
                        2 => target.spell_ranges.push(spell_range(&result)?),
                        3 => target
                            .spell_shapeshift_forms
                            .push(spell_shapeshift_form(&result)?),
                        4 => target.battle_pet_species.push(battle_pet_species(&result)?),
                        5 => target
                            .spell_category_definitions
                            .push(spell_category(&result)?),
                        6 => target.talents.push(talent(&result)?),
                        7 => target
                            .spell_item_enchantments
                            .push(spell_item_enchantment(&result)?),
                        _ => unreachable!(),
                    }
                    if !result.next_row() {
                        break;
                    }
                }
            }
        }
        Ok(rows)
    }
}

pub(super) fn spell_name(result: &SqlResult) -> Result<SpellNameLocaleRow, LoadError> {
    let mut r = SpellRow::new(result, 2)?;
    let row = SpellNameLocaleRow {
        id: r.read()?,
        name: r.text()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn difficulty(result: &SqlResult) -> Result<DifficultyLocaleRow, LoadError> {
    let mut r = SpellRow::new(result, 2)?;
    let row = DifficultyLocaleRow {
        id: r.read()?,
        name: r.text()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_range(result: &SqlResult) -> Result<SpellRangeLocaleRow, LoadError> {
    let mut r = SpellRow::new(result, 3)?;
    let row = SpellRangeLocaleRow {
        id: r.read()?,
        display_name: r.text()?,
        display_name_short: r.text()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_shapeshift_form(
    result: &SqlResult,
) -> Result<SpellShapeshiftFormLocaleRow, LoadError> {
    let mut r = SpellRow::new(result, 2)?;
    let row = SpellShapeshiftFormLocaleRow {
        id: r.read()?,
        name: r.text()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn battle_pet_species(
    result: &SqlResult,
) -> Result<BattlePetSpeciesLocaleRow, LoadError> {
    let mut r = SpellRow::new(result, 3)?;
    let row = BattlePetSpeciesLocaleRow {
        id: r.read()?,
        description: r.text()?,
        source_text: r.text()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_category(result: &SqlResult) -> Result<SpellCategoryLocaleRow, LoadError> {
    let mut r = SpellRow::new(result, 2)?;
    let row = SpellCategoryLocaleRow {
        id: r.read()?,
        name: r.text()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn talent(result: &SqlResult) -> Result<TalentLocaleRow, LoadError> {
    let mut r = SpellRow::new(result, 2)?;
    let row = TalentLocaleRow {
        id: r.read()?,
        description: r.text()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_item_enchantment(
    result: &SqlResult,
) -> Result<SpellItemEnchantmentLocaleRow, LoadError> {
    let mut r = SpellRow::new(result, 3)?;
    let row = SpellItemEnchantmentLocaleRow {
        id: r.read()?,
        name: r.text()?,
        horde_name: r.text()?,
    };
    r.finish()?;
    Ok(row)
}
