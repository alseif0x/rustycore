//! 02245dcd ObjectMgr::LoadPlayerInfo/GetSkillRangeType and
//! Player::InitializeSkillFields/LearnDefaultSkills/LearnDefaultSkill.
//! Source inputs/call requests only: SetSkill spell/aura/child/criteria effects
//! are mandatory consumers, not a saved or learned Player skill state.
use super::{SourceError, WorldSources};
use std::{collections::BTreeMap, sync::Arc};
use wow_data::forever_birth::{BirthCatalog, SkillRaceClassRecord, race_in_mask};
mod lookup;
pub use lookup::SkillLookupCounts;

// Player.h::PLAYER_MAX_SKILLS / UpdateFields.h::SkillInfo::SkillLineID.
// This is the pinned source limit, not a native 70170 update-field wire claim.
const MAX_SKILLS: usize = crate::forever::player::PLAYER_MAX_SKILLS;
const ALWAYS_MAX: i32 = 0x10;

/// StartingRank is one, but Rank is still zero. Preallocation does not learn.
pub struct SkillFieldSeed {
    skill: u32,
    slot: u16,
}
impl SkillFieldSeed {
    pub fn skill(&self) -> u32 {
        self.skill
    }
    pub fn slot(&self) -> u16 {
        self.slot
    }
    pub fn starting_rank(&self) -> u16 {
        1
    }
    pub fn line_field(&self) -> u16 {
        self.skill as u16
    }
}

/// A LearnDefaultSkill -> SetSkill input. Caller checks HasSkill immediately
/// before each request, after executing the previous request's spell effects.
/// Keeping duplicate skill IDs is intentional, not a duplicate-grant decision.
pub struct DefaultSkillRequest {
    source_record: u32,
    skill: u16,
    step: u16,
    rank: u16,
    maximum: u16,
}
impl DefaultSkillRequest {
    pub fn source_record(&self) -> u32 {
        self.source_record
    }
    pub fn skill(&self) -> u16 {
        self.skill
    }
    pub fn step(&self) -> u16 {
        self.step
    }
    pub fn rank(&self) -> u16 {
        self.rank
    }
    pub fn maximum(&self) -> u16 {
        self.maximum
    }
}

pub struct InitialSkillFields {
    fields: Vec<SkillFieldSeed>,
    requests: Vec<DefaultSkillRequest>,
}
impl InitialSkillFields {
    /// Execute only source preallocation into the future Player's canonical
    /// field storage. Default requests still need real SetSkill/spell effects.
    pub fn initialize_player_skills(&self) -> crate::forever::player::PlayerSkills {
        crate::forever::player::PlayerSkills::initialize(self)
    }
    pub fn fields(&self) -> &[SkillFieldSeed] {
        &self.fields
    }
    pub fn default_requests(&self) -> &[DefaultSkillRequest] {
        &self.requests
    }
}

// Indices only; raw birth records have one shared immutable authority.
struct PairSources {
    fields: Vec<u32>,
    defaults: Vec<u32>,
}
pub(super) struct SkillSources {
    birth: Arc<BirthCatalog>,
    pairs: BTreeMap<(u8, u8), PairSources>,
    lookup: Option<lookup::RaceClassLookup>,
}

fn matches(row: &SkillRaceClassRecord, race: u8, class: u8) -> bool {
    // Class admission precedes this shift. Zero masks are source wildcards.
    (row.race_mask == 0 || race_in_mask(row.race_mask, u32::from(race)))
        && (row.class_mask == 0 || row.class_mask & (1_i32 << (class - 1)) != 0)
}

impl WorldSources {
    /// Borrow the same final catalog admitted by birth skills and RC lookup.
    /// Player operations must not accept an unrelated second BirthCatalog.
    pub(crate) fn skill_birth_catalog(&self) -> Result<&BirthCatalog, SourceError> {
        self.skill_sources
            .as_ref()
            .map(|sources| sources.birth.as_ref())
            .ok_or(SourceError::MissingBirthSkillSources)
    }

    /// Source PlayerInfo skills: storage-ordered default RC identities. This
    /// differs from unordered first-match lookup; borrow final payloads only.
    pub(crate) fn default_skill_records(
        &self,
        race: u8,
        class: u8,
    ) -> Result<impl Iterator<Item = &SkillRaceClassRecord>, SourceError> {
        let sources = self
            .skill_sources
            .as_ref()
            .ok_or(SourceError::MissingBirthSkillSources)?;
        let pair = sources
            .pairs
            .get(&(race, class))
            .ok_or(SourceError::MissingDefinition)?;
        Ok(pair.defaults.iter().map(move |id| {
            sources
                .birth
                .race_class_record(*id)
                .expect("immutable RC index")
        }))
    }

    /// One GetSkillRangeType/LearnDefaultSkill rule shared by startup planning
    /// and the live canonical consumer. MinLevel/HasSkill are caller gates.
    pub(crate) fn default_skill_request(
        &self,
        rc: &SkillRaceClassRecord,
        class: u8,
        level: u8,
    ) -> Option<DefaultSkillRequest> {
        let birth = self.skill_birth_catalog().ok()?;
        let line = birth.skill_line(u32::from(rc.skill))?;
        let tier = self.skill_tier_value(rc.tier as i32 as u32, 0);
        let (step, rank, maximum) = if let Some(tier) = tier {
            let maximum = tier as u16;
            (1, starting_rank(rc.flags, class, level, maximum), maximum)
        } else if rc.skill == 960 || line.category == 8 {
            (0, 1, 1)
        } else if line.category == 10 {
            (0, 300, 300)
        } else {
            let maximum = u16::from(level) * 5;
            (0, starting_rank(rc.flags, class, level, maximum), maximum)
        };
        Some(DefaultSkillRequest {
            source_record: rc.id,
            skill: rc.skill,
            step,
            rank,
            maximum,
        })
    }

    /// Immutable startup index; no learned-rank state and no unordered winner.
    /// GetSkillRaceClassInfo's existence is order-independent here; choosing
    /// its first matching record for later SetSkill uses the separate admitted
    /// native-container lookup phase, not these storage-ordered default requests.
    pub fn with_birth_skills(mut self, birth: Arc<BirthCatalog>) -> Result<Self, SourceError> {
        let mut pairs = BTreeMap::new();
        for &(race, class) in self.definitions.keys() {
            if !(1..=15).contains(&class) {
                return Err(SourceError::InvalidClass);
            }
            let fields = birth
                .skill_lines()
                .filter(|line| {
                    birth
                        .race_class_records()
                        .any(|rc| u32::from(rc.skill) == line.id && matches(rc, race, class))
                })
                .take(MAX_SKILLS)
                .map(|line| line.id)
                .collect();
            // Source iterates the RC store in ascending storage ID, not the
            // unordered GetSkillRaceClassInfo index. Do not deduplicate/reorder.
            let defaults = birth
                .race_class_records()
                .filter(|rc| rc.availability == 1 && matches(rc, race, class))
                .map(|rc| rc.id)
                .collect();
            pairs.insert((race, class), PairSources { fields, defaults });
        }
        self.skill_sources = Some(SkillSources {
            birth,
            pairs,
            lookup: None,
        });
        Ok(self)
    }

    pub fn initial_skill_fields(
        &self,
        race: u8,
        class: u8,
        level: u8,
    ) -> Result<InitialSkillFields, SourceError> {
        if level == 0 {
            return Err(SourceError::InvalidLevel);
        }
        self.definition(race, class)
            .ok_or(SourceError::MissingDefinition)?;
        let sources = self
            .skill_sources
            .as_ref()
            .ok_or(SourceError::MissingBirthSkillSources)?;
        let pair = sources
            .pairs
            .get(&(race, class))
            .ok_or(SourceError::MissingDefinition)?;
        let fields = pair
            .fields
            .iter()
            .enumerate()
            .map(|(slot, &skill)| SkillFieldSeed {
                skill,
                slot: slot as u16,
            })
            .collect();
        let mut requests = Vec::new();
        for rc in self.default_skill_records(race, class)? {
            // int8 MinLevel and uint8 player level both promote to signed int.
            if i16::from(rc.min_level) > i16::from(level) {
                continue;
            }
            if let Some(request) = self.default_skill_request(rc, class, level) {
                requests.push(request);
            }
        }
        Ok(InitialSkillFields { fields, requests })
    }

    pub fn birth_skill_source_counts(&self) -> Option<[usize; 3]> {
        let sources = self.skill_sources.as_ref()?;
        Some([
            sources.pairs.len(),
            sources.pairs.values().map(|p| p.fields.len()).sum(),
            sources.pairs.values().map(|p| p.defaults.len()).sum(),
        ])
    }
}

fn starting_rank(flags: i32, class: u8, level: u8, maximum: u16) -> u16 {
    if flags & ALWAYS_MAX != 0 {
        return maximum;
    }
    if class == 6 {
        // Source uint8 level promotes to signed int before subtraction and
        // multiplication, then narrows to uint16, including level zero. The
        // original startup entry still rejects zero before reaching this rule.
        return (((i32::from(level) - 1) * 5) as u16).max(1).min(maximum);
    }
    1
}

#[cfg(test)]
mod tests;
