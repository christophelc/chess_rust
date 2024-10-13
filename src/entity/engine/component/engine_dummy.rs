use actix::prelude::*;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;

use super::engine_logic as logic;
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::game::component::bitboard::{self, piece_move::GenMoves};
use crate::monitoring::debug;

#[derive(Debug, Clone)]
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
    pub fn set_id_number(&self, id_number: &str) -> Self {
        Self {
            id_number: id_number.to_string(),
            ..self.clone()
        }
    }
}
unsafe impl Send for EngineDummy {}

const DUMMY_ENGINE_ID_NAME: &str = "Random engine";
const DUMMY_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

pub fn gen_moves(bit_position: &bitboard::BitPosition) -> Vec<bitboard::BitBoardMove> {
    let bit_boards_white_and_black = bit_position.bit_boards_white_and_black();
    let bit_position_status = bit_position.bit_position_status();
    let color = &bit_position_status.player_turn();
    let check_status = bit_boards_white_and_black.check_status(color);
    let capture_en_passant = bit_position_status.pawn_en_passant();
    bit_boards_white_and_black.gen_moves_for_all(
        color,
        check_status,
        &capture_en_passant,
        bit_position_status,
    )
}

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
        bit_position: bitboard::BitPosition,
    ) {
        // First generate moves
        let moves = gen_moves(&bit_position);
        // And then stop thinking and clear positino
        self_actor.do_send(dispatcher::handler_engine::EngineStopThinking);
        let mut rng = ChaCha12Rng::from_entropy();
        let best_move_opt = moves.choose(&mut rng).cloned();
        if let Some(best_move) = best_move_opt {
            let reply = dispatcher::handler_engine::EngineBestMoveFound(best_move);
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
