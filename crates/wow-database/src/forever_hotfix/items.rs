//! Full numeric/text official/custom batches, not a transactional snapshot.
//! Target source 02245dcd HotfixDatabase.cpp:870-872,980-981,1067-1081,1104.
mod locales;
mod sparse;
mod storage_bounds;
use super::{ForeverHotfixRepository, LoadError, PreparedStatement};
use crate::SqlResult;
use wow_persistence::forever::items::{
    AzeriteEmpoweredItemRow, ItemBonusRow, ItemEffectRow, ItemOffsetCurveRow, ItemOverlays,
    ItemRelationRow, ItemRow, ItemRows, ItemScalingConfigRow, ItemSquishEraRow,
};

const QUERIES: [&str; 9] = [
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
    "SELECT ID,Value1,Value2,Value3,Value4,ParentItemBonusListID,Type,OrderIndex FROM item_bonus WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ItemID,AzeriteTierUnlockSetID,AzeritePowerSetID FROM azerite_empowered_item WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ItemOffsetCurveID,ItemLevel,RequiredLevel,ItemSquishEraID,Flags FROM item_scaling_config WHERE (VerifiedBuild>0)=?",
    "SELECT ID,CurveID,`Offset` FROM item_offset_curve WHERE (VerifiedBuild>0)=?",
    "SELECT ID,Patch,CurveID,Flags FROM item_squish_era WHERE (VerifiedBuild>0)=?",
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
    /// Official and custom nine-table batches finish before publication.
    /// Repeated overlay IDs keep query order; failures are never swallowed.
    pub async fn load_item_overlays(&self) -> Result<ItemOverlays, LoadError> {
        Ok(ItemOverlays {
            official: self.item_batch(false).await?,
            custom: self.item_batch(true).await?,
        })
    }
    async fn item_batch(&self, custom: bool) -> Result<ItemRows, LoadError> {
        let mut rows = ItemRows::default();
        for (table, sql) in QUERIES.iter().enumerate() {
            eprintln!("item overlay query: table={table} custom={custom}");
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
            if table == 8 {
                if result.field_count() != 4 {
                    return Err(LoadError::InvalidRow);
                }
                let statement = PreparedStatement::new(storage_bounds::ITEM_SQUISH_SIZE_QUERY);
                let size = self
                    .0
                    .query(&statement)
                    .await
                    .map_err(|_| LoadError::Database)?;
                rows.item_squish_era_sql_index_size = Some(storage_bounds::decode(&size)?);
            }
            loop {
                if table == 1 {
                    rows.sparse.push(sparse::read(&result)?);
                } else {
                    let mut r = NumericRow::new(&result, [17, 104, 10, 3, 8, 4, 6, 3, 4][table])?;
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
                        4 => rows.item_bonuses.push(ItemBonusRow {
                            id: r.read()?,
                            values: r.array()?,
                            parent: r.read()?,
                            bonus_type: r.read()?,
                            order: r.read()?,
                        }),
                        5 => rows.azerite_empowered_items.push(AzeriteEmpoweredItemRow {
                            id: r.read()?,
                            item: r.read()?,
                            tier_unlock_set: r.read()?,
                            power_set: r.read()?,
                        }),
                        6 => rows.item_scaling_configs.push(ItemScalingConfigRow {
                            id: r.read()?,
                            offset_curve: r.read()?,
                            item_level: r.read()?,
                            required_level: r.read()?,
                            squish_era: r.read()?,
                            flags: r.read()?,
                        }),
                        7 => rows.item_offset_curves.push(ItemOffsetCurveRow {
                            id: r.read()?,
                            curve: r.read()?,
                            offset: r.read()?,
                        }),
                        8 => rows.item_squish_eras.push(ItemSquishEraRow {
                            id: r.read()?,
                            patch: r.read()?,
                            curve: r.read()?,
                            flags: r.read()?,
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
