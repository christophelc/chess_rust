use actix::prelude::*;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;

use super::engine_logic as logic;
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::game::component::game_state;
use crate::entity::stat::actor::stat_entity;
use crate::monitoring::debug;

#[derive(Debug)]
pub struct EngineDummy {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
}
impl EngineDummy {
    pub fn new(debug_actor_opt: Option<debug::DebugActor>) -> Self {
        Self {
            id_number: "".to_string(),
            debug_actor_opt,
        }
    }
    pub fn set_id_number(&mut self, id_number: &str) {
        self.id_number = id_number.to_string();
    }
}
unsafe impl Send for EngineDummy {}

const DUMMY_ENGINE_ID_NAME: &str = "Random engine";
const DUMMY_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineDummy {
    fn id(&self) -> logic::EngineId {
        let name = format!("{} {}", DUMMY_ENGINE_ID_NAME.to_owned(), self.id_number)
            .trim()
            .to_string();
        let author = DUMMY_ENGINE_ID_AUTHOR.to_owned();
        logic::EngineId::new(name, author)
    }
    fn find_best_move(
        &self,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        _stat_actor_opt: Option<stat_entity::StatActor>,
        game: game_state::GameState,
    ) {
        let moves = game.gen_moves();
        let mut rng = ChaCha12Rng::from_entropy();
        let best_move_opt = moves.choose(&mut rng).cloned();
        if let Some(best_move) = best_move_opt {
            self_actor.do_send(dispatcher::handler_engine::EngineStopThinking::new(None));
            let reply = dispatcher::handler_engine::EngineEndOfAnalysis(best_move);
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "EngineDummy of id {:?} reply is: '{:?}'",
                    self.id(),
                    reply
                )));
            }
            self_actor.do_send(reply);
        } else {
            // FIXME: Do nothing. The engine should be put asleep
            panic!("To be implemented. When EndGame detected in game_manager, stop the engines")
        }
    }
}
