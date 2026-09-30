//! Session scenarios exercising the represented movement responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn battle_pet_remove_pet_requires_lock_and_marks_removed_like_cpp() {
    let (mut session, _, _) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x125);
    let new_pet_guid = ObjectGuid::new(0, 0x126);
    let unknown_guid = ObjectGuid::new(0, 0x127);

    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0x10,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    session.add_represented_battle_pet_like_cpp(
        new_pet_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::New,
    );
    assert!(session.battle_pet_set_battle_slot_like_cpp(pet_guid, 1));

    assert!(!session.battle_pet_remove_pet_like_cpp(pet_guid));
    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0x10,
            RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        ))
    );

    session.send_battle_pet_journal_lock_status_like_cpp().await;
    assert!(!session.battle_pet_remove_pet_like_cpp(unknown_guid));
    assert!(session.battle_pet_remove_pet_like_cpp(pet_guid));
    assert!(session.battle_pet_remove_pet_like_cpp(new_pet_guid));

    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0x10,
            RepresentedBattlePetSaveInfoLikeCpp::Removed,
        ))
    );
    assert_eq!(
        session.represented_battle_pet_like_cpp(new_pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0,
            RepresentedBattlePetSaveInfoLikeCpp::Removed,
        ))
    );

    let journal = session
        .represented_battle_pet_journal_like_cpp()
        .expect("represented battle-pet journal");
    assert!(journal.pets.is_empty());
    assert_eq!(journal.slots[1].pet_guid, pet_guid);
}
#[test]
fn represented_faction_reaction_static_branch_uses_template_fallback_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let mut source = faction_template_entry(1, 72, 0, 0, 930);
    let target = faction_template_entry(2, 930, 0, 0, 0);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            source.clone(),
            target.clone(),
        ]),
    ));

    let input = RepresentedFactionReactionInputLikeCpp {
        source_faction_template_id: 1,
        target_faction_template_id: 2,
        target_has_player_owner: false,
        target_player_owner_is_current_session: false,
        target_player_contested_pvp: false,
        target_is_unit: true,
        target_ignores_reputation: false,
    };
    assert_eq!(
        session.represented_faction_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Hostile
    );

    source.enemies = [0; 8];
    source.flags = wow_data::progression_rewards::FACTION_TEMPLATE_FLAG_HOSTILE_BY_DEFAULT_LIKE_CPP;
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([source, target]),
    ));
    assert_eq!(
        session.represented_faction_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Hostile
    );
}
