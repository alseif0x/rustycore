//! Direct rule tests for `WorldSession::can_take_quest`.
//!
//! Target C++ SHA `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
//! `Player::CanTakeQuest` (Player.cpp:14087-14096), `SatisfyQuestSkill`
//! (15009-15037), `SatisfyQuestDependentPreviousQuests` (15115-15171),
//! `SatisfyQuestDependentBreadcrumbQuests` (15197-15216),
//! `SatisfyQuestReputation` (15256-15283), `SatisfyQuestConditions`
//! (15311-15325), `SatisfyQuestExclusiveGroup` (15342-15385),
//! `SatisfyQuestDay` (15387-15401), `SatisfyQuestWeek` (15403-15410),
//! `SatisfyQuestExpansion` (15425-15437), and `SatisfyQuestMonth`
//! (15439-15446). These tests construct a synthetic session without a canonical
//! Player handle and exercise its handle-less test-state fallback. They do not
//! prove Player loading, real quest/reputation/skill hydration, cooldown reset,
//! timed or seasonal quest state, persistence, or production composition parity.

use super::*;
use crate::handlers::quest::PlayerQuestStatus;
use std::sync::Arc;
use wow_constants::quest::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
use wow_constants::{ComparisonType, ConditionSourceType, ConditionType};
use wow_data::progression_rewards::{FactionEntry, FactionStore};
use wow_data::quest::{
    QUEST_FLAGS_DAILY_LIKE_CPP, QUEST_FLAGS_WEEKLY_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP, QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP, QuestStore,
    QuestTemplate,
};
use wow_data::{Condition, ConditionEntriesByTypeStore};

fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(8);
    let mut session = WorldSession::new(
        1,
        "QuestStatusTest".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "enUS".into(),
        pkt_rx,
        send_tx,
    );
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    (session, send_rx)
}

fn quest_template(id: u32) -> QuestTemplate {
    QuestTemplate {
        id,
        quest_type: 2,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; wow_data::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
        log_title: format!("Quest {id}"),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
    }
}

fn install_quest(session: &mut WorldSession, quest: &QuestTemplate) {
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest.clone()])));
}

fn seed_rewarded_quest(session: &mut WorldSession, quest_id: u32) {
    session
        .quest_test_fixture_like_cpp
        .rewarded_quests
        .insert(quest_id);
}

fn seed_active_quest(session: &mut WorldSession, quest_id: u32) {
    session
        .quest_test_fixture_like_cpp
        .player_quests
        .insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
}

fn set_reputation_standing(session: &mut WorldSession, reputation_list_id: u32, standing: i32) {
    session
        .mutate_reputation_mgr_like_cpp(|manager| {
            manager
                .get_state_mut(reputation_list_id)
                .expect("reputation state")
                .standing = standing;
        })
        .expect("reputation state");
}

fn set_skill_values(session: &mut WorldSession, skill_values: std::collections::HashMap<u16, u16>) {
    let _ = session.set_player_skill_values_like_cpp(skill_values);
}

#[test]
fn can_take_quest_blocks_daily_already_completed_like_cpp() {
    // C++ SatisfyQuestDay: daily IDs are checked in DailyQuestsCompleted.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7600u32);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    install_quest(&mut session, &quest);
    session
        .quest_test_fixture_like_cpp
        .daily_quests_completed_like_cpp
        .insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_allows_daily_not_yet_completed_like_cpp() {
    // C++ SatisfyQuestDay permits a daily absent from DailyQuestsCompleted.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7601u32);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    install_quest(&mut session, &quest);

    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_df_quest_already_completed_like_cpp() {
    // C++ SatisfyQuestDay checks the separate DFQuests set for DF quests.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7602u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    install_quest(&mut session, &quest);
    session
        .quest_test_fixture_like_cpp
        .df_quests_like_cpp
        .insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_weekly_already_completed_like_cpp() {
    // C++ SatisfyQuestWeek rejects IDs present in m_weeklyquests.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7603u32);
    quest.flags = QUEST_FLAGS_WEEKLY_LIKE_CPP;
    install_quest(&mut session, &quest);
    session
        .quest_test_fixture_like_cpp
        .weekly_quests_completed_like_cpp
        .insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_monthly_already_completed_like_cpp() {
    // C++ SatisfyQuestMonth rejects IDs present in m_monthlyquests.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7605u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    install_quest(&mut session, &quest);
    session
        .quest_test_fixture_like_cpp
        .monthly_quests_completed_like_cpp
        .insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_when_quest_available_condition_not_met_like_cpp() {
    // C++ SatisfyQuestConditions evaluates ungrouped QuestAvailable conditions.
    let (mut session, _send_rx) = make_session();
    let quest_id = 7570u32;
    let quest = quest_template(quest_id);
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id as i32,
            condition_type: ConditionType::Level,
            condition_value1: 90,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));

    // Sin la condición la quest sería aceptable (nivel 1, min_level 1, raza/clase sin filtro).
    // Con la condición de nivel 90, el jugador (80) no la cumple.
    assert!(!session.can_take_quest(&quest));

    // POSITIVA: nivel requerido 80 (alcanzable para el jugador en nivel 80).
    let quest_id2 = 7571u32;
    let quest2 = quest_template(quest_id2);
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session.set_quest_store(Arc::new(store2));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id2 as i32,
            condition_type: ConditionType::Level,
            condition_value1: 80,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));

    assert!(session.can_take_quest(&quest2));
}

#[test]
fn can_take_quest_blocks_when_session_expansion_below_required_like_cpp() {
    // C++ SatisfyQuestExpansion compares session expansion with quest expansion.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(9900u32);
    quest.expansion = 2;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.expansion = 1;
    assert!(!session.can_take_quest(&quest));

    // POSITIVA límite: expansión de sesión == expansión requerida → acepta.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(9901u32);
    quest2.expansion = 2;
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));
    session2.expansion = 2;
    assert!(session2.can_take_quest(&quest2));
}

#[test]
fn can_take_quest_allows_weekly_not_yet_completed_like_cpp() {
    // C++ SatisfyQuestWeek allows a weekly absent from m_weeklyquests.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7604u32);
    quest.flags = QUEST_FLAGS_WEEKLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_allows_monthly_not_yet_completed_like_cpp() {
    // C++ SatisfyQuestMonth allows a monthly absent from m_monthlyquests.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7606u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_exclusive_group_blocks_when_peer_rewarded_non_repeatable_like_cpp() {
    // C++ SatisfyQuestExclusiveGroup blocks a rewarded non-repeatable peer.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9910u32);
    peer.exclusive_group = 5;
    // Both templates have special_flags=0, so C++ Quest::IsRepeatable is false (QuestDef.h:643).

    let mut quest = quest_template(9911u32);
    quest.exclusive_group = 5;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // El peer ya fue recompensado (no repetible).
    seed_rewarded_quest(&mut session, peer.id);

    // La quest objetivo no está ni activa ni rewarded.
    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_exclusive_group_blocks_when_peer_active_like_cpp() {
    // C++ SatisfyQuestExclusiveGroup blocks a peer with non-NONE quest status.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9912u32);
    peer.exclusive_group = 7;

    let mut quest = quest_template(9913u32);
    quest.exclusive_group = 7;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // El peer está activo (Incomplete).
    seed_active_quest(&mut session, peer.id);

    // La quest objetivo aún no ha sido aceptada.
    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_exclusive_group_positive_no_conflicting_peer_allows_like_cpp() {
    // POSITIVA: exclusive_group > 0 pero ningún peer está activo ni rewarded → true.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9914u32);
    peer.exclusive_group = 9;

    let mut quest = quest_template(9915u32);
    quest.exclusive_group = 9;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // Ningún peer activo ni rewarded → debe permitir.
    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_exclusive_group_zero_never_blocks_like_cpp() {
    // C++ SatisfyQuestExclusiveGroup returns true for every non-positive group.
    let (mut session, _send_rx) = make_session();

    let mut quest = quest_template(9916u32);
    quest.exclusive_group = 0;

    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));

    // También verifica group negativo.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(9917u32);
    quest2.exclusive_group = -3;

    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));

    assert!(session2.can_take_quest(&quest2));
}

#[test]
fn can_take_quest_dependent_previous_not_rewarded_blocks_like_cpp() {
    // C++ SatisfyQuestDependentPreviousQuests requires one rewarded predecessor;
    // non-negative predecessor groups need only that predecessor rewarded.
    let (mut session, _send_rx) = make_session();

    let quest_id = 9920u32;
    let prev_id = 9921u32;

    // Construir quest previa con next_quest_id → quest objetivo, exclusive_group=0 (>= 0).
    let mut prev_quest = quest_template(prev_id);
    prev_quest.next_quest_id = quest_id;
    prev_quest.exclusive_group = 0;

    // from_quests_like_cpp normaliza dependent_previous_quests automáticamente.
    let store = Arc::new(QuestStore::from_quests_like_cpp([
        quest_template(quest_id),
        prev_quest,
    ]));
    // Obtener la quest del store ya normalizado (con dependent_previous_quests populado).
    let quest = store.get(quest_id).expect("quest in store").clone();
    session.set_quest_store(Arc::clone(&store));

    // prev NO en rewarded_quests → el gate debe bloquear.
    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_dependent_previous_rewarded_allows_like_cpp() {
    // POSITIVA: mismo setup pero prev en rewarded_quests (exclusive_group=0 >= 0)
    // → C++ SatisfyQuestDependentPreviousQuests (15115-15171) permite continuar.
    let (mut session, _send_rx) = make_session();

    let quest_id = 9922u32;
    let prev_id = 9923u32;

    let mut prev_quest = quest_template(prev_id);
    prev_quest.next_quest_id = quest_id;
    prev_quest.exclusive_group = 0;

    let store = Arc::new(QuestStore::from_quests_like_cpp([
        quest_template(quest_id),
        prev_quest,
    ]));
    let quest = store.get(quest_id).expect("quest in store").clone();
    session.set_quest_store(Arc::clone(&store));

    // prev en rewarded_quests → el gate no bloquea.
    seed_rewarded_quest(&mut session, prev_id);
    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_reputation_blocks_when_below_min_rep_like_cpp() {
    // C++ SatisfyQuestReputation rejects standing below the minimum (15256-15283).
    let (mut session, _send_rx) = make_session();

    // Facción 76 con reputation_index 5.
    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Jugador standing 100 — por debajo del mínimo requerido (500).
    set_reputation_standing(&mut session, rep_list_id, 100);

    let mut quest = quest_template(9920u32);
    quest.required_min_rep_faction = faction_id;
    quest.required_min_rep_value = 500;
    // Aislar el gate: sin restricciones de raza/clase/level/conditions/expansion/exclusive_group.
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_reputation_blocks_when_at_or_above_max_rep_like_cpp() {
    // C++ SatisfyQuestReputation rejects standing >= the maximum (15256-15283).
    let (mut session, _send_rx) = make_session();

    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Jugador standing 1000 — igual al máximo requerido (1000) → bloqueado.
    set_reputation_standing(&mut session, rep_list_id, 1000);

    let mut quest = quest_template(9921u32);
    quest.required_max_rep_faction = faction_id;
    quest.required_max_rep_value = 1000;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_reputation_allows_when_in_valid_range_like_cpp() {
    // C++ SatisfyQuestReputation accepts standing >= min and < max.
    let (mut session, _send_rx) = make_session();

    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Standing 500 — exactamente en el mínimo (500) y por debajo del máximo (1000).
    set_reputation_standing(&mut session, rep_list_id, 500);

    let mut quest = quest_template(9922u32);
    quest.required_min_rep_faction = faction_id;
    quest.required_min_rep_value = 500;
    quest.required_max_rep_faction = faction_id;
    quest.required_max_rep_value = 1000;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_skill_blocks_when_skill_below_required_like_cpp() {
    // C++ SatisfyQuestSkill compares GetSkillValue with the required value (15009-15037).
    let skill_id: u16 = 1_000;
    let required_points: u32 = 300;

    let (mut session, _send_rx) = make_session();
    // Jugador con skill 1000 en valor 299 — por debajo del requisito.
    set_skill_values(
        &mut session,
        std::collections::HashMap::from([(skill_id, 299_u16)]),
    );

    let mut quest = quest_template(9940u32);
    quest.required_skill_id = u32::from(skill_id);
    quest.required_skill_points = required_points;
    // Aislar el gate: sin restricciones de raza/clase/level/rep/conditions/expansion.
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_skill_allows_when_skill_at_or_above_required_like_cpp() {
    // POSITIVA: skill del jugador >= required_skill_points → el gate no bloquea.
    let skill_id: u16 = 1_000;
    let required_points: u32 = 300;

    let (mut session, _send_rx) = make_session();
    // Jugador con skill 1000 exactamente en 300.
    set_skill_values(
        &mut session,
        std::collections::HashMap::from([(skill_id, 300_u16)]),
    );

    let mut quest = quest_template(9941u32);
    quest.required_skill_id = u32::from(skill_id);
    quest.required_skill_points = required_points;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_when_dependent_breadcrumb_in_log_like_cpp() {
    let breadcrumb_quest_id = 9930u32;
    let quest_id = 9931u32;

    // C++ SatisfyQuestDependentBreadcrumbQuests blocks an active, complete, or failed
    // breadcrumb quest in the player's quest log (15197-15216).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(quest_id);
    quest.dependent_breadcrumb_quests = vec![breadcrumb_quest_id];
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    seed_active_quest(&mut session, breadcrumb_quest_id);
    assert!(!session.can_take_quest(&quest));

    // POSITIVA: breadcrumb no está en el log → no bloquea.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(quest_id);
    quest2.dependent_breadcrumb_quests = vec![breadcrumb_quest_id];
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));
    // breadcrumb_quest_id no insertado en player_quests
    assert!(session2.can_take_quest(&quest2));
}
