use wow_core::ObjectGuid;
use wow_data::{DifficultyStore, SpellLevelsStore};
use wow_map::{MapKey, MapManager, PlayerHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedCasterLevelError {
    OwnerUnavailable,
    OwnerMapUnavailable,
    CasterUnavailable,
    SpellLevelsUnavailable,
    DifficultyUnavailable,
    DifficultyCycle,
}

/// Resolve the caster level retained by a newly-created selected Aura.
///
/// C++ `SpellAuras.cpp:398-410,463-468` first resolves the raw caster in the
/// target Unit's map context, then captures either that Unit's current level or
/// the selected SpellInfo's known SpellLevel. The caller supplies the already
/// borrowed canonical map owner so this lookup needs no session or directory.
pub(crate) fn selected_caster_level(
    manager: &MapManager,
    owner: PlayerHandle,
    raw_caster_guid: ObjectGuid,
    spell_id: u32,
    difficulty_id: u8,
    spell_levels: Option<&SpellLevelsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<u8, SelectedCasterLevelError> {
    let Some((current_owner, _)) = manager.current_player_admission_like_cpp(owner.guid()) else {
        return Err(SelectedCasterLevelError::OwnerUnavailable);
    };
    if current_owner != owner {
        return Err(SelectedCasterLevelError::OwnerUnavailable);
    }

    let owner_snapshot = manager
        .with_player_like_cpp(owner, |player| {
            let world = player.unit().world();
            let assigned_map = world
                .has_current_map()
                .then(|| MapKey::new(world.map_id(), world.instance_id()));
            let owner_map_membership_matches = assigned_map.is_some_and(|key| {
                manager
                    .find_map(key.map_id, key.instance_id)
                    .and_then(|managed| managed.map().get_typed_player(owner.guid()))
                    .is_some_and(|member| std::ptr::eq(player, member))
            });
            (
                player.guid() == owner.guid(),
                player.unit().data().level,
                assigned_map,
                owner_map_membership_matches,
            )
        })
        .ok_or(SelectedCasterLevelError::OwnerUnavailable)?;
    if !owner_snapshot.0 {
        return Err(SelectedCasterLevelError::OwnerUnavailable);
    }

    if raw_caster_guid == owner.guid() {
        return Ok(owner_snapshot.1 as u8);
    }

    if !raw_caster_guid.is_unit() {
        return spell_level_fallback(spell_id, difficulty_id, spell_levels, difficulties);
    }

    let owner_map_key = owner_snapshot
        .2
        .filter(|_| owner_snapshot.3)
        .ok_or(SelectedCasterLevelError::OwnerMapUnavailable)?;
    let owner_map = manager
        .find_map(owner_map_key.map_id, owner_map_key.instance_id)
        .ok_or(SelectedCasterLevelError::OwnerMapUnavailable)?;

    if raw_caster_guid.is_player() {
        let Some((source_handle, _)) = manager.current_player_admission_like_cpp(raw_caster_guid)
        else {
            return spell_level_fallback(spell_id, difficulty_id, spell_levels, difficulties);
        };
        let source = manager
            .with_player_like_cpp(source_handle, |player| {
                let world = player.unit().world();
                let source_map_key = world
                    .has_current_map()
                    .then(|| MapKey::new(world.map_id(), world.instance_id()));
                let map_membership_matches = source_map_key.is_some_and(|key| {
                    manager
                        .find_map(key.map_id, key.instance_id)
                        .and_then(|source_map| {
                            source_map
                                .map()
                                .get_typed_player(raw_caster_guid)
                                .map(|member| (source_map, member))
                        })
                        .is_some_and(|(source_map, member)| {
                            std::ptr::eq(player, member)
                                && (key != owner_map_key
                                    || std::ptr::eq(owner_map.map(), source_map.map()))
                        })
                });
                (
                    player.guid() == raw_caster_guid,
                    source_map_key,
                    world.object().is_in_world(),
                    map_membership_matches,
                    player.unit().data().level as u8,
                )
            })
            .ok_or(SelectedCasterLevelError::CasterUnavailable)?;
        if !source.0 {
            return Err(SelectedCasterLevelError::CasterUnavailable);
        }
        if !source.2 {
            return spell_level_fallback(spell_id, difficulty_id, spell_levels, difficulties);
        }
        if let Some(source_map_key) = source.1 {
            if !source.3 {
                return Err(SelectedCasterLevelError::CasterUnavailable);
            }
            if source_map_key == owner_map_key {
                return Ok(source.4);
            }
        } else {
            return Err(SelectedCasterLevelError::CasterUnavailable);
        }
    } else if let Some(level) =
        owner_map
            .map()
            .with_creature_or_pet_like_cpp(raw_caster_guid, |creature, _owner_guid| {
                let world = creature.unit().world();
                (
                    creature.guid(),
                    world
                        .has_current_map()
                        .then(|| MapKey::new(world.map_id(), world.instance_id())),
                    creature.unit().data().level as u8,
                )
            })
    {
        if level.0 != raw_caster_guid || level.1 != Some(owner_map_key) {
            return Err(SelectedCasterLevelError::CasterUnavailable);
        }
        return Ok(level.2);
    }

    spell_level_fallback(spell_id, difficulty_id, spell_levels, difficulties)
}

fn spell_level_fallback(
    spell_id: u32,
    difficulty_id: u8,
    spell_levels: Option<&SpellLevelsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<u8, SelectedCasterLevelError> {
    let spell_levels = spell_levels.ok_or(SelectedCasterLevelError::SpellLevelsUnavailable)?;
    let mut current_difficulty = difficulty_id;
    let mut visited = [false; 256];

    loop {
        let index = usize::from(current_difficulty);
        if visited[index] {
            return Err(SelectedCasterLevelError::DifficultyCycle);
        }
        visited[index] = true;

        if let Some(entry) = spell_levels
            .entries_like_cpp()
            .filter(|entry| entry.spell_id == spell_id && entry.difficulty_id == current_difficulty)
            .max_by_key(|entry| entry.id)
        {
            return Ok(entry.spell_level as u8);
        }

        if current_difficulty == 0 {
            return Ok(0);
        }
        let difficulty_store =
            difficulties.ok_or(SelectedCasterLevelError::DifficultyUnavailable)?;
        let Some(next_difficulty) =
            difficulty_store.fallback_difficulty_id_like_cpp(current_difficulty)
        else {
            // A known Difficulty store with no row for this id cannot supply
            // C++'s fallback chain; SpellInfo's already-known level is zero.
            return Ok(0);
        };
        current_difficulty = next_difficulty;
    }
}
