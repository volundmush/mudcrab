use std::collections::{HashMap, HashSet};
use legion::Entity;
use std::time::{Instant, Duration};

pub struct UsersOnline(HashMap<Entity, Instant>);
pub struct MudSessions(HashMap<Entity, Entity>);
pub struct Modules(HashSet<Entity>);