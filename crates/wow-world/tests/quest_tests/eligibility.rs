//! Application catalog/skill/reputation/condition gates and eligibility wiring.
//! Pure Player-state cases live in wow-entities/player/quest_state/eligibility/tests.rs.
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
use std::collections::HashSet;
use std::sync::Arc;
use wow_constants::quest::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
use wow_constants::{ComparisonType, ConditionSourceType, ConditionType};
use wow_data::progression_rewards::{FactionEntry, FactionStore};
use wow_data::quest::{
    QUEST_FLAGS_DAILY_LIKE_CPP, QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP, QuestStore, QuestTemplate,
};
use wow_data::{Condition, ConditionEntriesByTypeStore};
use wow_world::handlers::quest::PlayerQuestStatus;
use wow_world::test_fixtures::quest_dependent_previous_blocks_for_test as represented_satisfy_quest_dependent_previous_quests_failed_like_cpp;

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
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 80, 0);
    (session, send_rx)
}

fn quest_template(id: u32) -> QuestTemplate {
    wow_world::test_fixtures::quest_template_row_for_test(id, 2, format!("Quest {id}"))
}

fn install_quest(session: &mut WorldSession, quest: &QuestTemplate) {
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest.clone()])));
}

fn seed_rewarded_quest(session: &mut WorldSession, quest_id: u32) {
    set_player_quest_gameplay_rewarded_for_test(session, quest_id);
}

fn seed_active_quest(session: &mut WorldSession, quest_id: u32) {
    insert_player_quest_gameplay_status_for_test(
        session,
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
    set_reputation_standing_for_test(session, reputation_list_id, standing);
}

fn set_skill_values(session: &mut WorldSession, skill_values: std::collections::HashMap<u16, u16>) {
    let _ = set_player_skill_values_for_test(session, skill_values);
}

#[test]
fn can_take_quest_adapter_preserves_df_priority_and_independent_exclusive_peer_checks() {
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(9950);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    install_quest(&mut session, &quest);
    mutate_player_quest_gameplay_for_test(&mut session, |state| {
        state.set_daily_like_cpp(quest.id, true);
    })
    .expect("test Player quest owner");

    assert!(session.can_take_quest(&quest));
    mutate_player_quest_gameplay_for_test(&mut session, |state| {
        state.set_df_quest_like_cpp(quest.id, true);
    })
    .expect("test Player quest owner");
    assert!(!session.can_take_quest(&quest));

    let (mut peer_session, _peer_send_rx) = make_session();
    let mut candidate = quest_template(9951);
    candidate.exclusive_group = 11;
    let mut peer = quest_template(9952);
    peer.exclusive_group = 11;
    peer.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    peer.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    peer_session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        candidate.clone(),
        peer.clone(),
    ])));

    assert!(peer_session.can_take_quest(&candidate));
    mutate_player_quest_gameplay_for_test(&mut peer_session, |state| {
        state.set_daily_like_cpp(peer.id, true);
    })
    .expect("test Player quest owner");
    // Retained Rust gap: C++ SatisfyQuestExclusiveGroup calls SatisfyQuestDay.
    assert!(!peer_session.can_take_quest(&candidate));
}

#[test]
fn can_take_quest_adapter_delegates_dependent_breadcrumb_membership() {
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(9953);
    quest.dependent_breadcrumb_quests = vec![9954];
    install_quest(&mut session, &quest);

    assert!(session.can_take_quest(&quest));
    seed_active_quest(&mut session, 9954);
    assert!(!session.can_take_quest(&quest));
}

#[test]
fn dependent_previous_adapter_passes_lazy_catalog_and_negative_group_facts_to_domain() {
    let mut first = quest_template(9955);
    let mut peer = quest_template(9956);
    let missing_id = 9958;
    first.exclusive_group = -12;
    peer.exclusive_group = -12;
    let store = QuestStore::from_quests_like_cpp([first, peer]);
    let mut quest = quest_template(9957);
    quest.dependent_previous_quests = vec![9955, missing_id];
    let mut rewarded = HashSet::from([9955]);

    // The real adapter must forward the negative group rather than only the
    // rewarded predecessor; the unvisited missing row remains in the input.
    let blocked = represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
        &store, &quest, &rewarded,
    );
    assert!(
        blocked,
        "the catalog's unrewarded negative-group peer blocks"
    );
    rewarded.insert(9956);
    let blocked = represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
        &store, &quest, &rewarded,
    );
    assert!(
        !blocked,
        "a complete first group allows before a missing later row"
    );
    quest.dependent_previous_quests.swap(0, 1);
    assert!(
        represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
            &store, &quest, &rewarded,
        ),
        "a missing first catalog row rejects even when a later group is complete"
    );
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
