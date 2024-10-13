use actix::Addr;

use super::engine_logic as logic;
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::game_state;
use crate::entity::game::component::square::Switch;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

use crate::entity::engine::actor::engine_dispatcher as dispatcher;

struct Score(i32);

#[derive(Debug)]
pub struct EngineMinimax {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
    zobrist_table : zobrist::Zobrist,    
    max_depth: u8,
}
impl EngineMinimax {
    pub fn new(debug_actor_opt: Option<debug::DebugActor>, zobrist_table: zobrist::Zobrist, max_depth: u8) -> Self {
        Self {
            id_number: "".to_string(),
            debug_actor_opt,
            zobrist_table,
            max_depth,
        }
    }
    pub fn set_id_number(&mut self, id_number: &str) {
        self.id_number = id_number.to_string();
    }
    fn minimax(&self, game: &game_state::GameState, current_depth: u8) -> (bitboard::BitBoardMove, Score) {
        let mut max_score = i32::MIN;
        let mut best_move = game.moves()[0];
        for m in game.moves() {
            // TODO: optimize that
            let long_algebric_move = long_notation::LongAlgebricNotationMove::build_from_b_move(*m);
            // if current_depth <=1 {
            //     println!("{} {}", current_depth, long_algebric_move.cast());
            // }
            //let _ = game.clone().play_moves(&[long_algebric_move], &self.zobrist_table, self.debug_actor_opt.clone()).unwrap();
            let _ = game.clone().play_moves(&[long_algebric_move], &self.zobrist_table, None).unwrap();            
            let score = if current_depth < self.max_depth {
                let (_, score) = self.minimax(&game.clone(), current_depth + 1);
                score
            } else {
                evaluate(game.bit_position())
            };
            if score.0 > max_score {
                best_move = *m;
                max_score = score.0;
            }
        }
        (best_move, Score(max_score))
    }    
}
unsafe impl Send for EngineMinimax {}

const MINIMAX_ENGINE_ID_NAME: &str = "Minimax engine";
const MINIMAX_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineMinimax {
    fn id(&self) -> logic::EngineId {
        let name = format!("{} {}", MINIMAX_ENGINE_ID_NAME.to_owned(), self.id_number)
            .trim()
            .to_string();
        let author = MINIMAX_ENGINE_ID_AUTHOR.to_owned();
        logic::EngineId::new(name, author)
    }
    fn find_best_move(
        &self,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        game: game_state::GameState,
    ) {
        // First generate moves
        let moves = logic::gen_moves(&game.bit_position());
        if !moves.is_empty() {
            let (best_move, _) = self.minimax(&game, 0);
            self_actor.do_send(dispatcher::handler_engine::EngineStopThinking);            
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

fn evaluate_one_side(bitboard: &bitboard::BitBoards) -> u32 {
    let n_rooks = bitboard.rooks().bitboard().iter().count() as u32;    
    let n_knights = bitboard.bishops().bitboard().iter().count() as u32;        
    let n_bishops = bitboard.bishops().bitboard().iter().count() as u32;
    let n_queens = bitboard.queens().bitboard().iter().count() as u32;    
    let n_pawns = bitboard.pawns().bitboard().iter().count() as u32;    
    let score = n_rooks * 5 + n_knights * 3 + n_bishops * 3 + n_queens * 10 + n_pawns;
    score
}

fn evaluate(bit_position: &bitboard::BitPosition) -> Score {
    let color = bit_position.bit_position_status().player_turn();
    let score_current = evaluate_one_side(bit_position.bit_boards_white_and_black().bit_board(&color));
    let score_opponent = evaluate_one_side(bit_position.bit_boards_white_and_black().bit_board(&color.switch()));    
    let score = score_current as i32 - score_opponent as i32;
    Score(score)
}