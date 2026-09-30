// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map GUID sequences and admitted addon aura provenance.

use super::*;
use crate::map_rules::ensure_map_guid_sequence_source_like_cpp;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// C++ `SPELL_CAST_SOURCE_NORMAL` encoded in a Cast GUID subtype
    /// (`Spell.h:140`, `ObjectGuid.h:227`).
    const SPELL_CAST_SOURCE_NORMAL_LIKE_CPP: u8 = 3;

    pub fn generate_low_guid_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> Result<i64, MapGuidSequenceErrorLikeCpp> {
        ensure_map_guid_sequence_source_like_cpp(high)?;
        Ok(self
            .guid_sequence_generator_like_cpp(high)
            .generator
            .generate())
    }

    /// Compose one spawn-addon `Aura::m_castId` from this Map's shared Cast
    /// sequence. The Creature GUID supplies the target map and realm bits;
    /// the spell id occupies the Cast GUID entry field just as
    /// `Unit::AddAura` does in C++ (`Unit.cpp:11473-11517`).
    pub fn allocate_creature_addon_aura_cast_guid_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        spell_id: u32,
    ) -> Result<ObjectGuid, MapGuidSequenceErrorLikeCpp> {
        let counter = self.generate_low_guid_like_cpp(HighGuid::Cast)?;
        Ok(ObjectGuid::create_world_object(
            HighGuid::Cast,
            Self::SPELL_CAST_SOURCE_NORMAL_LIKE_CPP,
            creature_guid.realm_id(),
            creature_guid.map_id(),
            0,
            spell_id,
            counter,
        ))
    }

    /// Settle the exact addon applications admitted by a detached Creature.
    /// Unit owns the application and its slot; Map owns CastGUID allocation.
    pub fn settle_creature_addon_aura_provenance_like_cpp(
        &mut self,
        creature: &mut Creature,
    ) -> Result<usize, MapGuidSequenceErrorLikeCpp> {
        let pending = creature.take_pending_addon_aura_provenance_like_cpp();
        let mut installed = 0;
        for pending in pending {
            if !creature.can_install_pending_addon_aura_provenance_like_cpp(pending) {
                continue;
            }
            let cast_id =
                self.allocate_creature_addon_aura_cast_guid_like_cpp(creature.guid(), pending.1)?;
            if creature.install_pending_addon_aura_provenance_like_cpp(pending, cast_id) {
                installed += 1;
            }
        }
        Ok(installed)
    }

    /// Settle a canonical Creature after a runtime mutation without holding a
    /// mutable entity borrow while consuming the Map Cast sequence. The first
    /// borrow drains the exact pending batch; allocation happens at Map scope;
    /// the second borrow installs only matching live slots.
    pub fn settle_creature_addon_aura_provenance_by_guid_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) -> Result<Option<usize>, MapGuidSequenceErrorLikeCpp> {
        let Some(pending) = self
            .entity_world
            .with_creature_mut(creature_guid, |creature| {
                creature
                    .take_pending_addon_aura_provenance_like_cpp()
                    .into_iter()
                    .filter(|pending| {
                        creature.can_install_pending_addon_aura_provenance_like_cpp(*pending)
                    })
                    .collect::<Vec<_>>()
            })
        else {
            return Ok(None);
        };

        let mut allocated = Vec::with_capacity(pending.len());
        for pending in pending {
            let cast_id =
                self.allocate_creature_addon_aura_cast_guid_like_cpp(creature_guid, pending.1)?;
            allocated.push((pending, cast_id));
        }

        let installed = self
            .entity_world
            .with_creature_mut(creature_guid, |creature| {
                allocated
                    .into_iter()
                    .filter(|(pending, cast_id)| {
                        creature.install_pending_addon_aura_provenance_like_cpp(*pending, *cast_id)
                    })
                    .count()
            })
            .unwrap_or(0);
        Ok(Some(installed))
    }

    pub fn get_max_low_guid_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> Result<i64, MapGuidSequenceErrorLikeCpp> {
        ensure_map_guid_sequence_source_like_cpp(high)?;
        Ok(self
            .guid_sequence_generator_like_cpp(high)
            .generator
            .next_after_max_used())
    }

    pub fn set_guid_sequence_like_cpp(
        &mut self,
        high: HighGuid,
        next: i64,
    ) -> Result<(), MapGuidSequenceErrorLikeCpp> {
        ensure_map_guid_sequence_source_like_cpp(high)?;
        self.guid_sequence_generator_like_cpp(high)
            .generator
            .set(next);
        Ok(())
    }

    fn guid_sequence_generator_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> &mut MapGuidSequenceGeneratorLikeCpp {
        self.guid_generators
            .entry(high)
            .or_insert_with(|| MapGuidSequenceGeneratorLikeCpp::new(high))
    }
}
