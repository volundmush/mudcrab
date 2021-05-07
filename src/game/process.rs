use std::time::{Instant, Duration};
use legion::*;
use crate::game::objects::{MudSession};

#[derive(Debug)]
pub struct ProcessComponent {
    pub created: Instant,
    pub id: usize,
    pub enactor_user: Option<Entity>,
    pub enactor_obj: Option<Entity>,
    pub executor: Option<Entity>,
    pub wait_for: Option<Duration>,
    pub command: String,
    pub split_actions: bool
}

impl ProcessComponent {
    pub fn from_command(sess: &MudSession, id: usize, command: String) -> Self {
        Self {
            created: Instant::now(),
            id,
            enactor_user: Some(sess.user),
            enactor_obj: Some(sess.puppet),
            executor: Some(sess.puppet),
            wait_for: None,
            command,
            split_actions: false
        }
    }
}