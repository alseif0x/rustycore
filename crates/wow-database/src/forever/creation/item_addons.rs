//! ObjectMgr::LoadItemTemplateAddon, 02245dcd:3424-3470. Complete read-only batch.
use super::{LoadError, SqlResult, WorldDatabase, field, rows};
use wow_persistence::forever::item_specs::ItemAddonRow;
const QUERY: &str = "SELECT Id,FlagsCu,FoodType,MinMoneyLoot,MaxMoneyLoot,SpellPPMChance,RandomBonusListTemplateId,QuestLogItemId FROM item_template_addon";
pub(super) async fn load(world: &WorldDatabase) -> Result<Vec<ItemAddonRow>, LoadError> {
    rows(world, QUERY, decode).await
}
fn decode(row: &SqlResult) -> Result<ItemAddonRow, LoadError> {
    if row.field_count() != 8 {
        return Err(LoadError::InvalidRow);
    }
    Ok(ItemAddonRow {
        id: field(row, 0)?,
        flags: field(row, 1)?,
        food: field(row, 2)?,
        min_money: field(row, 3)?,
        max_money: field(row, 4)?,
        spell_ppm: field(row, 5)?,
        random_bonus_template: field(row, 6)?,
        quest_log_item: field(row, 7)?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_preserves_all_eight_source_columns_and_no_invented_item_template_join() {
        assert_eq!(
            QUERY,
            "SELECT Id,FlagsCu,FoodType,MinMoneyLoot,MaxMoneyLoot,SpellPPMChance,RandomBonusListTemplateId,QuestLogItemId FROM item_template_addon"
        );
        assert!(!QUERY.contains("JOIN") && !QUERY.contains("ORDER BY"));
    }
}
