use wow_entities::{
    ActivePlayerDataUpdate, AreaTriggerDataUpdate, AreaTriggerValuesUpdate, BagValuesUpdate,
    ContainerDataUpdate, Conversation, ConversationDataUpdate, ConversationValuesUpdate, Corpse,
    CorpseDataUpdate, CorpseValuesUpdate, DynamicObjectDataUpdate, DynamicObjectValuesUpdate,
    GameObjectDataUpdate, GameObjectValuesUpdate, ItemDataUpdate, ItemValuesUpdate,
    ObjectDataUpdate, PlayerDataUpdate, PlayerValuesUpdate, SceneObject, SceneObjectDataUpdate,
    SceneObjectValuesUpdate, TYPEID_ACTIVE_PLAYER, TYPEID_AREA_TRIGGER, TYPEID_CONTAINER,
    TYPEID_CONVERSATION, TYPEID_CORPSE, TYPEID_DYNAMIC_OBJECT, TYPEID_GAME_OBJECT, TYPEID_ITEM,
    TYPEID_OBJECT, TYPEID_PLAYER, TYPEID_SCENE_OBJECT, TYPEID_UNIT, UnitDataUpdate,
    UnitValuesUpdate,
};
use wow_packet::packets::update::{
    ActivePlayerDataValuesUpdate as PacketActivePlayerDataValuesUpdate, AreaTriggerCreateData,
    AreaTriggerDataValuesUpdate, AreaTriggerOrbitCreateData, AreaTriggerPosition2CreateData,
    AreaTriggerPosition3CreateData, AreaTriggerShapeCreateData, ChrCustomizationChoiceValuesUpdate,
    ContainerDataValuesUpdate, ConversationActorValuesUpdate, ConversationCreateData,
    ConversationDataValuesUpdate, ConversationLineValuesUpdate, CorpseCreateData,
    CorpseDataValuesUpdate, DynamicObjectDataValuesUpdate, GameObjectDataValuesUpdate,
    ItemBonusKeyValuesUpdate, ItemDataValuesDeltaUpdate, ItemEnchantmentValuesUpdate,
    ItemModListValuesUpdate, ItemModValuesUpdate, ObjectDataValuesUpdate,
    PlayerDataValuesDeltaUpdate, RestInfoValuesUpdate, ScaleCurveValuesUpdate,
    SceneObjectCreateData, SceneObjectDataValuesUpdate, SocketedGemValuesUpdate,
    UnitDataValuesDeltaUpdate, UpdateObject, VisibleItemValuesUpdate, VisualAnimValuesUpdate,
};

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "entity_update_bridge/tests/mod.rs"]
mod tests;
