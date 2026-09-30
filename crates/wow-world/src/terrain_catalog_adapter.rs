use std::io;
use std::path::Path;

use wow_data::AreaTableStore;

pub(crate) fn zone_and_area_for_position_like_cpp(
    data_dir: impl AsRef<Path>,
    map_id: u32,
    x: f32,
    y: f32,
    area_store: Option<&AreaTableStore>,
    map_area_id_fallback: impl FnOnce(u32) -> u32,
) -> io::Result<(u32, u32)> {
    crate::map_manager::zone_and_area_for_position_like_cpp(
        data_dir,
        map_id,
        x,
        y,
        |area_id| {
            area_store
                .and_then(|store| store.get(area_id))
                .filter(|area| area.parent_area_id != 0 && area.is_subzone_like_cpp())
                .map(|area| u32::from(area.parent_area_id))
        },
        map_area_id_fallback,
    )
}
