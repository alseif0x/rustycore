//! Miscellaneous generated-style DB2 readers still required for full C++ store parity.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "misc_generated/tests/mod.rs"]
mod tests;

macro_rules! db2_store {
    ($store:ident, $entry:ty) => {
        pub struct $store {
            entries: HashMap<u32, $entry>,
        }

        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                }
            }

            pub fn get(&self, id: u32) -> Option<&$entry> {
                self.entries.get(&id)
            }

            pub fn entries(&self) -> impl Iterator<Item = &$entry> {
                self.entries.values()
            }

            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

db2_store!(AdventureJournalStore, AdventureJournalEntry);
db2_store!(AdventureMapPoiStore, AdventureMapPoiEntry);
db2_store!(BannedAddonsStore, BannedAddonsEntry);
db2_store!(BroadcastTextStore, BroadcastTextEntry);
db2_store!(CfgCategoriesStore, CfgCategoriesEntry);
db2_store!(CfgRegionsStore, CfgRegionsEntry);
db2_store!(ChatChannelsStore, ChatChannelsEntry);
db2_store!(CinematicCameraStore, CinematicCameraEntry);
db2_store!(CinematicSequencesStore, CinematicSequencesEntry);
db2_store!(ConditionalChrModelStore, ConditionalChrModelEntry);
db2_store!(ConditionalContentTuningStore, ConditionalContentTuningEntry);
db2_store!(ExpectedStatStore, ExpectedStatEntry);
db2_store!(ExpectedStatModStore, ExpectedStatModEntry);
db2_store!(GarrAbilityStore, GarrAbilityEntry);
db2_store!(GarrBuildingStore, GarrBuildingEntry);
db2_store!(GarrBuildingPlotInstStore, GarrBuildingPlotInstEntry);
db2_store!(GarrClassSpecStore, GarrClassSpecEntry);
db2_store!(GarrFollowerStore, GarrFollowerEntry);
db2_store!(GarrFollowerXAbilityStore, GarrFollowerXAbilityEntry);
db2_store!(GarrMissionStore, GarrMissionEntry);
db2_store!(GarrPlotStore, GarrPlotEntry);
db2_store!(GarrPlotBuildingStore, GarrPlotBuildingEntry);
db2_store!(GarrPlotInstanceStore, GarrPlotInstanceEntry);
db2_store!(GarrSiteLevelStore, GarrSiteLevelEntry);
db2_store!(GarrSiteLevelPlotInstStore, GarrSiteLevelPlotInstEntry);
db2_store!(GarrTalentTreeStore, GarrTalentTreeEntry);
db2_store!(GemPropertiesStore, GemPropertiesEntry);
db2_store!(GossipNpcOptionStore, GossipNpcOptionEntry);
db2_store!(GuildColorBackgroundStore, GuildColorEntry);
db2_store!(GuildColorBorderStore, GuildColorEntry);
db2_store!(GuildColorEmblemStore, GuildColorEntry);
db2_store!(GuildPerkSpellsStore, GuildPerkSpellsEntry);
db2_store!(HolidaysStore, HolidaysEntry);
db2_store!(KeychainStore, KeychainEntry);
db2_store!(KeystoneAffixStore, KeystoneAffixEntry);
db2_store!(LfgDungeonsStore, LfgDungeonsEntry);
db2_store!(LanguageWordsStore, LanguageWordsEntry);
db2_store!(LanguagesStore, LanguagesEntry);
db2_store!(MailTemplateStore, MailTemplateEntry);
db2_store!(MovieStore, MovieEntry);
db2_store!(MythicPlusSeasonStore, MythicPlusSeasonEntry);
db2_store!(NamesProfanityStore, NamesProfanityEntry);
db2_store!(NamesReservedStore, NamesReservedEntry);
db2_store!(NamesReservedLocaleStore, NamesReservedLocaleEntry);
db2_store!(OverrideSpellDataStore, OverrideSpellDataEntry);
db2_store!(PvpDifficultyStore, PvpDifficultyEntry);
db2_store!(PrestigeLevelInfoStore, PrestigeLevelInfoEntry);
db2_store!(ScenarioStore, ScenarioEntry);
db2_store!(SceneScriptStore, SceneScriptEntry);
db2_store!(SceneScriptGlobalTextStore, SceneScriptTextEntry);
db2_store!(SceneScriptTextStore, SceneScriptTextEntry);
db2_store!(ServerMessagesStore, ServerMessagesEntry);
db2_store!(SoundKitStore, SoundKitEntry);
db2_store!(SpecSetMemberStore, SpecSetMemberEntry);
db2_store!(SpecializationSpellsStore, SpecializationSpellsEntry);
db2_store!(SummonPropertiesStore, SummonPropertiesEntry);
db2_store!(TactKeyStore, TactKeyEntry);
db2_store!(TotemCategoryStore, TotemCategoryEntry);

macro_rules! impl_guild_color_load {
    ($store:ident, $file:literal) => {
        impl $store {
            pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
                load_store(data_dir, locale, $file, |id, idx, r| GuildColorEntry {
                    id,
                    red: r.get_field_u8(idx, 0),
                    blue: r.get_field_u8(idx, 1),
                    green: r.get_field_u8(idx, 2),
                })
            }
        }
    };
}

impl_guild_color_load!(GuildColorBackgroundStore, "GuildColorBackground.db2");
impl_guild_color_load!(GuildColorBorderStore, "GuildColorBorder.db2");
impl_guild_color_load!(GuildColorEmblemStore, "GuildColorEmblem.db2");

macro_rules! impl_scene_text_load {
    ($store:ident, $file:literal) => {
        impl $store {
            pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
                load_store(data_dir, locale, $file, |id, idx, r| SceneScriptTextEntry {
                    id,
                    name: r.get_field_string(idx, 0),
                    script: r.get_field_string(idx, 1),
                })
            }
        }
    };
}

impl_scene_text_load!(SceneScriptGlobalTextStore, "SceneScriptGlobalText.db2");
impl_scene_text_load!(SceneScriptTextStore, "SceneScriptText.db2");

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }

            fn len(&self) -> usize {
                self.len()
            }
        }
    };
}

impl_from_entries!(AdventureJournalStore, AdventureJournalEntry);
impl_from_entries!(AdventureMapPoiStore, AdventureMapPoiEntry);
impl_from_entries!(BannedAddonsStore, BannedAddonsEntry);
impl_from_entries!(BroadcastTextStore, BroadcastTextEntry);
impl_from_entries!(CfgCategoriesStore, CfgCategoriesEntry);
impl_from_entries!(CfgRegionsStore, CfgRegionsEntry);
impl_from_entries!(ChatChannelsStore, ChatChannelsEntry);
impl_from_entries!(CinematicCameraStore, CinematicCameraEntry);
impl_from_entries!(CinematicSequencesStore, CinematicSequencesEntry);
impl_from_entries!(ConditionalChrModelStore, ConditionalChrModelEntry);
impl_from_entries!(ConditionalContentTuningStore, ConditionalContentTuningEntry);
impl_from_entries!(ExpectedStatStore, ExpectedStatEntry);
impl_from_entries!(ExpectedStatModStore, ExpectedStatModEntry);
impl_from_entries!(GarrAbilityStore, GarrAbilityEntry);
impl_from_entries!(GarrBuildingStore, GarrBuildingEntry);
impl_from_entries!(GarrBuildingPlotInstStore, GarrBuildingPlotInstEntry);
impl_from_entries!(GarrClassSpecStore, GarrClassSpecEntry);
impl_from_entries!(GarrFollowerStore, GarrFollowerEntry);
impl_from_entries!(GarrFollowerXAbilityStore, GarrFollowerXAbilityEntry);
impl_from_entries!(GarrMissionStore, GarrMissionEntry);
impl_from_entries!(GarrPlotStore, GarrPlotEntry);
impl_from_entries!(GarrPlotBuildingStore, GarrPlotBuildingEntry);
impl_from_entries!(GarrPlotInstanceStore, GarrPlotInstanceEntry);
impl_from_entries!(GarrSiteLevelStore, GarrSiteLevelEntry);
impl_from_entries!(GarrSiteLevelPlotInstStore, GarrSiteLevelPlotInstEntry);
impl_from_entries!(GarrTalentTreeStore, GarrTalentTreeEntry);
impl_from_entries!(GemPropertiesStore, GemPropertiesEntry);
impl_from_entries!(GossipNpcOptionStore, GossipNpcOptionEntry);
impl_from_entries!(GuildColorBackgroundStore, GuildColorEntry);
impl_from_entries!(GuildColorBorderStore, GuildColorEntry);
impl_from_entries!(GuildColorEmblemStore, GuildColorEntry);
impl_from_entries!(GuildPerkSpellsStore, GuildPerkSpellsEntry);
impl_from_entries!(HolidaysStore, HolidaysEntry);
impl_from_entries!(KeychainStore, KeychainEntry);
impl_from_entries!(KeystoneAffixStore, KeystoneAffixEntry);
impl_from_entries!(LfgDungeonsStore, LfgDungeonsEntry);
impl_from_entries!(LanguageWordsStore, LanguageWordsEntry);
impl_from_entries!(LanguagesStore, LanguagesEntry);
impl_from_entries!(MailTemplateStore, MailTemplateEntry);
impl_from_entries!(MovieStore, MovieEntry);
impl_from_entries!(MythicPlusSeasonStore, MythicPlusSeasonEntry);
impl_from_entries!(NamesProfanityStore, NamesProfanityEntry);
impl_from_entries!(NamesReservedStore, NamesReservedEntry);
impl_from_entries!(NamesReservedLocaleStore, NamesReservedLocaleEntry);
impl_from_entries!(OverrideSpellDataStore, OverrideSpellDataEntry);
impl_from_entries!(PvpDifficultyStore, PvpDifficultyEntry);
impl_from_entries!(PrestigeLevelInfoStore, PrestigeLevelInfoEntry);
impl_from_entries!(ScenarioStore, ScenarioEntry);
impl_from_entries!(SceneScriptStore, SceneScriptEntry);
impl_from_entries!(SceneScriptGlobalTextStore, SceneScriptTextEntry);
impl_from_entries!(SceneScriptTextStore, SceneScriptTextEntry);
impl_from_entries!(ServerMessagesStore, ServerMessagesEntry);
impl_from_entries!(SoundKitStore, SoundKitEntry);
impl_from_entries!(SpecSetMemberStore, SpecSetMemberEntry);
impl_from_entries!(SpecializationSpellsStore, SpecializationSpellsEntry);
impl_from_entries!(SummonPropertiesStore, SummonPropertiesEntry);
impl_from_entries!(TactKeyStore, TactKeyEntry);
impl_from_entries!(TotemCategoryStore, TotemCategoryEntry);
