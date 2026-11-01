use std::ptr;

use wow_constants::{
    creature::CreatureStaticFlags4,
    unit::{UnitFlags, UnitFlags2, UnitPvpFlags},
};
use wow_core::ObjectGuid;
use wow_data::progression_rewards::{
    FactionStore, FactionTemplateStore, FriendshipRepReactionStore,
};
use wow_data::reputation::ReputationRankLikeCpp;
use wow_entities::{Creature, Player, Unit};
use wow_map::{MapKey, MapManager, PlayerHandle, PlayerOwnerError, PlayerResidenceLikeCpp};
use wow_world_core::session::{
    FactionReactionCatalogs, FactionReactionError, FactionReactionOwner,
    RepresentedGetReactionInputLikeCpp, resolve_faction_reaction,
    resolve_faction_reaction_with_raid,
};

use super::selected_caster::SelectedCasterLevelError;

#[derive(Clone, Copy)]
pub(super) struct SelectedOwnerContext {
    pub(super) level: u8,
    pub(super) map_key: Option<MapKey>,
}

#[derive(Clone, Copy)]
pub(super) enum SelectedUnit<'a> {
    Player(&'a Player),
    Creature {
        body: &'a Creature,
        pet_owner_guid: Option<ObjectGuid>,
    },
}

impl<'a> SelectedUnit<'a> {
    pub(super) fn unit(self) -> &'a Unit {
        match self {
            Self::Player(player) => player.unit(),
            Self::Creature { body, .. } => body.unit(),
        }
    }

    fn player(self) -> Option<&'a Player> {
        match self {
            Self::Player(player) => Some(player),
            Self::Creature { .. } => None,
        }
    }

    fn control_guid(self) -> Result<Option<ObjectGuid>, SelectedCasterLevelError> {
        let control = &self.unit().subsystems().control;
        let canonical_pet_owner = match self {
            Self::Creature {
                pet_owner_guid: Some(pet_owner),
                ..
            } if !pet_owner.is_empty() => Some(pet_owner),
            _ => None,
        };
        if let Some(charmer) = control.charmer_guid.filter(|guid| !guid.is_empty()) {
            return Ok(Some(charmer));
        }
        if let (Some(owner), Some(pet_owner)) = (control.owner_guid, canonical_pet_owner)
            && !owner.is_empty()
            && owner != pet_owner
        {
            return Err(SelectedCasterLevelError::CasterUnavailable);
        }

        Ok(control
            .owner_guid
            .filter(|guid| !guid.is_empty())
            .or(canonical_pet_owner))
    }
}

pub(super) fn selected_owner_context(
    manager: &MapManager,
    owner: PlayerHandle,
) -> Result<SelectedOwnerContext, SelectedCasterLevelError> {
    let Some((current_owner, _)) = manager.current_player_admission_like_cpp(owner.guid()) else {
        return Err(SelectedCasterLevelError::OwnerUnavailable);
    };
    if current_owner != owner {
        return Err(SelectedCasterLevelError::OwnerUnavailable);
    }
    manager
        .with_player_like_cpp(owner, |player| {
            if player.guid() != owner.guid() {
                return Err(SelectedCasterLevelError::OwnerUnavailable);
            }
            let world = player.unit().world();
            let map_key = world
                .has_current_map()
                .then(|| MapKey::new(world.map_id(), world.instance_id()));
            Ok(SelectedOwnerContext {
                level: player.unit().data().level as u8,
                map_key,
            })
        })
        .ok_or(SelectedCasterLevelError::OwnerUnavailable)?
}

pub(super) fn selected_owner_map(
    manager: &MapManager,
    owner: PlayerHandle,
    context: SelectedOwnerContext,
) -> Result<MapKey, SelectedCasterLevelError> {
    let key = context
        .map_key
        .ok_or(SelectedCasterLevelError::OwnerMapUnavailable)?;
    let Some((current_owner, _)) = manager.current_player_admission_like_cpp(owner.guid()) else {
        return Err(SelectedCasterLevelError::OwnerUnavailable);
    };
    if current_owner != owner {
        return Err(SelectedCasterLevelError::OwnerUnavailable);
    }
    let residence = manager.checked_player_residence_like_cpp(owner);
    manager
        .with_player_like_cpp(owner, |player| {
            let world = player.unit().world();
            if player.guid() != owner.guid()
                || !world.has_current_map()
                || MapKey::new(world.map_id(), world.instance_id()) != key
            {
                return Err(SelectedCasterLevelError::OwnerMapUnavailable);
            }
            let managed = manager
                .find_map(key.map_id, key.instance_id)
                .ok_or(SelectedCasterLevelError::OwnerMapUnavailable)?;
            let member = managed.map().get_typed_player(owner.guid());
            match residence {
                Ok(PlayerResidenceLikeCpp::Active(residence_key)) if residence_key == key => {
                    let member = member.ok_or(SelectedCasterLevelError::OwnerMapUnavailable)?;
                    if !ptr::eq(player, member) {
                        return Err(SelectedCasterLevelError::OwnerUnavailable);
                    }
                }
                Err(PlayerOwnerError::ActivePlayerNotInWorld { guid })
                    if guid == owner.guid()
                        && !world.object().is_in_world()
                        && member.is_some_and(|member| ptr::eq(player, member)) => {}
                Err(PlayerOwnerError::DetachedPlayerStillBound {
                    guid,
                    key: bound_key,
                }) if guid == owner.guid() && bound_key == key && !world.object().is_in_world() => {
                    if member.is_some() {
                        return Err(SelectedCasterLevelError::OwnerUnavailable);
                    }
                }
                Ok(PlayerResidenceLikeCpp::Detached) => {
                    return Err(SelectedCasterLevelError::OwnerUnavailable);
                }
                _ => return Err(SelectedCasterLevelError::OwnerUnavailable),
            }
            Ok(key)
        })
        .ok_or(SelectedCasterLevelError::OwnerUnavailable)?
}

/// Resolve a Player, Creature or Pet through the selected target's physical map.
/// Player access follows ObjectAccessor's in-world and same-map rule; creature
/// access follows the typed map container and checks the body's map assignment.
pub(super) fn with_unit_in_map<R>(
    manager: &MapManager,
    map_key: MapKey,
    guid: ObjectGuid,
    read: impl FnOnce(SelectedUnit<'_>) -> R,
) -> Result<Option<R>, SelectedCasterLevelError> {
    let target_map = manager
        .find_map(map_key.map_id, map_key.instance_id)
        .ok_or(SelectedCasterLevelError::OwnerMapUnavailable)?;

    if guid.is_player() {
        let Some((handle, _)) = manager.current_player_admission_like_cpp(guid) else {
            if target_map.map().contains_map_object_like_cpp(guid) {
                return Err(SelectedCasterLevelError::CasterUnavailable);
            }
            return Ok(None);
        };
        return manager
            .with_player_like_cpp(handle, |player| {
                if player.guid() != guid {
                    return Err(SelectedCasterLevelError::CasterUnavailable);
                }
                let world = player.unit().world();
                if !world.object().is_in_world() {
                    return Ok(None);
                }
                let residence = manager
                    .player_residence_like_cpp(handle)
                    .ok_or(SelectedCasterLevelError::CasterUnavailable)?;
                let body_map = world
                    .has_current_map()
                    .then(|| MapKey::new(world.map_id(), world.instance_id()));
                let registered_map = match (residence, body_map) {
                    (PlayerResidenceLikeCpp::Detached, None) => None,
                    (PlayerResidenceLikeCpp::Active(key), Some(body_key)) if key == body_key => {
                        let member = manager
                            .find_map(key.map_id, key.instance_id)
                            .and_then(|managed| managed.map().get_typed_player(guid))
                            .ok_or(SelectedCasterLevelError::CasterUnavailable)?;
                        if !ptr::eq(player, member) {
                            return Err(SelectedCasterLevelError::CasterUnavailable);
                        }
                        Some(key)
                    }
                    _ => return Err(SelectedCasterLevelError::CasterUnavailable),
                };

                if registered_map != Some(map_key) {
                    return Ok(None);
                }
                let member = target_map
                    .map()
                    .get_typed_player(guid)
                    .ok_or(SelectedCasterLevelError::CasterUnavailable)?;
                if !ptr::eq(player, member) {
                    return Err(SelectedCasterLevelError::CasterUnavailable);
                }
                Ok(Some(read(SelectedUnit::Player(player))))
            })
            .ok_or(SelectedCasterLevelError::CasterUnavailable)?;
    }

    let Some(result) =
        target_map
            .map()
            .with_creature_or_pet_like_cpp(guid, |body, pet_owner_guid| {
                if body.guid() != guid {
                    return Err(SelectedCasterLevelError::CasterUnavailable);
                }
                let world = body.unit().world();
                let body_map = world
                    .has_current_map()
                    .then(|| MapKey::new(world.map_id(), world.instance_id()));
                if body_map != Some(map_key) {
                    return Err(SelectedCasterLevelError::CasterUnavailable);
                }
                Ok(read(SelectedUnit::Creature {
                    body,
                    pet_owner_guid,
                }))
            })
    else {
        return Ok(None);
    };
    result.map(Some)
}

pub(crate) fn selected_caster_is_friendly(
    manager: &MapManager,
    owner: PlayerHandle,
    raw_caster_guid: ObjectGuid,
    faction_store: Option<&FactionStore>,
    faction_templates: Option<&FactionTemplateStore>,
    friendship_reactions: Option<&FriendshipRepReactionStore>,
) -> Result<bool, SelectedCasterLevelError> {
    let owner_context = selected_owner_context(manager, owner)?;
    if raw_caster_guid == owner.guid() {
        return Ok(true);
    }
    if !raw_caster_guid.is_unit() {
        return Ok(false);
    }
    let map_key = selected_owner_map(manager, owner, owner_context)?;
    let Some(reaction) = with_unit_in_map(manager, map_key, raw_caster_guid, |source| {
        manager
            .with_player_like_cpp(owner, |target| {
                resolve_reaction(
                    manager,
                    map_key,
                    source,
                    SelectedUnit::Player(target),
                    owner.guid(),
                    faction_store,
                    faction_templates,
                    friendship_reactions,
                )
            })
            .ok_or(SelectedCasterLevelError::OwnerUnavailable)?
    })?
    else {
        return Ok(false);
    };
    Ok(reaction?)
}

fn resolve_reaction(
    manager: &MapManager,
    map_key: MapKey,
    source: SelectedUnit<'_>,
    target: SelectedUnit<'_>,
    target_session_guid: ObjectGuid,
    faction_store: Option<&FactionStore>,
    faction_templates: Option<&FactionTemplateStore>,
    friendship_reactions: Option<&FriendshipRepReactionStore>,
) -> Result<bool, SelectedCasterLevelError> {
    if ptr::eq(source.unit(), target.unit()) {
        return resolve_rank(
            RepresentedGetReactionInputLikeCpp {
                same_object: true,
                ..reaction_input(
                    source, target, false, false, false, false, false, false, false, None, None,
                )
            },
            None,
            None,
            FactionReactionCatalogs::new(None, None, None),
        );
    }

    let attackable_by_summoner = attackable_by_summoner(source)? || attackable_by_summoner(target)?;
    if attackable_by_summoner {
        return resolve_rank(
            RepresentedGetReactionInputLikeCpp {
                attackable_by_summoner: true,
                ..reaction_input(
                    source, target, false, false, false, false, false, false, false, None, None,
                )
            },
            None,
            None,
            FactionReactionCatalogs::new(None, None, None),
        );
    }

    with_owner_or_self(manager, map_key, source, |source_owner| {
        with_owner_or_self(manager, map_key, target, |target_owner| {
            let same_owner = ptr::eq(source_owner.unit(), target_owner.unit());
            if same_owner {
                return resolve_rank(
                    reaction_input(
                        source, target, true, false, false, false, false, false, false, None, None,
                    ),
                    None,
                    None,
                    FactionReactionCatalogs::new(None, None, None),
                );
            }
            with_affecting_player(manager, map_key, source, |source_player| {
                with_affecting_player(manager, map_key, target, |target_player| {
                    let same_player_owner = source_player
                        .zip(target_player)
                        .is_some_and(|(source, target)| ptr::eq(source, target));
                    let duel = if same_player_owner {
                        false
                    } else {
                        source_player
                            .zip(target_player)
                            .is_some_and(|(source, target)| {
                                source.is_dueling_opponent_in_progress_like_cpp(target.guid())
                            })
                    };
                    let input = reaction_input(
                        source,
                        target,
                        same_owner,
                        source_player.is_some(),
                        target_player.is_some(),
                        same_player_owner,
                        duel,
                        false,
                        target_player.is_some_and(|player| player.guid() == target_session_guid),
                        source_player,
                        target_player,
                    );
                    let source_owner = source_player.map(|player| {
                        FactionReactionOwner::new(
                            player.reputation_like_cpp(),
                            player.unit().data().race,
                            player.unit().data().class_id,
                        )
                    });
                    let target_owner = target_player.map(|player| {
                        FactionReactionOwner::new(
                            player.reputation_like_cpp(),
                            player.unit().data().race,
                            player.unit().data().class_id,
                        )
                    });
                    resolve_rank_with_raid(
                        input,
                        source_owner,
                        target_owner,
                        FactionReactionCatalogs::new(
                            faction_store,
                            faction_templates,
                            friendship_reactions,
                        ),
                        || match (source_player, target_player) {
                            (Some(source_player), Some(target_player)) => inherited_raid_relation(
                                manager,
                                map_key,
                                source_player,
                                target_player,
                            )
                            .map_err(|_| FactionReactionError::UnknownRaidRelation),
                            _ => Ok(false),
                        },
                    )
                })?
            })?
        })?
    })?
}

fn reaction_input(
    source: SelectedUnit<'_>,
    target: SelectedUnit<'_>,
    same_owner: bool,
    source_has_player_owner: bool,
    target_has_player_owner: bool,
    same_player_owner: bool,
    duel: bool,
    same_raid: bool,
    target_owner_is_session: bool,
    source_affecting_player: Option<&Player>,
    target_affecting_player: Option<&Player>,
) -> RepresentedGetReactionInputLikeCpp {
    let source_unit = source.unit();
    let target_unit = target.unit();
    RepresentedGetReactionInputLikeCpp {
        self_faction_template_id: source_unit.data().faction_template as u32,
        target_faction_template_id: target_unit.data().faction_template as u32,
        same_object: false,
        attackable_by_summoner: false,
        same_charmer_or_owner_or_self: same_owner,
        self_has_player_owner: source_has_player_owner,
        target_has_player_owner,
        target_player_owner_is_current_session: target_owner_is_session,
        target_owner_forced_rank_for_self: None,
        same_player_owner,
        duel_in_progress: duel,
        same_raid,
        self_unit_player_controlled: player_controlled(source_unit),
        target_unit_player_controlled: player_controlled(target_unit),
        self_ffa_pvp: source_unit
            .pvp_flags_like_cpp()
            .contains(UnitPvpFlags::FFA_PVP),
        target_ffa_pvp: target_unit
            .pvp_flags_like_cpp()
            .contains(UnitPvpFlags::FFA_PVP),
        self_ignores_reputation: source_affecting_player
            .is_some_and(|player| ignores_reputation(player.unit())),
        target_ignores_reputation: ignores_reputation(target_unit),
        target_is_unit: true,
        self_player_contested_pvp: source_affecting_player.is_some_and(contested_pvp),
        target_player_contested_pvp: target_affecting_player.is_some_and(contested_pvp),
    }
}

fn resolve_rank(
    input: RepresentedGetReactionInputLikeCpp,
    source_owner: Option<FactionReactionOwner<'_>>,
    target_owner: Option<FactionReactionOwner<'_>>,
    catalogs: FactionReactionCatalogs<'_>,
) -> Result<bool, SelectedCasterLevelError> {
    resolve_faction_reaction(input, source_owner, target_owner, catalogs)
        .map(|rank| rank >= ReputationRankLikeCpp::Friendly)
        .map_err(|_| SelectedCasterLevelError::ReactionUnavailable)
}

fn resolve_rank_with_raid(
    input: RepresentedGetReactionInputLikeCpp,
    source_owner: Option<FactionReactionOwner<'_>>,
    target_owner: Option<FactionReactionOwner<'_>>,
    catalogs: FactionReactionCatalogs<'_>,
    raid_relation: impl FnOnce() -> Result<bool, FactionReactionError>,
) -> Result<bool, SelectedCasterLevelError> {
    resolve_faction_reaction_with_raid(input, source_owner, target_owner, catalogs, raid_relation)
        .map(|rank| rank >= ReputationRankLikeCpp::Friendly)
        .map_err(|_| SelectedCasterLevelError::ReactionUnavailable)
}

fn attackable_by_summoner(unit: SelectedUnit<'_>) -> Result<bool, SelectedCasterLevelError> {
    match unit {
        SelectedUnit::Player(_) => Ok(false),
        SelectedUnit::Creature {
            pet_owner_guid: Some(_),
            ..
        } => Ok(false),
        SelectedUnit::Creature {
            body,
            pet_owner_guid: None,
        } if body.is_summon_like_cpp() => Err(SelectedCasterLevelError::ReactionUnavailable),
        SelectedUnit::Creature { .. } => Ok(false),
    }
}

fn player_controlled(unit: &Unit) -> bool {
    UnitFlags::from_bits_retain(unit.data().flags).contains(UnitFlags::PLAYER_CONTROLLED)
}

fn ignores_reputation(unit: &Unit) -> bool {
    UnitFlags2::from_bits_retain(unit.data().flags2).contains(UnitFlags2::IGNORE_REPUTATION)
}

fn contested_pvp(player: &Player) -> bool {
    player.has_player_flag(0x0000_0100)
}

fn with_owner_or_self<R>(
    manager: &MapManager,
    map_key: MapKey,
    unit: SelectedUnit<'_>,
    read: impl FnOnce(SelectedUnit<'_>) -> R,
) -> Result<R, SelectedCasterLevelError> {
    let Some(owner_guid) = unit.control_guid()? else {
        return Ok(read(unit));
    };
    let mut read = Some(read);
    match with_unit_in_map(manager, map_key, owner_guid, |owner| {
        read.take().map(|read| read(owner))
    })? {
        Some(Some(value)) => Ok(value),
        Some(None) => Err(SelectedCasterLevelError::CasterUnavailable),
        None => read
            .take()
            .map(|read| read(unit))
            .ok_or(SelectedCasterLevelError::CasterUnavailable),
    }
}

fn with_affecting_player<R>(
    manager: &MapManager,
    map_key: MapKey,
    unit: SelectedUnit<'_>,
    read: impl FnOnce(Option<&Player>) -> R,
) -> Result<R, SelectedCasterLevelError> {
    let Some(owner_guid) = unit.control_guid()? else {
        return Ok(read(unit.player()));
    };
    let mut read = Some(read);
    match with_unit_in_map(manager, map_key, owner_guid, |owner| {
        with_owner_player(manager, map_key, owner, |player| {
            read.take().map(|read| read(player))
        })
    })? {
        Some(Ok(Some(value))) => Ok(value),
        Some(Ok(None)) => Err(SelectedCasterLevelError::CasterUnavailable),
        Some(Err(error)) => Err(error),
        None => read
            .take()
            .map(|read| read(None))
            .ok_or(SelectedCasterLevelError::CasterUnavailable),
    }
}

fn with_owner_player<R>(
    manager: &MapManager,
    map_key: MapKey,
    owner: SelectedUnit<'_>,
    read: impl FnOnce(Option<&Player>) -> R,
) -> Result<R, SelectedCasterLevelError> {
    let Some(player_guid) = owner.control_guid()?.filter(|guid| guid.is_player()) else {
        return Ok(read(owner.player()));
    };
    let mut read = Some(read);
    match with_unit_in_map(manager, map_key, player_guid, |player| {
        read.take().map(|read| read(player.player()))
    })? {
        Some(Some(value)) => Ok(value),
        Some(None) => Err(SelectedCasterLevelError::CasterUnavailable),
        None => read
            .take()
            .map(|read| read(None))
            .ok_or(SelectedCasterLevelError::CasterUnavailable),
    }
}

fn inherited_raid_relation(
    manager: &MapManager,
    map_key: MapKey,
    source: &Player,
    target: &Player,
) -> Result<bool, SelectedCasterLevelError> {
    with_owner_or_self(
        manager,
        map_key,
        SelectedUnit::Player(source),
        |source_unit| {
            with_owner_or_self(
                manager,
                map_key,
                SelectedUnit::Player(target),
                |target_unit| inherited_raid_units(source_unit, target_unit),
            )?
        },
    )?
}

fn inherited_raid_units(
    source: SelectedUnit<'_>,
    target: SelectedUnit<'_>,
) -> Result<bool, SelectedCasterLevelError> {
    if ptr::eq(source.unit(), target.unit()) {
        return Ok(true);
    }
    match (source, target) {
        (SelectedUnit::Player(source), SelectedUnit::Player(target)) => {
            let source_group = source
                .gameplay_state()
                .group
                .as_ref()
                .map(|group| group.group_guid);
            let target_group = target
                .gameplay_state()
                .group
                .as_ref()
                .map(|group| group.group_guid);
            Ok(source_group.is_some_and(|guid| !guid.is_empty() && target_group == Some(guid)))
        }
        (SelectedUnit::Player(_), SelectedUnit::Creature { body, .. })
        | (SelectedUnit::Creature { body, .. }, SelectedUnit::Player(_)) => Ok(
            CreatureStaticFlags4::from_bits_retain(body.lifecycle_metadata().static_flags[3])
                .contains(CreatureStaticFlags4::TREAT_AS_RAID_UNIT_FOR_HELPFUL_SPELLS),
        ),
        (
            SelectedUnit::Creature { body: source, .. },
            SelectedUnit::Creature { body: target, .. },
        ) => Ok(source.unit().data().faction_template == target.unit().data().faction_template),
    }
}

#[cfg(test)]
#[path = "../../unit_tests/aura_session/selected_reaction.rs"]
mod tests;
