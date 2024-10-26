use actix::{Addr, Handler, Message};

use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::uci::actor::uci_entity;
use crate::monitoring::debug;

use super::GameManager;

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct GetCurrentEngineAsync {
    uci_caller: uci_entity::UciActor,
}
impl GetCurrentEngineAsync {
    pub fn new(uci_caller: uci_entity::UciActor) -> Self {
        Self { uci_caller }
    }
}
impl Handler<GetCurrentEngineAsync> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: GetCurrentEngineAsync, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(
                "game_manager_actor receive GetCurrentEngineAsync".to_string(),
            ));
        }
        if let Some(game_state) = &self.game_state_opt {
            let bitboard_position = game_state.bit_position();
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            let engine_actor_opt = self.players.get_player_into(color).get_engine().cloned();
            if let Some(engine_actor) = engine_actor_opt {
                let reply =
                    dispatcher::handler_engine::EngineGetIdAsync::new(msg.uci_caller.clone());
                engine_actor.do_send(reply);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct GetCurrentEngine;
impl Message for GetCurrentEngine {
    type Result = Option<Addr<dispatcher::EngineDispatcher>>;
}

impl Handler<GetCurrentEngine> for GameManager {
    type Result = Option<Addr<dispatcher::EngineDispatcher>>;

    fn handle(&mut self, msg: GetCurrentEngine, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        if let Some(game_state) = &self.game_state_opt {
            let bitboard_position = game_state.bit_position();
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            self.players.get_player_into(color).get_engine().cloned()
        } else {
            None
        }
    }
}
