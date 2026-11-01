use crate::{RepresentedCreatureAuraLikeCpp, WorldEntitiesState};

impl WorldEntitiesState {
    pub fn record_represented_creature_aura_like_cpp(
        &mut self,
        aura: RepresentedCreatureAuraLikeCpp,
    ) {
        self.represented_creature_auras_like_cpp.push(aura);
    }

    pub fn represented_creature_auras_are_empty_like_cpp(&self) -> bool {
        self.represented_creature_auras_like_cpp.is_empty()
    }

    pub fn expired_represented_creature_auras_like_cpp(
        &self,
    ) -> Vec<RepresentedCreatureAuraLikeCpp> {
        self.represented_creature_auras_like_cpp
            .iter()
            .filter(|aura| {
                aura.duration_ms > 0
                    && aura.applied_at.elapsed().as_millis() as u32 >= aura.duration_ms
            })
            .copied()
            .collect()
    }

    pub fn retire_represented_creature_aura_like_cpp(
        &mut self,
        aura: RepresentedCreatureAuraLikeCpp,
    ) {
        self.represented_creature_auras_like_cpp
            .retain(|tracked| *tracked != aura);
    }
}
