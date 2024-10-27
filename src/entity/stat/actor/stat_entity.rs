pub mod handler_stat;

use std::collections::HashMap;

use actix::prelude::*;

use crate::entity::engine::component::engine_logic as logic;
use crate::{entity::stat::component::stat_data, monitoring::debug};

pub struct StatEntity {
    debug_actor_opt: Option<debug::DebugActor>,
    stats: HashMap<logic::EngineId, stat_data::StatData>,
}
impl StatEntity {
    pub fn new(debug_actor_opt: Option<debug::DebugActor>) -> Self {
        Self {
            debug_actor_opt,
            stats: HashMap::default(),
        }
    }
}
impl Actor for StatEntity {
    type Context = Context<Self>;
}

pub type StatActor = Addr<StatEntity>;
