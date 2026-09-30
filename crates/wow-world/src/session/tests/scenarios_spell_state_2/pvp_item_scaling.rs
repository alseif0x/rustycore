use super::*;

#[test]
fn represented_item_level_area_scaling_activates_on_pvp_rules_aura_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_156_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 30);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(30_156, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(30_156, Position::ZERO);
    session
        .visible_auras
        .insert(1, test_visible_aura(1, SPELL_PVP_RULES_ENABLED_LIKE_CPP));

    assert!(session.update_represented_item_level_area_based_scaling_like_cpp());
    assert!(session.represented_using_pvp_item_levels_like_cpp());
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(130)
    );
}
#[test]
fn represented_pvp_rules_aura_application_recalculates_item_level_scaling_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_158_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 35);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(30_158, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(30_158, Position::ZERO);

    assert!(!session.represented_using_pvp_item_levels_like_cpp());
    session
        .apply_aura(
            SPELL_PVP_RULES_ENABLED_LIKE_CPP,
            ObjectGuid::EMPTY,
            30_000,
            0x0000_0001,
        )
        .expect("represented PvP rules aura should apply");

    assert!(
        session.represented_using_pvp_item_levels_like_cpp(),
        "C++ Player::EnablePvpRules calls UpdateItemLevelAreaBasedScaling after applying SPELL_PVP_RULES_ENABLED"
    );
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(135)
    );
}
#[test]
fn represented_pvp_rules_aura_removal_recalculates_item_level_scaling_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_159_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 40);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(30_159, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(30_159, Position::ZERO);
    session
        .apply_aura(
            SPELL_PVP_RULES_ENABLED_LIKE_CPP,
            ObjectGuid::EMPTY,
            30_000,
            0x0000_0001,
        )
        .expect("represented PvP rules aura should apply");
    let slot = session
        .visible_auras
        .values()
        .find_map(|aura| (aura.spell_id == SPELL_PVP_RULES_ENABLED_LIKE_CPP).then_some(aura.slot))
        .expect("PvP rules aura slot");

    session
        .remove_aura(slot)
        .expect("represented PvP rules aura should remove");

    assert!(
        !session.represented_using_pvp_item_levels_like_cpp(),
        "C++ Player::DisablePvpRules removes SPELL_PVP_RULES_ENABLED and then calls UpdateItemLevelAreaBasedScaling"
    );
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(100)
    );
}
