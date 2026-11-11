//! Frozen Garrison projection; construction, selection and identity reservation remain absent.
pub struct NewGarrisonSave {
    pub site_level_id: u32,
    pub follower_activations_remaining_today: u32,
    pub known_buildings: Vec<u32>,
    pub plots: Vec<Option<NewGarrisonBuilding>>,
    pub followers: Vec<NewGarrisonFollower>,
}

pub struct NewGarrisonBuilding {
    pub plot_instance_id: u32,
    pub building_id: u32,
    pub time_built: i64,
    pub active: bool,
}

pub struct NewGarrisonFollower {
    pub db_id: u64,
    pub follower_id: u32,
    pub quality: u32,
    pub level: u32,
    pub item_level_weapon: u32,
    pub item_level_armor: u32,
    pub xp: u32,
    pub current_building: u32,
    pub current_mission: u32,
    pub status: u32,
    pub abilities: Vec<u32>,
}
