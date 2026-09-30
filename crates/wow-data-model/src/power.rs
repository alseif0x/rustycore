use wow_constants::PowerType;

pub trait PlayerPowerIndexResolver {
    fn power_index_by_class(&self, power: PowerType, class_id: u8) -> Option<usize>;
}
