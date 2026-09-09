//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn spell_acquisition_snapshot_adapter_is_exact_or_fails_closed() {
    use crate::spell_acquisition::{
        PlayerAcquisitionLifecycleLikeCpp, PlayerCastAcquisitionResolutionLikeCpp,
        PlayerFuturePlayerConditionResolutionLikeCpp, PlayerSkillPersistenceStateLikeCpp,
        PlayerSpellPersistenceStateLikeCpp, SpellAcquisitionSnapshotAdapterErrorLikeCpp,
    };

    let (mut session, _, _) = make_session();
    session.player_race = 1;
    session.set_player_class_like_cpp(1);
    session.set_player_level_like_cpp(40);
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id: 100,
                active: true,
                disabled: false,
                dependent: true,
                favorite: true,
                state: RepresentedPlayerSpellStateLikeCpp::Changed,
            },
            RepresentedPlayerSpellLikeCpp {
                spell_id: 200,
                active: false,
                disabled: true,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Removed,
            },
        ])
    );
    session
        .represented_spell_trait_definition_ids_like_cpp
        .insert(100, 7);
    session
        .represented_override_spells_like_cpp
        .entry(90)
        .or_default()
        .insert(100);
    assert!(session.set_complete_player_skill_records_like_cpp(
        HashMap::from([
            (
                333,
                RepresentedPlayerSkillLikeCpp {
                    skill_id: 333,
                    step: 2,
                    value: 150,
                    max: 225,
                    profession_slot: 0,
                    state: RepresentedPlayerSkillStateLikeCpp::Changed,
                },
            ),
            (
                755,
                RepresentedPlayerSkillLikeCpp {
                    skill_id: 755,
                    step: 0,
                    value: 0,
                    max: 0,
                    profession_slot: -1,
                    state: RepresentedPlayerSkillStateLikeCpp::Deleted,
                },
            ),
        ]),
        2,
    ));
    assert!(
        !session.set_player_skill_occupied_slots_like_cpp(3),
        "a complete snapshot cannot hide an occupied SkillLine identity"
    );
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::MissingSkillSlotOccupancy)
    );
    assert!(session.set_player_skill_occupied_slots_like_cpp(2));
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteTraitDefinitions),
        "a populated mirror is not proof that every C++ PlayerSpell trait ID was loaded"
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([(100, 7)]));
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteOverrides),
        "a populated m_overrideSpells mirror remains untrusted until its full source is replaced"
    );
    assert!(session.set_complete_represented_override_spells_like_cpp([(90, 100)]));
    let cast_resolutions = BTreeMap::from([(
        500,
        PlayerCastAcquisitionResolutionLikeCpp {
            reached_immediate_phase: true,
            executed_hit_target_effect_mask: 0b101,
            effective_effects: Vec::new(),
            executed_dual_wield_effects: Vec::new(),
        },
    )]);
    let future_player_condition_resolutions = vec![PlayerFuturePlayerConditionResolutionLikeCpp {
        condition_id: 55,
        allowed: true,
    }];

    let snapshot = session
        .spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            future_player_condition_resolutions.clone(),
            cast_resolutions.clone(),
        )
        .expect("explicit complete session state");
    assert_eq!(snapshot.race, 1);
    assert_eq!(snapshot.class, 1);
    assert_eq!(snapshot.level, 40);
    assert_eq!(snapshot.occupied_skill_slots, 2);
    assert_eq!(snapshot.overrides, vec![(90, 100)]);
    assert_eq!(
        snapshot.future_player_condition_resolutions,
        future_player_condition_resolutions
    );
    assert_eq!(snapshot.cast_resolutions, cast_resolutions);
    assert_eq!(
        snapshot.spells[0].state,
        PlayerSpellPersistenceStateLikeCpp::Changed
    );
    assert_eq!(snapshot.spells[0].trait_definition_id, Some(7));
    assert_eq!(
        snapshot.skills[0].state,
        PlayerSkillPersistenceStateLikeCpp::Changed
    );
    assert_eq!(
        snapshot.skills[1].state,
        PlayerSkillPersistenceStateLikeCpp::Deleted,
        "a C++ SKILL_DELETED row still owns its SkillLineID update-field slot"
    );

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        ObjectGuid::create_player(1, 42),
        "SnapshotOwner".to_string(),
        Position::ZERO,
        0,
        1,
        1,
        40,
        0,
    ));
    assert_eq!(
        session.complete_player_skill_occupied_slots_like_cpp(),
        Some(2),
        "attaching the runtime owner must retain exact slot occupancy authority"
    );
    let mut owner_snapshot = snapshot.clone();
    owner_snapshot.character_guid = Some(ObjectGuid::create_player(1, 42));
    assert_eq!(
        session
            .spell_acquisition_snapshot_like_cpp(
                PlayerAcquisitionLifecycleLikeCpp::InWorld,
                future_player_condition_resolutions,
                cast_resolutions.clone(),
            )
            .expect("controller attach preserves the complete immutable snapshot"),
        owner_snapshot,
        "the controller receives the exact skill rows and contributes its character identity"
    );

    session.player_skill_records_complete_like_cpp = false;
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteSkillRows)
    );
    session.player_skill_records_complete_like_cpp = true;

    session.player_skill_occupied_slots_like_cpp = None;
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::MissingSkillSlotOccupancy)
    );
    session.player_skill_occupied_slots_like_cpp = Some(2);

    session
        .represented_spell_trait_definition_ids_like_cpp
        .insert(999, 8);
    session
        .represented_spell_trait_definition_ids_like_cpp
        .insert(998, 9);
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::OrphanTraitDefinition { spell_id: 998 })
    );
    session
        .represented_spell_trait_definition_ids_like_cpp
        .remove(&999);
    session
        .represented_spell_trait_definition_ids_like_cpp
        .remove(&998);

    session
        .represented_spell_trait_definition_ids_like_cpp
        .insert(100, 0);
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(
            SpellAcquisitionSnapshotAdapterErrorLikeCpp::InvalidTraitDefinitionId {
                spell_id: 100,
                trait_definition_id: 0,
            }
        )
    );
    session
        .represented_spell_trait_definition_ids_like_cpp
        .insert(100, 7);

    session
        .represented_override_spells_like_cpp
        .entry(-1)
        .or_default()
        .insert(100);
    session
        .represented_override_spells_like_cpp
        .entry(-2)
        .or_default()
        .insert(200);
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(
            SpellAcquisitionSnapshotAdapterErrorLikeCpp::InvalidOverride {
                overridden_spell_id: -2,
                overriding_spell_id: 200,
            }
        )
    );
    session.represented_override_spells_like_cpp.remove(&-1);
    session.represented_override_spells_like_cpp.remove(&-2);

    session.reset_represented_talents_like_cpp();
    assert!(
        session.represented_override_spells_like_cpp().is_empty(),
        "a fresh character load cannot retain the previous C++ Player's runtime override edges"
    );
    assert!(
        session
            .represented_spell_trait_definition_ids_like_cpp()
            .is_empty(),
        "a fresh character load cannot retain the previous C++ PlayerSpell trait IDs"
    );
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteTraitDefinitions),
        "character-owned trait and override state is reconstructed after reset"
    );
    assert!(session.set_complete_represented_override_spells_like_cpp([]));
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    let exact_empty_auxiliary_snapshot = session
        .spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        )
        .expect("explicitly complete empty auxiliary maps are exact");
    assert!(exact_empty_auxiliary_snapshot.overrides.is_empty());
    assert!(
        exact_empty_auxiliary_snapshot
            .spells
            .iter()
            .all(|spell| spell.trait_definition_id.is_none())
    );
    assert!(session.represented_override_spells_like_cpp().is_empty());
    assert!(
        session
            .represented_spell_trait_definition_ids_like_cpp()
            .is_empty()
    );

    session.set_known_spells_like_cpp(vec![100]);
    assert_eq!(
        session.spell_acquisition_snapshot_like_cpp(
            PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            BTreeMap::new(),
        ),
        Err(SpellAcquisitionSnapshotAdapterErrorLikeCpp::IncompleteSpellRows)
    );
    assert!(
        session
            .complete_represented_spell_trait_definition_ids_like_cpp()
            .is_none()
    );
    assert!(
        session
            .complete_represented_override_spells_like_cpp()
            .is_none()
    );
}
#[test]
fn represented_favorite_known_spells_are_retained_only_for_known_spells_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_known_spells_like_cpp(vec![100, 200]);
    session.set_represented_favorite_known_spells_like_cpp(HashSet::from([100, 300]));

    assert_eq!(
        session.represented_favorite_known_spells_like_cpp(),
        HashSet::from([100])
    );

    session.remove_known_spell_like_cpp(100);
    assert!(
        session
            .represented_favorite_known_spells_like_cpp()
            .is_empty()
    );
}
#[test]
fn represented_spell_charge_restore_pops_last_charge_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.record_loaded_character_spell_charge_like_cpp(7, 1_001, 1_020);
    session.record_loaded_character_spell_charge_like_cpp(7, 1_002, 1_040);
    session.mark_represented_character_spell_charges_loaded_like_cpp();

    assert_eq!(
        session.apply_modify_spell_charges_effect_like_cpp(1, 7, player_guid),
        1
    );

    assert_eq!(
        session.represented_character_spell_charges_like_cpp[&7]
            .iter()
            .map(|charge| (
                charge.recharge_start_unix_secs,
                charge.recharge_end_unix_secs
            ))
            .collect::<Vec<_>>(),
        vec![(1_001, 1_020)]
    );
}
#[test]
fn character_talent_load_applies_active_spell_side_effects_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut talent = test_talent_entry_like_cpp(406, 0, 14_908);
    talent.spell_id = 14_908;
    talent.overrides_spell_id = 50_000;
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries([talent])));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        14_908,
        wow_data::SpellInfo {
            effects: vec![wow_data::SpellEffectInfo {
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL,
                effect_trigger_spell: 60_100,
                ..Default::default()
            }],
            ..test_spell_info_like_cpp(14_908)
        },
    );
    spell_store.insert(60_100, test_spell_info_like_cpp(60_100));
    session.set_spell_store(Arc::new(spell_store));
    let talent_tabs = install_test_talent_tab_store_like_cpp(&mut session);

    let mut known_spells = vec![14_914];
    let mut dependent_spells = HashSet::new();
    assert!(
        session.load_represented_talent_row_with_spell_side_effects_like_cpp(
            &talent_tabs,
            406,
            0,
            0,
            &mut known_spells,
            &mut dependent_spells,
        ),
        "C++ _LoadTalents calls AddTalent before _LoadSpells"
    );
    assert!(known_spells.contains(&14_908));
    assert!(known_spells.contains(&60_100));
    assert_eq!(dependent_spells, HashSet::from([14_908, 60_100]));
    assert!(
        session
            .represented_override_spells_like_cpp()
            .get(&50_000)
            .is_some_and(|spells| spells.contains(&14_908))
    );
}
#[tokio::test]
async fn spell_upgrade_heirloom_effect_uses_metadata_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 79);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let spell_id = 77_001;

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([HeirloomEntry {
        id: 1,
        source_text: "known".to_string(),
        item_id: 44_000,
        legacy_upgraded_item_id: 0,
        static_upgraded_item_id: 0,
        source_type_enum: 0,
        flags: 0,
        legacy_item_id: 0,
        upgrade_item_id: [90_001, 90_002, 0, 0, 0, 0],
        upgrade_item_bonus_list_id: [101, 202, 0, 0, 0, 0],
    }])));
    session.load_represented_account_heirlooms_like_cpp([(44_000, 0x01)]);
    session
        .add_player_heirloom_dynamic_fields_like_cpp(44_000, 0x01)
        .unwrap();
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                from_client: true,
                misc: [44_000, 0],
                cast_item_entry: Some(90_002),
                cast_item_battle_pet_modifiers: None,
                cast_flags_ex: CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP,
                original_cast_id: ObjectGuid::EMPTY,
                unit_target_battle_pet_companion_guid: None,
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented upgrade-heirloom spell row should execute");

    assert_eq!(
        session.account_heirloom_rows_like_cpp(),
        vec![(44_000, 0x03)]
    );
    assert_eq!(session.account_heirloom_bonus_like_cpp(44_000), 202);
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.heirloom_flags_like_cpp().to_vec())
            .unwrap(),
        vec![0x03]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn battle_pet_grant_experience_spell_effect_levels_without_active_criteria_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x194);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    session.set_represented_battle_pet_xp_per_level_like_cpp(23, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(24, 100);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 7,
            level: 23,
            exp: 20,
            flags: 6,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            150,
            RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
            9.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("experienced pet");
    assert_eq!(pet.level, 24);
    assert_eq!(pet.exp, 70);
    assert_eq!(pet.health, 1180);
    assert_eq!(pet.max_health, 1180);
    assert_eq!(pet.power, 126);
    assert_eq!(pet.speed, 81);
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &[RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 24
        }]
    );
    assert!(
        session
            .represented_battle_pet_active_level_criteria_like_cpp()
            .is_empty()
    );

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetUpdates as u16
    );
    assert_eq!(packet.read_uint32().expect("pet count"), 1);
    assert!(!packet.read_bit().expect("pet added"));
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.read_uint32().expect("species"), 11);
    assert_eq!(packet.read_uint32().expect("creature"), 22);
    assert_eq!(packet.read_uint32().expect("display"), 33);
    assert_eq!(packet.read_uint16().expect("breed"), 7);
    assert_eq!(packet.read_uint16().expect("level"), 24);
    assert_eq!(packet.read_uint16().expect("exp"), 70);
    assert_eq!(packet.read_uint16().expect("flags"), 6);
    assert_eq!(packet.read_uint32().expect("power"), 126);
    assert_eq!(packet.read_uint32().expect("health"), 1180);
    assert_eq!(packet.read_uint32().expect("max health"), 1180);
    assert_eq!(packet.read_uint32().expect("speed"), 81);
    assert_eq!(packet.read_uint8().expect("quality"), 3);
}
#[tokio::test]
async fn spell_effect_grant_battle_pet_experience_uses_unit_companion_guid_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 221);
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A1);
    let creature_guid = ObjectGuid::create_global(HighGuid::Creature, 0, 0xCAFF);
    let spell_id = 77_287;

    session.set_player_guid(Some(player_guid));
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    session.set_represented_battle_pet_xp_per_level_like_cpp(23, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(24, 100);
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 7,
            level: 23,
            exp: 20,
            flags: 6,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect:
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
                effect_base_points: 150,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: creature_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                unit_target_battle_pet_companion_guid: Some(pet_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented battle-pet XP spell effect should execute");

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("experienced pet");
    assert_eq!(pet.level, 24);
    assert_eq!(pet.exp, 70);
    assert_eq!(pet.health, 1180);
    assert_eq!(pet.max_health, 1180);
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &[RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 24
        }]
    );
    assert!(
        session
            .represented_battle_pet_active_level_criteria_like_cpp()
            .is_empty()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::BattlePetUpdates,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_effect_uncage_battle_pet_rejects_disappeared_cast_item_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 229);
    let item_guid = ObjectGuid::create_item(1, 900);
    let spell_id = 77_289;

    session.set_player_guid(Some(player_guid));
    install_represented_battle_pet_species_like_cpp(
        &mut session,
        11,
        9000,
        wow_data::BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP,
    );
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                cast_item_entry: Some(8_281),
                cast_item_battle_pet_modifiers: Some(SpellCastBattlePetItemModifiersLikeCpp {
                    source_item_guid: item_guid,
                    species_id: 11,
                    breed_data: 7 | (3 << 24),
                    level: 3,
                    display_id: 33,
                }),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented cast should finish without applying a missing cast item");

    assert!(session.represented_battle_pets_like_cpp.is_empty());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
