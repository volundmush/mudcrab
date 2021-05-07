use std::collections::{HashMap, HashSet, VecDeque};
use legion::Entity;
use std::time::{Instant, Duration};
use super::objects::{MudObjectType};

#[derive(Default)]
pub struct UsersOnline(pub HashMap<Entity, Instant>);
#[derive(Default)]
pub struct MudSessions(pub HashMap<Entity, Entity>);
#[derive(Default)]
pub struct Modules(pub HashSet<Entity>);
#[derive(Default)]
pub struct ObjTypeIndex(pub HashMap<MudObjectType, HashSet<Entity>>);
#[derive(Default)]
pub struct ProcessCounter(pub usize);
#[derive(Default)]
pub struct ProcessIndex(pub HashMap<usize, Entity>);
#[derive(Default)]
pub struct PendingUserCreations(pub VecDeque<(Entity, String, String)>);
#[derive(Default)]
pub struct PendingUserLogins(pub VecDeque<(Entity, String, String)>);