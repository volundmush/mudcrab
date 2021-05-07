use legion::Entity;
use std::collections::{
    HashMap, HashSet
};
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum MudObjectType {
    Alliance,
    Board,
    Channel,
    Dimension,
    Faction,
    HeavenlyBody,
    Item,
    Mobile,
    Room,
    Sector,
    User,
    Vehicle,
    Wilderness,
    Zone,
}

#[derive(Debug)]
pub struct MudProtoTypeComponent {
    pub entity: Entity,
    pub objtype: MudObjectType
}


#[derive(Debug)]
pub struct MudObjectComponent {
    pub entity: Entity,
    pub objtype: MudObjectType
}

#[derive(Default, Debug, Clone)]
pub struct InventoryBase {
    pub commodities: HashMap<usize, usize>,
    pub items: HashSet<Entity>
}


#[derive(Default, Debug, Clone)]
pub struct InventoryComponent(InventoryBase);
#[derive(Default, Debug, Clone)]
pub struct FuelBayComponent(InventoryBase);
#[derive(Default, Debug, Clone)]
pub struct ResourceHopperComponent(InventoryBase);
#[derive(Default, Debug, Clone)]
pub struct ShipHangarComponent(InventoryBase);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocationType {
    Room,
    Inventory,
    FuelBay,
    ResourceHopper,
    ShipHangar,
    Sector(f64, f64, f64),
    Wilderness(u64, u64, u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationComponent {
    pub ltype: LocationType,
    pub entity: Entity
}