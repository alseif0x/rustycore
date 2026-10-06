use wow_constants::InventoryType;

impl crate::session::HubRef<'_> {
    /// Resolve C++ `ItemTemplate::GetInventoryType()` for equipment-slot mapping.
    pub fn item_template_inventory_type(&self, item_id: u32) -> Option<u8> {
        self.catalogs
            .item_storage_template(item_id)
            .map(|template| template.inventory_type as u8)
            .filter(|&inventory_type| inventory_type != InventoryType::NonEquip as u8)
    }
}
