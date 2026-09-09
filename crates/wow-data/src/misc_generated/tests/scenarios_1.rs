//! Miscellaneous DB2 readers regression scenarios, part 1 of 1.
//!
//! Moved out of the misc_generated.rs root under #664; every test is unchanged.

use super::*;

#[test]
fn adventure_map_poi_find_start_quest_matches_quest_and_condition_like_cpp() {
    let store = AdventureMapPoiStore::from_entries([
        adventure_map_poi(20, 900, 44),
        adventure_map_poi(10, 900, 43),
        adventure_map_poi(30, 901, 43),
    ]);

    let selected = store
        .find_start_quest_poi_like_cpp(900, |condition_id| condition_id == 44)
        .expect("matching adventure map poi");

    assert_eq!(selected.id, 20);
    assert!(
        store
            .find_start_quest_poi_like_cpp(900, |condition_id| condition_id == 99)
            .is_none()
    );
}

#[test]
fn load_misc_generated_db2_subbatch_when_fixtures_exist() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
    if !dbc_dir.exists() {
        eprintln!(
            "Skipping test: DB2 fixture directory not found at {}",
            dbc_dir.display()
        );
        return;
    }

    macro_rules! load_if_exists {
        ($file:literal, $store:ty) => {
            if dbc_dir.join($file).exists() {
                let _store = <$store>::load(data_dir, locale)
                    .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
            }
        };
    }

    load_if_exists!("AdventureJournal.db2", AdventureJournalStore);
    load_if_exists!("AdventureMapPOI.db2", AdventureMapPoiStore);
    load_if_exists!("BannedAddons.db2", BannedAddonsStore);
    load_if_exists!("BroadcastText.db2", BroadcastTextStore);
    load_if_exists!("Cfg_Categories.db2", CfgCategoriesStore);
    load_if_exists!("Cfg_Regions.db2", CfgRegionsStore);
    load_if_exists!("ChatChannels.db2", ChatChannelsStore);
    load_if_exists!("CinematicCamera.db2", CinematicCameraStore);
    load_if_exists!("CinematicSequences.db2", CinematicSequencesStore);
    load_if_exists!("ConditionalChrModel.db2", ConditionalChrModelStore);
    load_if_exists!(
        "ConditionalContentTuning.db2",
        ConditionalContentTuningStore
    );
    load_if_exists!("ExpectedStat.db2", ExpectedStatStore);
    load_if_exists!("ExpectedStatMod.db2", ExpectedStatModStore);
    load_if_exists!("GarrAbility.db2", GarrAbilityStore);
    load_if_exists!("GarrBuilding.db2", GarrBuildingStore);
    load_if_exists!("GarrBuildingPlotInst.db2", GarrBuildingPlotInstStore);
    load_if_exists!("GarrClassSpec.db2", GarrClassSpecStore);
    load_if_exists!("GarrFollower.db2", GarrFollowerStore);
    load_if_exists!("GarrFollowerXAbility.db2", GarrFollowerXAbilityStore);
    load_if_exists!("GarrMission.db2", GarrMissionStore);
    load_if_exists!("GarrPlot.db2", GarrPlotStore);
    load_if_exists!("GarrPlotBuilding.db2", GarrPlotBuildingStore);
    load_if_exists!("GarrPlotInstance.db2", GarrPlotInstanceStore);
    load_if_exists!("GarrSiteLevel.db2", GarrSiteLevelStore);
    load_if_exists!("GarrSiteLevelPlotInst.db2", GarrSiteLevelPlotInstStore);
    load_if_exists!("GarrTalentTree.db2", GarrTalentTreeStore);
    load_if_exists!("GemProperties.db2", GemPropertiesStore);
    load_if_exists!("GossipNPCOption.db2", GossipNpcOptionStore);
    load_if_exists!("GuildColorBackground.db2", GuildColorBackgroundStore);
    load_if_exists!("GuildColorBorder.db2", GuildColorBorderStore);
    load_if_exists!("GuildColorEmblem.db2", GuildColorEmblemStore);
    load_if_exists!("GuildPerkSpells.db2", GuildPerkSpellsStore);
    load_if_exists!("Holidays.db2", HolidaysStore);
    load_if_exists!("Keychain.db2", KeychainStore);
    load_if_exists!("KeystoneAffix.db2", KeystoneAffixStore);
    load_if_exists!("LFGDungeons.db2", LfgDungeonsStore);
    load_if_exists!("LanguageWords.db2", LanguageWordsStore);
    load_if_exists!("Languages.db2", LanguagesStore);
    load_if_exists!("MailTemplate.db2", MailTemplateStore);
    load_if_exists!("Movie.db2", MovieStore);
    load_if_exists!("MythicPlusSeason.db2", MythicPlusSeasonStore);
    load_if_exists!("NamesProfanity.db2", NamesProfanityStore);
    load_if_exists!("NamesReserved.db2", NamesReservedStore);
    load_if_exists!("NamesReservedLocale.db2", NamesReservedLocaleStore);
    load_if_exists!("OverrideSpellData.db2", OverrideSpellDataStore);
    load_if_exists!("PVPDifficulty.db2", PvpDifficultyStore);
    load_if_exists!("PVPItem.db2", PvpItemStore);
    load_if_exists!("PrestigeLevelInfo.db2", PrestigeLevelInfoStore);
    load_if_exists!("Scenario.db2", ScenarioStore);
    load_if_exists!("SceneScript.db2", SceneScriptStore);
    load_if_exists!("SceneScriptGlobalText.db2", SceneScriptGlobalTextStore);
    load_if_exists!("SceneScriptText.db2", SceneScriptTextStore);
    load_if_exists!("ServerMessages.db2", ServerMessagesStore);
    load_if_exists!("SoundKit.db2", SoundKitStore);
    load_if_exists!("SpecSetMember.db2", SpecSetMemberStore);
    load_if_exists!("SpecializationSpells.db2", SpecializationSpellsStore);
    load_if_exists!("SummonProperties.db2", SummonPropertiesStore);
    load_if_exists!("TactKey.db2", TactKeyStore);
    load_if_exists!("TotemCategory.db2", TotemCategoryStore);
}

#[test]
fn pvp_item_store_bonus_lookup_matches_cpp_item_id_map() {
    let store = PvpItemStore::from_entries([
        PvpItemEntry {
            id: 10,
            item_id: 1_000,
            item_level_delta: 7,
        },
        PvpItemEntry {
            id: 11,
            item_id: 2_000,
            item_level_delta: 13,
        },
        PvpItemEntry {
            id: 12,
            item_id: 1_000,
            item_level_delta: 21,
        },
    ]);

    assert_eq!(
        store.item_level_bonus_like_cpp(1_000),
        21,
        "C++ DB2Manager::_pvpItemBonus is keyed by PVPItem.ItemID and later duplicate rows overwrite earlier ones"
    );
    assert_eq!(store.item_level_bonus_like_cpp(2_000), 13);
    assert_eq!(store.item_level_bonus_like_cpp(9_999), 0);
}
