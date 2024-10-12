use actix::prelude::*;

use crate::entity::game::component::bitboard;

use crate::entity::engine::actor::engine_dispatcher;

#[derive(Debug, Clone)]
pub struct EngineId {
    name: String,
    author: String,
}
impl EngineId {
    pub fn new(name: String, author: String) -> Self {
        Self { name, author }
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn author(&self) -> String {
        self.author.clone()
    }
}
pub trait Engine {
    fn id(&self) -> EngineId;

    fn think(
        &self,
        self_actor: Addr<engine_dispatcher::EngineDispatcher>,
        bit_position: &bitboard::BitPosition,
    );
}
