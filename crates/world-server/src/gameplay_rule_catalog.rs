//! Composition boundary for bounded gameplay rule catalogs.

use anyhow::{Result, bail};
use wow_persistence::{
    FactionChangePairPersistenceRowLikeCpp, GameplayRuleCatalogPersistencePortLikeCpp,
    GameplayRuleRowsLoadOutcomeLikeCpp, NpcSpellClickPersistenceRowLikeCpp,
    NpcVendorPersistenceRowLikeCpp,
};

fn loaded<T>(outcome: GameplayRuleRowsLoadOutcomeLikeCpp<T>) -> Result<T> {
    match outcome {
        GameplayRuleRowsLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        GameplayRuleRowsLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn faction_pair(
    row: FactionChangePairPersistenceRowLikeCpp,
) -> wow_data::FactionChangePairRowLikeCpp {
    wow_data::FactionChangePairRowLikeCpp {
        alliance_id: row.alliance_id,
        horde_id: row.horde_id,
    }
}

pub(super) async fn load_faction_change_store_like_cpp<
    AchievementExists,
    QuestExists,
    ReputationExists,
    SpellExists,
    TitleExists,
>(
    persistence: &dyn GameplayRuleCatalogPersistencePortLikeCpp,
    achievement_exists: AchievementExists,
    quest_exists: QuestExists,
    reputation_exists: ReputationExists,
    spell_exists: SpellExists,
    title_exists: TitleExists,
) -> Result<wow_data::FactionChangeLoadOutcomeLikeCpp>
where
    AchievementExists: FnMut(u32) -> bool,
    QuestExists: FnMut(u32) -> bool,
    ReputationExists: FnMut(u32) -> bool,
    SpellExists: FnMut(u32) -> bool,
    TitleExists: FnMut(u32) -> bool,
{
    let rows = loaded(persistence.load_faction_change_rows_like_cpp().await)?;
    Ok(
        wow_data::FactionChangeStoreLikeCpp::from_validated_rows_like_cpp(
            rows.achievements.into_iter().map(faction_pair),
            rows.quests.into_iter().map(faction_pair),
            rows.reputations.into_iter().map(faction_pair),
            rows.spells.into_iter().map(faction_pair),
            rows.titles.into_iter().map(faction_pair),
            achievement_exists,
            quest_exists,
            reputation_exists,
            spell_exists,
            title_exists,
        ),
    )
}

pub(super) async fn load_npc_vendor_store_like_cpp(
    persistence: &dyn GameplayRuleCatalogPersistencePortLikeCpp,
) -> Result<wow_data::NpcVendorLoadOutcomeLikeCpp> {
    let rows = loaded(persistence.load_npc_vendor_rows_like_cpp().await)?;
    Ok(wow_data::NpcVendorStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter().map(
            |row: NpcVendorPersistenceRowLikeCpp| wow_data::NpcVendorRowLikeCpp {
                entry: row.entry,
                item: row.item,
                maxcount: row.maxcount,
                incrtime: row.incrtime,
                extended_cost: row.extended_cost,
                vendor_type: row.vendor_type,
                bonus_list_ids_raw: row.bonus_list_ids_raw,
                player_condition_id: row.player_condition_id,
                ignore_filtering: row.ignore_filtering,
            },
        ),
    ))
}

pub(super) async fn load_npc_spell_click_store_like_cpp(
    persistence: &dyn GameplayRuleCatalogPersistencePortLikeCpp,
    creature_templates: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
    spells: &wow_data::SpellStore,
) -> Result<wow_data::NpcSpellClickStoreLikeCpp> {
    let rows = loaded(persistence.load_npc_spell_click_rows_like_cpp().await)?;
    Ok(wow_data::NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter()
            .map(
                |row: NpcSpellClickPersistenceRowLikeCpp| wow_data::NpcSpellClickRowLikeCpp {
                    npc_entry: row.npc_entry,
                    spell_id: row.spell_id,
                    cast_flags: row.cast_flags,
                    user_type: row.user_type,
                },
            ),
        |npc_entry| creature_templates.get(npc_entry).is_some(),
        |spell_id| spells.get(spell_id as i32).is_some(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faction_pair_preserves_unsigned_identity() {
        assert_eq!(
            faction_pair(FactionChangePairPersistenceRowLikeCpp {
                alliance_id: u32::MAX,
                horde_id: 7,
            }),
            wow_data::FactionChangePairRowLikeCpp {
                alliance_id: u32::MAX,
                horde_id: 7,
            }
        );
    }
}
