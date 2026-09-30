use super::*;
use std::sync::Arc;

use wow_world::session::WorldSession;
use wow_world::test_fixtures::{
    player_gossip_options_for_test, set_player_faction_template_for_test,
    set_player_position_for_test, set_loaded_player_identity_like_cpp,
};
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_packet::WorldPacket;
use wow_persistence::{
    GossipBroadcastTextLocaleRequestLikeCpp, GossipCatalogPersistencePortLikeCpp,
    GossipCatalogReadOutcomeLikeCpp, GossipCreatureMenuRequestLikeCpp,
    GossipMenuCatalogRequestLikeCpp, GossipMenuOptionCatalogRowLikeCpp,
    GossipNpcTextCatalogRequestLikeCpp, PersistenceFutureLikeCpp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum GossipCatalogRequestTraceLikeCpp {
    CreatureMenu(GossipCreatureMenuRequestLikeCpp),
    MenuTexts(GossipMenuCatalogRequestLikeCpp),
    NpcText(GossipNpcTextCatalogRequestLikeCpp),
    MenuOptions(GossipMenuCatalogRequestLikeCpp),
    BroadcastLocale(GossipBroadcastTextLocaleRequestLikeCpp),
}

struct GossipCatalogPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<GossipCatalogRequestTraceLikeCpp>>,
    creature_menu:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<u32>>>,
    menu_texts:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<Vec<u32>>>>,
    npc_text:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<i32>>>,
    menu_options: std::sync::Mutex<
        std::collections::VecDeque<
            GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
        >,
    >,
    broadcast_locale:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<String>>>,
}

impl GossipCatalogPortFixtureLikeCpp {
    fn new(
        creature_menu: impl IntoIterator<Item = GossipCatalogReadOutcomeLikeCpp<u32>>,
        menu_texts: impl IntoIterator<Item = GossipCatalogReadOutcomeLikeCpp<Vec<u32>>>,
        npc_text: impl IntoIterator<Item = GossipCatalogReadOutcomeLikeCpp<i32>>,
        menu_options: impl IntoIterator<
            Item = GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
        >,
        broadcast_locale: impl IntoIterator<Item = GossipCatalogReadOutcomeLikeCpp<String>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            creature_menu: std::sync::Mutex::new(creature_menu.into_iter().collect()),
            menu_texts: std::sync::Mutex::new(menu_texts.into_iter().collect()),
            npc_text: std::sync::Mutex::new(npc_text.into_iter().collect()),
            menu_options: std::sync::Mutex::new(menu_options.into_iter().collect()),
            broadcast_locale: std::sync::Mutex::new(broadcast_locale.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<GossipCatalogRequestTraceLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl GossipCatalogPersistencePortLikeCpp for GossipCatalogPortFixtureLikeCpp {
    fn load_creature_gossip_menu_id_like_cpp<'a>(
        &'a self,
        request: GossipCreatureMenuRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<u32>> {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::CreatureMenu(request));
        let outcome = self
            .creature_menu
            .lock()
            .unwrap()
            .pop_front()
            .expect("one creature-menu outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_gossip_menu_text_ids_like_cpp<'a>(
        &'a self,
        request: GossipMenuCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<Vec<u32>>> {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::MenuTexts(request));
        let outcome = self
            .menu_texts
            .lock()
            .unwrap()
            .pop_front()
            .expect("one menu-text outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_npc_text_broadcast_id_like_cpp<'a>(
        &'a self,
        request: GossipNpcTextCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<i32>> {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::NpcText(request));
        let outcome = self
            .npc_text
            .lock()
            .unwrap()
            .pop_front()
            .expect("one npc-text outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_gossip_menu_options_like_cpp<'a>(
        &'a self,
        request: GossipMenuCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        'a,
        GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
    > {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::MenuOptions(request));
        let outcome = self
            .menu_options
            .lock()
            .unwrap()
            .pop_front()
            .expect("one menu-options outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_broadcast_text_locale_like_cpp<'a>(
        &'a self,
        request: GossipBroadcastTextLocaleRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<String>> {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::BroadcastLocale(request));
        let outcome = self
            .broadcast_locale
            .lock()
            .unwrap()
            .pop_front()
            .expect("one broadcast-locale outcome per request");
        Box::pin(async move { outcome })
    }
}

fn make_quest_status_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(8);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 80, 0);
    set_player_faction_template_for_test(&mut session, 1);
    set_player_position_for_test(&mut session, Position::new(10.0, 0.0, 0.0, 0.0));
    enable_gossip_fixture_for_test(&mut session);
    (session, send_rx)
}

fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, counter)
}

fn gossip_catalog_option_like_cpp(
    menu_id: u32,
    option_id: u32,
    broadcast_text_id: u32,
) -> GossipMenuOptionCatalogRowLikeCpp {
    GossipMenuOptionCatalogRowLikeCpp {
        menu_id,
        gossip_option_id: 77,
        option_id,
        option_npc: 1,
        option_text: "Original option".to_owned(),
        option_broadcast_text_id: broadcast_text_id,
        language: 0,
        flags: 3,
        action_menu_id: 88,
        action_poi_id: 0,
        gossip_npc_option_id: None,
        box_coded: false,
        box_money: 25,
        box_text: "Confirm".to_owned(),
        box_broadcast_text_id: 0,
        spell_id: Some(99),
        override_icon_id: Some(4),
    }
}

#[tokio::test]
async fn gossip_catalog_port_preserves_read_order_and_localized_projection_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let menu_id = 700;
    let npc_guid = creature_guid(9001, 701);
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(menu_id)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![10, 20])],
        [GossipCatalogReadOutcomeLikeCpp::Found(900)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![
            gossip_catalog_option_like_cpp(menu_id, 2, 901),
        ])],
        [GossipCatalogReadOutcomeLikeCpp::Found(
            "Opción localizada".to_owned(),
        )],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port.clone());

    let message = build_gossip_menu_for_test(&mut session, 9001, 0, npc_guid)
        .await
        .expect("typed catalog produces gossip message");

    assert_eq!(message.gossip_id, menu_id as i32);
    assert_eq!(message.broadcast_text_id, Some(900));
    assert_eq!(message.gossip_options.len(), 1);
    assert_eq!(message.gossip_options[0].text, "Opción localizada");
    assert_eq!(message.gossip_options[0].gossip_option_id, 77);
    assert_eq!(player_gossip_options_for_test(&session).len(), 1);
    assert_eq!(
        port.requests(),
        vec![
            GossipCatalogRequestTraceLikeCpp::CreatureMenu(GossipCreatureMenuRequestLikeCpp {
                creature_entry: 9001,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuTexts(GossipMenuCatalogRequestLikeCpp {
                menu_id,
            }),
            GossipCatalogRequestTraceLikeCpp::NpcText(GossipNpcTextCatalogRequestLikeCpp {
                npc_text_id: 20,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuOptions(GossipMenuCatalogRequestLikeCpp {
                menu_id,
            }),
            GossipCatalogRequestTraceLikeCpp::BroadcastLocale(
                GossipBroadcastTextLocaleRequestLikeCpp {
                    broadcast_text_id: 901,
                    locale: "esES".to_owned(),
                },
            ),
        ]
    );
}

#[tokio::test]
async fn gossip_catalog_required_read_failure_stops_before_locale_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(701)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![21])],
        [GossipCatalogReadOutcomeLikeCpp::Missing],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "options unavailable".to_owned(),
        }],
        [],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port.clone());

    assert!(
        build_gossip_menu_for_test(&mut session, 9002, 0, creature_guid(9002, 702))
            .await
            .is_none()
    );
    assert_eq!(
        port.requests(),
        vec![
            GossipCatalogRequestTraceLikeCpp::CreatureMenu(GossipCreatureMenuRequestLikeCpp {
                creature_entry: 9002,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuTexts(GossipMenuCatalogRequestLikeCpp {
                menu_id: 701,
            }),
            GossipCatalogRequestTraceLikeCpp::NpcText(GossipNpcTextCatalogRequestLikeCpp {
                npc_text_id: 21,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuOptions(GossipMenuCatalogRequestLikeCpp {
                menu_id: 701,
            }),
        ]
    );
}

#[tokio::test]
async fn gossip_catalog_optional_reads_fail_to_existing_fallbacks_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let menu_id = 702;
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(menu_id)],
        [GossipCatalogReadOutcomeLikeCpp::Missing],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "npc text unavailable".to_owned(),
        }],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![
            gossip_catalog_option_like_cpp(menu_id, 3, 902),
        ])],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "locale unavailable".to_owned(),
        }],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port);

    let message = build_gossip_menu_for_test(&mut session, 9003, 0, creature_guid(9003, 703))
        .await
        .expect("optional catalog failures retain the menu");
    assert_eq!(message.broadcast_text_id, None);
    assert_eq!(message.gossip_options[0].text, "Original option");
}

#[tokio::test]
async fn gossip_catalog_stale_owner_refuses_menu_after_complete_catalog_read_order() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    super::fixtures_world::insert_bank_test_player_in_world(&session, &canonical);
    let source = creature_guid(9004, 704);
    push_player_gossip_option_for_test(&mut session, wow_world::session::GossipOptionInfo {
        gossip_option_id: 41,
        menu_id: 42,
        order_index: 43,
        option_npc: 6,
        action_menu_id: 0,
    });
    let previous_options = player_gossip_options_for_test(&session);
    assert!(adopt_gossip_canonical_player_for_test(&mut session));
    let guid = session.player_guid().expect("fixture GUID");
    assert!(canonical.lock().unwrap().find_map_mut(571, 0).unwrap()
        .map_mut().remove_map_object(guid).is_some());
    let menu_id = 703;
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(menu_id)],
        [GossipCatalogReadOutcomeLikeCpp::Missing],
        [GossipCatalogReadOutcomeLikeCpp::Missing],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![
            gossip_catalog_option_like_cpp(menu_id, 4, 0),
        ])],
        [],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port.clone());

    let message = build_gossip_menu_for_test(&mut session, 9004, 0, source).await;

    assert!(message.is_none());
    assert_eq!(player_gossip_options_for_test(&session), previous_options);
    assert!(send_rx.try_recv().is_err());
    assert_eq!(port.requests(), vec![
        GossipCatalogRequestTraceLikeCpp::CreatureMenu(GossipCreatureMenuRequestLikeCpp {
            creature_entry: 9004,
        }),
        GossipCatalogRequestTraceLikeCpp::MenuTexts(GossipMenuCatalogRequestLikeCpp {
            menu_id,
        }),
        GossipCatalogRequestTraceLikeCpp::NpcText(GossipNpcTextCatalogRequestLikeCpp {
            npc_text_id: 1,
        }),
        GossipCatalogRequestTraceLikeCpp::MenuOptions(GossipMenuCatalogRequestLikeCpp {
            menu_id,
        }),
    ]);
}
