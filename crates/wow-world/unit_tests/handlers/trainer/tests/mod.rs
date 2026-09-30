use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::*;
use crate::session::{
    AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedPlayerSpellLikeCpp,
    RepresentedPlayerSpellStateLikeCpp, SessionPlayerController, SessionState,
};
use wow_constants::unit::UnitState;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_data::{
    ConditionEntriesByTypeStore, CreatureTrainerRowLikeCpp, EffectiveSpellAcquisitionRowsLikeCpp,
    MountStore, SkillLineAbilityRecord, SkillLineEntry, SkillLineStore, SkillRaceClassInfoRecord,
    SkillStore, SkillTiersRowLikeCpp, SkillTiersStoreLikeCpp, SpellAcquisitionCatalogLikeCpp,
    SpellAcquisitionCoverageSeedLikeCpp, SpellAcquisitionEffectLikeCpp,
    SpellAcquisitionMiscLikeCpp, SpellAcquisitionTableHashesLikeCpp, SpellChainStoreLikeCpp,
    SpellCustomAttributeStoreLikeCpp, SpellLearnSkillStoreLikeCpp, SpellLearnSpellStoreLikeCpp,
    SpellRequiredStoreLikeCpp, TrainerLocaleRowLikeCpp, TrainerRowLikeCpp, TrainerSpellLikeCpp,
    TrainerSpellRowLikeCpp,
};
use wow_packet::{ServerPacket, WorldPacket};

const CREATURE_ENTRY: u32 = 123;
const DEFAULT_TRAINER_ID: u32 = 7;
const KNOWN_TRAINER_SPELL: i32 = 54_321;
const AVAILABLE_TRAINER_SPELL: i32 = 54_322;
const UNAVAILABLE_TRAINER_SPELL: i32 = 54_323;
const WRAPPER_TRAINER_SPELL: i32 = 54_324;
const WRAPPER_LEARNED_SPELL: i32 = 54_325;

fn spell_target_restriction_row(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    target_creature_type: i16,
) -> wow_data::SpellTargetRestrictionsEntry {
    wow_data::SpellTargetRestrictionsEntry {
        id,
        difficulty_id,
        cone_degrees: 0.0,
        max_targets: 0,
        max_target_level: 0,
        target_creature_type,
        targets: 0,
        width: 0.0,
        spell_id,
    }
}

fn player_learn_effect(
    record_id: u32,
    wrapper_spell_id: u32,
    learned_spell_id: u32,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: i64::from(wrapper_spell_id),
        difficulty_id_raw: 0,
        effect_index_raw: 0,
        effect_type_raw: 36,
        effect_aura_raw: 0,
        effect_mechanic_raw: 0,
        effect_attributes_raw: 0,
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0_f32.to_bits(),
        effect_real_points_per_level_bits: 0.0_f32.to_bits(),
        effect_coefficient_bits: 0.0_f32.to_bits(),
        effect_variance_bits: 0.0_f32.to_bits(),
        effect_trigger_spell_raw: i64::from(learned_spell_id),
        effect_item_type_raw: 0,
        effect_misc_value_raw: [0, 0],
        implicit_target_raw: [1, 0],
    }
}

fn player_aura_effect(
    record_id: u32,
    spell_id: u32,
    aura_type: i64,
    aura_misc_value: i64,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: i64::from(spell_id),
        difficulty_id_raw: 0,
        effect_index_raw: 0,
        effect_type_raw: 6,
        effect_aura_raw: aura_type,
        effect_mechanic_raw: 0,
        effect_attributes_raw: 0,
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0_f32.to_bits(),
        effect_real_points_per_level_bits: 0.0_f32.to_bits(),
        effect_coefficient_bits: 0.0_f32.to_bits(),
        effect_variance_bits: 0.0_f32.to_bits(),
        effect_trigger_spell_raw: 0,
        effect_item_type_raw: 0,
        effect_misc_value_raw: [aura_misc_value, 0],
        implicit_target_raw: [1, 0],
    }
}

fn trainer_row(id: u32, trainer_type: u8, greeting: &str) -> TrainerRowLikeCpp {
    TrainerRowLikeCpp {
        id,
        trainer_type,
        greeting: greeting.to_string(),
    }
}

fn trainer_spell_row(
    trainer_id: u32,
    spell_id: i32,
    money_cost: u32,
    req_level: u8,
) -> TrainerSpellRowLikeCpp {
    TrainerSpellRowLikeCpp {
        trainer_id,
        spell: TrainerSpellLikeCpp {
            spell_id: spell_id as u32,
            money_cost,
            req_skill_line: 0,
            req_skill_rank: 0,
            req_ability: [0; 3],
            req_level,
        },
    }
}

fn trainer_store_from_rows(
    trainer_rows: Vec<TrainerRowLikeCpp>,
    spell_rows: Vec<TrainerSpellRowLikeCpp>,
    locale_rows: Vec<TrainerLocaleRowLikeCpp>,
    creature_rows: Vec<CreatureTrainerRowLikeCpp>,
) -> Arc<TrainerStoreLikeCpp> {
    Arc::new(
        TrainerStoreLikeCpp::from_rows_like_cpp(
            trainer_rows,
            spell_rows,
            locale_rows,
            creature_rows,
            |_| true,
            |_| true,
            |_| true,
            |_, _| true,
        )
        .store,
    )
}

fn standard_trainer_store(trainer_id: u32) -> Arc<TrainerStoreLikeCpp> {
    trainer_store_from_rows(
        vec![trainer_row(trainer_id, 2, "Train")],
        vec![
            trainer_spell_row(trainer_id, KNOWN_TRAINER_SPELL, 10, 1),
            trainer_spell_row(trainer_id, AVAILABLE_TRAINER_SPELL, 20, 80),
            trainer_spell_row(trainer_id, UNAVAILABLE_TRAINER_SPELL, 30, 81),
        ],
        Vec::new(),
        vec![CreatureTrainerRowLikeCpp {
            creature_id: CREATURE_ENTRY,
            trainer_id,
            menu_id: 0,
            option_id: 0,
        }],
    )
}

fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(32);
    (
        WorldSession::new(
            1,
            "TrainerTest".into(),
            0,
            2,
            9,
            54_261,
            vec![0; 40],
            "enUS".into(),
            pkt_rx,
            send_tx,
        ),
        send_rx,
    )
}

fn creature_guid(counter: u32) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, counter, 1)
}

fn insert_canonical_creature(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    x: f32,
    npc_flags: u32,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(CREATURE_ENTRY);
    creature.unit_mut().world_mut().set_map(0, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(x, 0.0, 0.0, 0.0));
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.unit_mut().world_mut().object_mut().add_to_world();

    manager
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .expect("canonical test map")
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

struct TrainerFixture {
    session: WorldSession,
    send_rx: flume::Receiver<Vec<u8>>,
    trainer: ObjectGuid,
    other_trainer: ObjectGuid,
    vendor: ObjectGuid,
}

fn assert_trainer_charge_and_visuals_like_cpp(fixture: &mut TrainerFixture) {
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        wow_packet::packets::update::UpdateObject::player_money_update(
            fixture.session.player_guid().unwrap(),
            fixture.session.player_map_id_like_cpp(),
            75,
            None,
        )
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        PlaySpellVisualKit {
            unit: fixture.trainer,
            kit_record_id: 179,
            kit_type: 0,
            duration: 0,
            mounted_visual: false,
        }
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        PlaySpellVisualKit {
            unit: fixture.session.player_guid().unwrap(),
            kit_record_id: 362,
            kit_type: 1,
            duration: 0,
            mounted_visual: false,
        }
        .to_bytes()
    );
    assert!(fixture.send_rx.try_recv().is_err());
}

fn trainer_fixture_with_store(store: Arc<TrainerStoreLikeCpp>) -> TrainerFixture {
    trainer_fixture_with_store_and_map_difficulty(store, 0)
}

fn trainer_fixture_with_store_and_map_difficulty(
    store: Arc<TrainerStoreLikeCpp>,
    map_difficulty: u8,
) -> TrainerFixture {
    let (mut session, send_rx) = make_session();
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    if map_difficulty != 0 {
        canonical.lock().unwrap().create_map_entry(
            0,
            0,
            map_difficulty,
            wow_map::ManagedMapKind::World,
        );
        session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
            wow_data::DifficultyEntry {
                id: u32::from(map_difficulty),
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ])));
    }
    let trainer = creature_guid(100);
    let other_trainer = creature_guid(101);
    let vendor = creature_guid(102);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_trainer_store_like_cpp(store);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_disable_mgr(Arc::new(wow_data::DisableMgrLikeCpp::default()));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        ObjectGuid::create_player(1, 42),
        "TrainerTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_skill_store(Arc::new(
        SkillStore::from_skill_line_abilities_and_race_class_like_cpp([], []),
    ));
    session.set_skill_line_store(Arc::new(SkillLineStore::from_entries([])));
    session.set_skill_tiers_store(Arc::new(SkillTiersStoreLikeCpp::default()));
    session.set_trait_definition_store(Arc::new(
        wow_data::trait_tree::TraitDefinitionStore::from_entries([]),
    ));
    session.set_mount_store(Arc::new(MountStore::from_entries([])));
    session.set_spell_chain_store(Arc::new(SpellChainStoreLikeCpp::default()));
    session.set_spell_custom_attribute_store(Arc::new(SpellCustomAttributeStoreLikeCpp::default()));
    let mut learn_skills = SpellLearnSkillStoreLikeCpp::default();
    learn_skills.covered_spell_ids.extend([
        KNOWN_TRAINER_SPELL as u32,
        AVAILABLE_TRAINER_SPELL as u32,
        UNAVAILABLE_TRAINER_SPELL as u32,
    ]);
    session.set_spell_learn_skill_store(Arc::new(learn_skills));
    session.set_spell_learn_spell_store(Arc::new(SpellLearnSpellStoreLikeCpp::default()));
    session.set_spell_required_store(Arc::new(SpellRequiredStoreLikeCpp::default()));
    session.set_spell_linked_store(Arc::new(wow_data::SpellLinkedStoreLikeCpp::default()));
    session.set_spell_pet_aura_store(Arc::new(wow_data::SpellPetAuraStoreLikeCpp::default()));
    session.set_spell_target_restrictions_store(Arc::new(
        wow_data::SpellTargetRestrictionsStore::from_entries([]),
    ));
    session.set_spell_aura_restrictions_store(Arc::new(
        wow_data::SpellAuraRestrictionsStore::from_entries([]),
    ));
    session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [
                KNOWN_TRAINER_SPELL as u32,
                AVAILABLE_TRAINER_SPELL as u32,
                UNAVAILABLE_TRAINER_SPELL as u32,
            ]
            .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
            EffectiveSpellAcquisitionRowsLikeCpp::default(),
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    session.set_known_spells_like_cpp(vec![KNOWN_TRAINER_SPELL]);
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id: KNOWN_TRAINER_SPELL,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical player map");
    assert!(session.set_player_aura_authority_complete_like_cpp(true));
    // The production login path hydrates `Player::SetFactionForRace`
    // before publishing the Player. This synthetic fixture has no
    // ChrRaces store, so install the exact canonical prerequisite here.
    session.set_player_faction_template_like_cpp(1);
    assert!(session.set_complete_player_skill_records_like_cpp(HashMap::new(), 0));
    insert_canonical_creature(&canonical, trainer, 1.0, TRAINER_BUY_NPC_FLAGS_LIKE_CPP);
    insert_canonical_creature(
        &canonical,
        other_trainer,
        2.0,
        TRAINER_BUY_NPC_FLAGS_LIKE_CPP,
    );
    insert_canonical_creature(&canonical, vendor, 3.0, NPCFlags1::VENDOR.bits());

    TrainerFixture {
        session,
        send_rx,
        trainer,
        other_trainer,
        vendor,
    }
}

fn trainer_fixture() -> TrainerFixture {
    trainer_fixture_with_store(standard_trainer_store(DEFAULT_TRAINER_ID))
}

fn trainer_wrapper_fixture() -> TrainerFixture {
    trainer_wrapper_fixture_with_map_difficulty(0)
}

fn trainer_wrapper_fixture_with_map_difficulty(map_difficulty: u8) -> TrainerFixture {
    let store = trainer_store_from_rows(
        vec![trainer_row(DEFAULT_TRAINER_ID, 2, "Train")],
        vec![trainer_spell_row(
            DEFAULT_TRAINER_ID,
            WRAPPER_TRAINER_SPELL,
            25,
            1,
        )],
        Vec::new(),
        vec![CreatureTrainerRowLikeCpp {
            creature_id: CREATURE_ENTRY,
            trainer_id: DEFAULT_TRAINER_ID,
            menu_id: 0,
            option_id: 0,
        }],
    );
    let mut fixture = trainer_fixture_with_store_and_map_difficulty(store, map_difficulty);
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let learned_id = WRAPPER_LEARNED_SPELL as u32;
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [wrapper_id, learned_id]
                .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![player_learn_effect(1, wrapper_id, learned_id)],
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    let mut learn_skills = SpellLearnSkillStoreLikeCpp::default();
    learn_skills
        .covered_spell_ids
        .extend([wrapper_id, learned_id]);
    fixture
        .session
        .set_spell_learn_skill_store(Arc::new(learn_skills));
    fixture
        .session
        .set_spell_acquisition_static_authority_like_cpp([wrapper_id], []);
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(100);
    fixture
        .session
        .set_loot_money_persistence_test_result_like_cpp(true);
    fixture
}

fn trainer_buy_packet(trainer_guid: ObjectGuid, trainer_id: i32, spell_id: i32) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&trainer_guid);
    packet.write_int32(trainer_id);
    packet.write_int32(spell_id);
    packet.reset_read();
    packet
}

fn seed_feign_death(session: &mut WorldSession, slot: u8) {
    let player_guid = session.player_guid().expect("active player");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().add_unit_state(UnitState::DIED.bits());
        })
        .expect("canonical player");
    assert!(
        session.insert_player_visible_aura_like_cpp(AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::FeignDeath),
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        })
    );
    assert!(session.set_player_aura_authority_complete_like_cpp(true));
}

fn seed_unclassified_active_aura(session: &mut WorldSession, slot: u8) {
    let player_guid = session.player_guid().expect("active player");
    assert!(
        session.insert_player_visible_aura_like_cpp(AuraApplication {
            spell_id: 999,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        })
    );
    assert!(session.set_player_aura_authority_complete_like_cpp(true));
}

fn install_wrapper_and_aura_catalog(
    session: &mut WorldSession,
    aura_spell_id: u32,
    aura_type: i64,
    aura_misc_value: i64,
    wrapper_effect_attributes: i64,
    wrapper_spell_attributes: i64,
) {
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let learned_id = WRAPPER_LEARNED_SPELL as u32;
    let mut learn_effect = player_learn_effect(1, wrapper_id, learned_id);
    learn_effect.effect_attributes_raw = wrapper_effect_attributes;
    session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [wrapper_id, learned_id, aura_spell_id]
                .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    learn_effect,
                    player_aura_effect(2, aura_spell_id, aura_type, aura_misc_value),
                ],
                spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                    record_id: 3,
                    spell_id_raw: i64::from(wrapper_id),
                    difficulty_id_raw: 0,
                    attributes_raw: [wrapper_spell_attributes, 0],
                    show_future_spell_player_condition_id_raw: 0,
                }],
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
}

fn install_aura_link(session: &mut WorldSession, aura_spell_id: u32, effect: i32) {
    let mut linked = wow_data::SpellLinkedStoreLikeCpp::default();
    linked.effects_by_type_and_trigger.insert(
        (wow_data::SpellLinkedTypeLikeCpp::Aura, aura_spell_id),
        vec![effect],
    );
    session.set_spell_linked_store(Arc::new(linked));
}

fn canonical_player_has_died_state(session: &mut WorldSession) -> bool {
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit().has_unit_state(UnitState::DIED.bits())
        })
        .expect("canonical player")
}

mod admission;
mod cast;
mod failures;
mod immunity;
mod purchase;
