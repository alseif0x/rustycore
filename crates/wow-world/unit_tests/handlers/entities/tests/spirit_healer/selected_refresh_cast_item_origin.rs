use super::*;

fn apply_sickness_with_cast_metadata(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    cast_metadata: wow_entities::SpellCastMetadata,
) {
    let spell_info = session
        .catalogs
        .spell_store()
        .and_then(|store| store.get(SICKNESS_SPELL))
        .cloned()
        .expect("canonical sickness SpellInfo");
    session
        .player_aura_application_cx_like_cpp()
        .apply_resurrection_sickness_aura(
            SICKNESS_SPELL,
            player_guid,
            &spell_info,
            60_000,
            0,
            0,
            0,
            Ok((
                0x0011,
                wow_entities::AuraCastProvenanceLikeCpp::default(),
                0,
            )),
            cast_metadata,
        )
        .expect("apply selected sickness with cast-item facts");
}

#[tokio::test]
async fn sickness_cast_item_origin_refresh_matches_cpp_guid_gate() {
    let (mut session, _send_rx, player_guid) =
        sickness_refresh_session(46_051, false, false, 0, 60_000, 120_000).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let original_guid = ObjectGuid::create_item(1, 46_052);
    let replacement_guid = ObjectGuid::create_item(1, 46_053);

    apply_sickness_with_cast_metadata(
        &mut session,
        player_guid,
        wow_entities::SpellCastMetadata {
            cast_item_guid: Some(original_guid),
            cast_item_entry: Some(701),
            cast_item_level: Some(31),
            ..Default::default()
        },
    );
    let fresh = selected_aura_state(&session, SICKNESS_SPELL).1;
    assert_eq!(fresh.item_guid, original_guid);
    assert_eq!(fresh.cast_item_id, 701);
    assert_eq!(fresh.cast_item_level, 31);

    apply_sickness_with_cast_metadata(
        &mut session,
        player_guid,
        wow_entities::SpellCastMetadata {
            cast_item_guid: Some(original_guid),
            cast_item_entry: Some(702),
            cast_item_level: Some(42),
            ..Default::default()
        },
    );
    let same_guid = selected_aura_state(&session, SICKNESS_SPELL).1;
    assert_eq!(same_guid.item_guid, original_guid);
    assert_eq!(same_guid.cast_item_id, 701);
    assert_eq!(same_guid.cast_item_level, 31);

    apply_sickness_with_cast_metadata(
        &mut session,
        player_guid,
        wow_entities::SpellCastMetadata {
            cast_item_guid: Some(replacement_guid),
            cast_item_entry: Some(703),
            cast_item_level: Some(53),
            ..Default::default()
        },
    );
    let changed_guid = selected_aura_state(&session, SICKNESS_SPELL).1;
    assert_eq!(changed_guid.item_guid, replacement_guid);
    assert_eq!(changed_guid.cast_item_id, 703);
    assert_eq!(changed_guid.cast_item_level, 53);

    apply_sickness_with_cast_metadata(
        &mut session,
        player_guid,
        wow_entities::SpellCastMetadata {
            cast_item_level: Some(27),
            ..Default::default()
        },
    );
    let empty_guid_with_level = selected_aura_state(&session, SICKNESS_SPELL).1;
    assert_eq!(empty_guid_with_level.item_guid, ObjectGuid::EMPTY);
    assert_eq!(empty_guid_with_level.cast_item_id, 0);
    assert_eq!(empty_guid_with_level.cast_item_level, 27);

    apply_sickness_with_cast_metadata(
        &mut session,
        player_guid,
        wow_entities::SpellCastMetadata {
            cast_item_guid: Some(replacement_guid),
            cast_item_entry: Some(703),
            cast_item_level: Some(53),
            ..Default::default()
        },
    );
    apply_sickness_with_cast_metadata(
        &mut session,
        player_guid,
        wow_entities::SpellCastMetadata::default(),
    );
    let no_item = selected_aura_state(&session, SICKNESS_SPELL).1;
    assert_eq!(no_item.item_guid, ObjectGuid::EMPTY);
    assert_eq!(no_item.cast_item_id, 0);
    assert_eq!(no_item.cast_item_level, -1);
}

#[tokio::test]
async fn sickness_cast_item_origin_keeps_explicit_level_without_guid() {
    let (mut session, _send_rx, player_guid) =
        sickness_refresh_session(46_054, false, false, 0, 60_000, 120_000).await;
    let _packet_peer = keep_recovery_input_open(&mut session);

    apply_sickness_with_cast_metadata(
        &mut session,
        player_guid,
        wow_entities::SpellCastMetadata {
            cast_item_level: Some(27),
            ..Default::default()
        },
    );
    let metadata = selected_aura_state(&session, SICKNESS_SPELL).1;
    assert_eq!(metadata.item_guid, ObjectGuid::EMPTY);
    assert_eq!(metadata.cast_item_id, 0);
    assert_eq!(metadata.cast_item_level, 27);
}
