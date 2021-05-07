use legion::Entity;
use std::collections::{
    HashMap, HashSet, VecDeque
};

use serde_derive::{Serialize, Deserialize};
use std::time::{Instant, Duration};
use crate::net::{ProtocolOutEvent, ProtocolEvent};

#[derive(Debug)]
pub struct MudSession {
    pub user: Entity,
    pub player: Entity,
    pub puppet: Entity,
    pub created: Instant,
    pub in_events: VecDeque<ProtocolEvent>,
    pub out_events: VecDeque<ProtocolOutEvent>,
    pub connections: HashSet<Entity>
}

#[derive(Debug)]
pub struct ModuleComponent {
    pub display_name: String,
    pub sys_name: String,
    pub prototypes: HashSet<Entity>,
    pub objects: HashSet<Entity>,
    pub protected: bool
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash)]
pub enum MudObjectType {
    Alliance,
    Board,
    Channel,
    Dimension,
    Faction,
    HeavenlyBody,
    Item,
    Mobile,
    Player,
    Room,
    Sector,
    User,
    Vehicle,
    Wilderness,
    Zone,
}

#[derive(Debug)]
pub struct MudProtoTypeComponent {
    pub objid: String,
    pub name: String,
    pub entity: Entity,
    pub objtype: MudObjectType,
    pub module: Entity,
}

#[derive(Debug)]
pub struct MudObjectSession {
    pub session: Entity
}

#[derive(Debug)]
pub struct MudObjectComponent {
    pub objid: String,
    pub name: String,
    pub entity: Entity,
    pub objtype: MudObjectType,
    pub module: Entity,
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