//! Full numeric/text official/custom batches, not a transactional snapshot.
//! Target source 02245dcd HotfixDatabase.cpp:870-872,980-981,1067-1081,1104.
mod locales;
mod sparse;
use super::{ForeverHotfixRepository, LoadError, PreparedStatement};
use crate::SqlResult;
use wow_persistence::forever::items::{
    ItemEffectRow, ItemOverlays, ItemRelationRow, ItemRow, ItemRows,
};

const QUERIES: [&str; 4] = [
    "SELECT ID,ClassID,SubclassID,Material,InventoryType,SheatheType,ItemPetFoodID,SoundOverrideSubclassID,IconFileDataID,ItemGroupSoundsID,ContentTuningID,ModifiedCraftingReagentItemID,Unknown1200,CraftingQualityID,ItemSquishEraID,RecraftReagentCountPercentage,OrderSource FROM item WHERE (VerifiedBuild>0)=?",
    concat!(
        "SELECT ID,Description,Display3,Display2,Display1,Display,ExpansionID,DmgVariance,LimitCategory,DurationInInventory,QualityModifier,BagFamily,StartQuestID,LanguageID,ItemRange,",
        "StatPercentageOfSocket1,StatPercentageOfSocket2,StatPercentageOfSocket3,StatPercentageOfSocket4,StatPercentageOfSocket5,StatPercentageOfSocket6,StatPercentageOfSocket7,StatPercentageOfSocket8,StatPercentageOfSocket9,StatPercentageOfSocket10,",
        "StatPercentEditor1,StatPercentEditor2,StatPercentEditor3,StatPercentEditor4,StatPercentEditor5,StatPercentEditor6,StatPercentEditor7,StatPercentEditor8,StatPercentEditor9,StatPercentEditor10,",
        "StatModifierBonusStat1,StatModifierBonusStat2,StatModifierBonusStat3,StatModifierBonusStat4,StatModifierBonusStat5,StatModifierBonusStat6,StatModifierBonusStat7,StatModifierBonusStat8,StatModifierBonusStat9,StatModifierBonusStat10,",
        "Stackable,MaxCount,MinReputation,RequiredAbility,AllowableRace1,AllowableRace2,SellPrice,BuyPrice,VendorStackCount,PriceVariance,PriceRandomValue,Flags1,Flags2,Flags3,Flags4,Flags5,",
        "FactionRelated,ModifiedCraftingReagentItemID,ContentTuningID,PlayerLevelToItemLevelCurveID,ItemLevelOffsetCurveID,ItemLevelOffsetItemLevel,ItemSquishEraID,ItemNameDescriptionID,RequiredTransmogHoliday,RequiredHoliday,GemProperties,SocketMatchEnchantmentId,TotemCategoryID,InstanceBound,ZoneBound1,ZoneBound2,",
        "ItemSet,LockID,PageID,ItemDelay,MinFactionID,RequiredSkillRank,RequiredSkill,ItemLevel,AllowableClass,ArtifactID,SpellWeight,SpellWeightCategory,SocketType1,SocketType2,SocketType3,SheatheType,Material,PageMaterialID,Bonding,DamageDamageType,ContainerSlots,RequiredPVPMedal,RequiredPVPRank,RequiredLevel,InventoryType,OverallQualityID,AmmunitionType FROM item_sparse WHERE (VerifiedBuild>0)=?"
    ),
    "SELECT ID,LegacySlotIndex,TriggerType,Charges,CoolDownMSec,CategoryCoolDownMSec,SpellCategoryID,SpellID,ChrSpecializationID,PlayerConditionID FROM item_effect WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ItemEffectID,ItemID FROM item_x_item_effect WHERE (VerifiedBuild>0)=?",
];

struct NumericRow<'a> {
    row: &'a SqlResult,
    next: usize,
}
impl<'a> NumericRow<'a> {
    fn new(row: &'a SqlResult, columns: usize) -> Result<Self, LoadError> {
        if row.field_count() != columns {
            return Err(LoadError::InvalidRow);
        }
        Ok(Self { row, next: 0 })
    }
    fn read<T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>>(
        &mut self,
    ) -> Result<T, LoadError> {
        let value = self.row.try_read(self.next).ok_or(LoadError::InvalidRow)?;
        self.next += 1;
        Ok(value)
    }
    fn array<
        T: Copy + Default + for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
        const N: usize,
    >(
        &mut self,
    ) -> Result<[T; N], LoadError> {
        let mut values = [T::default(); N];
        for value in &mut values {
            *value = self.read()?;
        }
        Ok(values)
    }
    fn text(&mut self) -> Result<Vec<u8>, LoadError> {
        if self.next >= self.row.field_count() {
            return Err(LoadError::InvalidRow);
        }
        // C++ Field::GetStringView treats SQL NULL as an empty string. Preserve
        // raw binary-collation bytes; never use lossy read_string conversion.
        let value = if self.row.is_null(self.next) {
            Vec::new()
        } else {
            self.row
                .try_read::<Vec<u8>>(self.next)
                .or_else(|| {
                    self.row
                        .try_read::<String>(self.next)
                        .map(String::into_bytes)
                })
                .ok_or(LoadError::InvalidRow)?
        };
        self.next += 1;
        Ok(value)
    }
    fn finish(self) -> Result<(), LoadError> {
        if self.next == self.row.field_count() {
            Ok(())
        } else {
            Err(LoadError::InvalidRow)
        }
    }
}

impl ForeverHotfixRepository {
    /// Eight queries finish before publication. Legal repeated overlay IDs
    /// keep query order, never sort by ID/build or swallow failed queries.
    pub async fn load_item_overlays(&self) -> Result<ItemOverlays, LoadError> {
        Ok(ItemOverlays {
            official: self.item_batch(false).await?,
            custom: self.item_batch(true).await?,
        })
    }
    async fn item_batch(&self, custom: bool) -> Result<ItemRows, LoadError> {
        let mut rows = ItemRows::default();
        for (table, sql) in QUERIES.iter().enumerate() {
            let mut statement = PreparedStatement::new(*sql);
            statement.set_bool(0, !custom);
            let mut result = self
                .0
                .query(&statement)
                .await
                .map_err(|_| LoadError::Database)?;
            if result.is_empty() {
                continue;
            }
            loop {
                if table == 1 {
                    rows.sparse.push(sparse::read(&result)?);
                } else {
                    let mut r = NumericRow::new(&result, [17, 104, 10, 3][table])?;
                    match table {
                        0 => rows.items.push(ItemRow {
                            id: r.read()?,
                            class: r.read()?,
                            subclass: r.read()?,
                            material: r.read()?,
                            inventory_type: r.read()?,
                            sheathe: r.read()?,
                            pet_food: r.read()?,
                            sound_override: r.read()?,
                            icon_file: r.read()?,
                            group_sounds: r.read()?,
                            content_tuning: r.read()?,
                            modified_crafting_reagent: r.read()?,
                            unknown_1200: r.read()?,
                            crafting_quality: r.read()?,
                            squish_era: r.read()?,
                            recraft_reagent_percentage: r.read()?,
                            order_source: r.read()?,
                        }),
                        2 => rows.effects.push(ItemEffectRow {
                            id: r.read()?,
                            legacy_slot: r.read()?,
                            trigger: r.read()?,
                            charges: r.read()?,
                            cooldown: r.read()?,
                            category_cooldown: r.read()?,
                            spell_category: r.read()?,
                            spell: r.read()?,
                            specialization: r.read()?,
                            player_condition: r.read()?,
                        }),
                        3 => rows.relations.push(ItemRelationRow {
                            id: r.read()?,
                            effect: r.read()?,
                            item: r.read()?,
                        }),
                        _ => unreachable!(),
                    }
                    r.finish()?;
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests;
