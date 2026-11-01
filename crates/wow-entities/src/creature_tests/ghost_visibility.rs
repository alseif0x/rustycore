use super::*;

#[test]
fn creature_creation_applies_ghost_visibility_and_healer_override() {
    let cases = [
        ("ordinary", 0, 0, 0, 0b01, 0b01),
        (
            "visible-to-ghosts",
            wow_constants::CreatureTypeFlags::VISIBLE_TO_GHOSTS.bits(),
            0,
            0,
            0b11,
            0b01,
        ),
        (
            "spirit-healer",
            0,
            u64::from(wow_constants::NPCFlags1::SPIRIT_HEALER.bits()),
            0,
            0b10,
            0b10,
        ),
        (
            "area-spirit-healer",
            0,
            u64::from(wow_constants::NPCFlags1::AREA_SPIRIT_HEALER.bits()),
            0,
            0b10,
            0b10,
        ),
        (
            "extra-ghost-visibility",
            0,
            0,
            wow_constants::CreatureFlagsExtra::GHOST_VISIBILITY.bits(),
            0b10,
            0b10,
        ),
        (
            "extra-overrides-type-flag",
            wow_constants::CreatureTypeFlags::VISIBLE_TO_GHOSTS.bits(),
            0,
            wow_constants::CreatureFlagsExtra::GHOST_VISIBILITY.bits(),
            0b10,
            0b10,
        ),
        (
            "healer-overrides-type-flag",
            wow_constants::CreatureTypeFlags::VISIBLE_TO_GHOSTS.bits(),
            u64::from(wow_constants::NPCFlags1::SPIRIT_HEALER.bits()),
            0,
            0b10,
            0b10,
        ),
    ];
    let mut alive_seer = Unit::new(true);
    alive_seer.set_server_side_ghost_visibility_detect_like_cpp(0b01);
    let mut ghost_seer = Unit::new(true);
    ghost_seer.set_server_side_ghost_visibility_detect_like_cpp(0b10);
    let mut ghost_player = Unit::new(true);
    ghost_player.set_server_side_ghost_visibility_like_cpp(0b10);

    for (index, (name, type_flags, npc_flags, flags_extra, visibility, detect)) in
        cases.into_iter().enumerate()
    {
        let mut record = creature_lifecycle_create_record();
        record.guid = ObjectGuid::new(8, 10_100 + index as i64);
        record.template.type_flags = type_flags;
        record.template.npc_flags = npc_flags;
        record.template.flags_extra = flags_extra;
        let creature = Creature::create_from_lifecycle(record);

        assert_eq!(
            alive_seer.can_see_or_detect_unit_like_cpp(creature.unit(), false, true, false),
            visibility & 0b01 != 0,
            "alive seer visibility for {name}"
        );
        assert_eq!(
            ghost_seer.can_see_or_detect_unit_like_cpp(creature.unit(), false, true, false),
            visibility & 0b10 != 0,
            "ghost seer visibility for {name}"
        );
        assert_eq!(
            creature
                .unit()
                .can_see_or_detect_unit_like_cpp(&ghost_player, false, false, true),
            detect & 0b10 != 0,
            "creature detection mask for {name}"
        );
    }
}
