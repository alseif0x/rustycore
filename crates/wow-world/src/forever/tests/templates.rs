//! Session-linked default permissions and AuthResponse template publication.
use super::*;
use wow_data::forever_initialization::{ClassRecord, InitializationRecords};
use wow_persistence::forever::{
    creation::templates::{TemplateClassRow, TemplateRow, TemplateRows},
    permissions::DefaultPermissionRows,
};

fn template_permissions(granted: bool) -> Arc<permissions::DefaultAccountPermissions> {
    Arc::new(permissions::DefaultAccountPermissions::load(
        DefaultPermissionRows {
            known: vec![10, 17, 195],
            links: vec![(195, 10)],
            roots: if granted { vec![195] } else { vec![17] },
        },
    ))
}
fn templates(name: &str) -> Arc<creation::CharacterTemplates> {
    let initialization = InitializationRecords {
        classes: vec![ClassRecord {
            id: 1,
            flags: 0,
            starting_level: 1,
            cinematic: 0,
            default_spec: 0,
            strength_bonus: 0,
            primary_stat_priority: 0,
            display_power: 0,
            ranged_attack_per_agility: 0,
            attack_per_agility: 0,
            attack_per_strength: 0,
            spell_class_set: 0,
        }],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &Default::default())
    .unwrap();
    Arc::new(
        creation::CharacterTemplates::load(
            TemplateRows {
                classes: vec![
                    TemplateClassRow {
                        template_id: 70170,
                        class: 1,
                        faction_group: 3,
                    },
                    TemplateClassRow {
                        template_id: 70170,
                        class: 1,
                        faction_group: 5,
                    },
                ],
                templates: vec![TemplateRow {
                    id: 70170,
                    name: name.into(),
                    description: "Synthetic".into(),
                    level: 77,
                }],
            },
            &initialization,
        )
        .unwrap(),
    )
}

#[tokio::test]
async fn auth_templates_follow_permission_and_source_class_string_shape_without_a_level_field() {
    for granted in [false, true] {
        let repository = Arc::new(Repository::good());
        let mut session = session(repository.clone());
        session.identity.permissions = template_permissions(granted);
        let mut policy = policy();
        policy.character_templates = templates("Fixture");
        let output = session
            .initialize(&CharacterCatalog::fixture(), &policy, 17)
            .await
            .unwrap();
        assert_eq!(repository.loads.load(Ordering::SeqCst), 1);
        let mut reader = wow_packet::WorldPacket::from_bytes(output[0].payload());
        assert_eq!(reader.read_uint32().unwrap(), 0); // AuthResponse result
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        reader.reset_bits();
        assert_eq!(reader.read_uint32().unwrap(), 0x02010001);
        assert_eq!(reader.read_uint32().unwrap(), 1); // realm count
        assert_eq!(reader.read_uint32().unwrap(), 0); // rested
        assert_eq!(reader.read_uint8().unwrap(), 0);
        assert_eq!(reader.read_uint8().unwrap(), 0);
        assert_eq!(reader.read_uint32().unwrap(), 0); // kick
        assert_eq!(reader.read_uint32().unwrap(), 1); // race count
        assert_eq!(reader.read_uint32().unwrap(), u32::from(granted)); // templates
        assert_eq!(reader.read_uint32().unwrap(), 0); // currency
        for _ in 0..3 {
            assert_eq!(reader.read_uint32().unwrap(), 0);
        }
        assert_eq!(reader.read_bits(3).unwrap(), 0);
        reader.reset_bits();
        assert_eq!(reader.read_int64().unwrap(), 17);
        assert_eq!(reader.read_uint32().unwrap(), 0x02010001);
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        let actual_len = reader.read_bits(8).unwrap() as usize;
        let normalized_len = reader.read_bits(8).unwrap() as usize;
        reader.reset_bits();
        assert_eq!(reader.read_string(actual_len).unwrap(), "Forever");
        assert_eq!(reader.read_string(normalized_len).unwrap(), "Forever");
        assert_eq!(reader.read_uint8().unwrap(), 1);
        assert_eq!(reader.read_uint32().unwrap(), 1);
        assert_eq!(reader.read_uint8().unwrap(), 1);
        for _ in 0..3 {
            assert_eq!(reader.read_uint8().unwrap(), 0);
        }
        if granted {
            assert_eq!(reader.read_uint32().unwrap(), 70170);
            assert_eq!(reader.read_uint32().unwrap(), 2);
            for faction in [3, 5] {
                assert_eq!(reader.read_uint8().unwrap(), 1);
                assert_eq!(reader.read_uint8().unwrap(), faction);
            }
            let name_len = reader.read_bits(7).unwrap() as usize;
            let description_len = reader.read_bits(10).unwrap() as usize;
            reader.reset_bits();
            assert_eq!(reader.read_string(name_len).unwrap(), "Fixture");
            assert_eq!(reader.read_string(description_len).unwrap(), "Synthetic");
        }
        // Source does not serialize template Level. Next byte is six option
        // flags, all unset, not the template's stored level 77.
        assert_eq!(reader.read_bits(6).unwrap(), 0);
        reader.reset_bits();
        assert_eq!(reader.remaining(), 0);
        assert!(!session.is_closed());
    }
}

#[tokio::test]
async fn permitted_template_codec_failure_publishes_nothing_and_closes_before_snapshot_admission() {
    for granted in [false, true] {
        let repository = Arc::new(Repository::good());
        let mut session = session(repository.clone());
        session.identity.permissions = template_permissions(granted);
        let mut policy = policy();
        policy.character_templates = templates(&"x".repeat(128));
        let result = session
            .initialize(&CharacterCatalog::fixture(), &policy, 17)
            .await;
        assert_eq!(repository.loads.load(Ordering::SeqCst), 1);
        if granted {
            assert!(matches!(result, Err(SessionError::Codec)));
            assert!(session.is_closed() && session.snapshot.is_none());
            assert!(matches!(
                session
                    .initialize(&CharacterCatalog::fixture(), &policy, 17)
                    .await,
                Err(SessionError::Phase)
            ));
            assert_eq!(repository.loads.load(Ordering::SeqCst), 1);
        } else {
            assert!(result.is_ok() && !session.is_closed());
        }
    }
}
