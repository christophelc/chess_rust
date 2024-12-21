use actix::Addr;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use super::engine_logic::{self as logic, Engine};
use super::{engine_alphabeta, engine_mat, score, stat_eval};
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::game::component::bitboard::piece_move::GenMoves;
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::square::Switch;
use crate::entity::game::component::{game_state, player, square};
use crate::entity::stat::actor::stat_entity;
use crate::entity::stat::component::stat_data;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

#[derive(Debug)]
enum IddfsAction {
    Stop,
    Dig(usize),
    Evaluate(usize),
}

pub struct EngineIddfs {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
    zobrist_table: zobrist::Zobrist,
    max_depth: u8,
    engine_alphabeta: engine_alphabeta::EngineAlphaBeta,
    engine_mat_solver: engine_mat::EngineMat,
}
impl EngineIddfs {
    pub fn new(
        debug_actor_opt: Option<debug::DebugActor>,
        zobrist_table: zobrist::Zobrist,
        max_depth: u8,
    ) -> Self {
        assert!(max_depth >= 1);
        Self {
            id_number: "".to_string(),
            debug_actor_opt,
            zobrist_table: zobrist_table.clone(),
            max_depth: max_depth * 2 - 1,
            engine_alphabeta: engine_alphabeta::EngineAlphaBeta::new(
                // fIXME: max_depth here should be dynamic
                None,
                zobrist_table.clone(),
                max_depth,
            ),
            engine_mat_solver: engine_mat::EngineMat::new(
                // fIXME: max_depth here should be dynamic
                None,
                zobrist_table,
                max_depth * 2,
            ),
        }
    }
    pub fn set_id_number(&mut self, id_number: &str) {
        self.id_number = id_number.to_string();
    }

    fn aspiration_window(
        &self,
        game: &mut game_state::GameState,
        transposition_table: &mut score::TranspositionScore,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_eval: &mut stat_eval::StatEval,
        alpha_opt: &mut Option<i32>,
        beta_opt: &mut Option<i32>,
        b_move_score: &mut score::BitboardMoveScore,
        max_depth: u8,
    ) {
        let window_aspiration = 50;

        if alpha_opt.is_some() && b_move_score.score().value() <= alpha_opt.unwrap()
            || beta_opt.is_some() && b_move_score.score().value() >= beta_opt.unwrap()
        {
            // aspiration window failed
            *b_move_score = self.engine_alphabeta.alphabeta_inc_rec(
                "",
                game,
                0,
                max_depth,
                None,
                None,
                self_actor.clone(),
                None,
                stat_eval,
                transposition_table,
            );
        }
        if let Some(alpha) = alpha_opt {
            *alpha = b_move_score.score().value() - window_aspiration;
        }
        if let Some(beta) = beta_opt {
            *beta = b_move_score.score().value() + window_aspiration;
        }
    }

    fn iddfs_init(
        &self,
        game: &game_state::GameState,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
    ) -> bitboard::BitBoardMove {
        let mut transposition_table = score::TranspositionScore::default();
        let mut stat_eval = stat_eval::StatEval::default();

        let mut game_clone = game.clone();

        let mat_move_opt = self.engine_mat_solver.mat_solver_init(
            game,
            self_actor.clone(),
            stat_actor_opt.clone(),
            self.max_depth,
            &mut stat_eval,
        );
        if let Some(mat_move) = mat_move_opt {
            return *mat_move.bitboard_move();
        }

        let mut b_move_score_opt: Option<score::BitboardMoveScore> = None;
        let mut alpha_opt: Option<i32> = None;
        let mut beta_opt: Option<i32> = None;
        for max_depth in 1..self.max_depth {
            // evaluate move with aplha beta
            let mut b_move_score = self.engine_alphabeta.alphabeta_inc_rec(
                "",
                &mut game_clone,
                0,
                max_depth,
                alpha_opt,
                beta_opt,
                self_actor.clone(),
                None,
                &mut stat_eval,
                &mut transposition_table,
            );
            self.aspiration_window(
                &mut game_clone,
                &mut transposition_table,
                self_actor.clone(),
                &mut stat_eval,
                &mut alpha_opt,
                &mut beta_opt,
                &mut b_move_score,
                max_depth,
            );
            if let Some(stat_actor) = stat_actor_opt.as_ref() {
                let msg = stat_entity::handler_stat::StatUpdate::new(
                    self.id(),
                    stat_eval.n_positions_evaluated(),
                );
                stat_actor.do_send(msg);
            }
            println!(
                "info => {} / '{}' : {}",
                max_depth,
                b_move_score.get_variant(),
                b_move_score.score().value()
            );
            b_move_score_opt = Some(b_move_score);
        }
        b_move_score_opt.unwrap().bitboard_move().clone()
    }
}
unsafe impl Send for EngineIddfs {}

const ALPHABETA_INC_ENGINE_ID_NAME: &str = "Alphabeta incremental engine";
const ALPHABETA_INC_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineIddfs {
    fn id(&self) -> logic::EngineId {
        let name = format!(
            "{} max_depth {} - {}",
            ALPHABETA_INC_ENGINE_ID_NAME.to_owned(),
            self.max_depth,
            self.id_number
        )
        .trim()
        .to_string();
        let author = ALPHABETA_INC_ENGINE_ID_AUTHOR.to_owned();
        logic::EngineId::new(name, author)
    }
    fn find_best_move(
        &self,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        game: game_state::GameState,
    ) {
        // First generate moves
        let moves = logic::gen_moves(game.bit_position());
        if !moves.is_empty() {
            let best_move = self.iddfs_init(&game, self_actor.clone(), stat_actor_opt.clone());
            self_actor.do_send(dispatcher::handler_engine::EngineStopThinking::new(
                stat_actor_opt,
            ));
            let reply = dispatcher::handler_engine::EngineEndOfAnalysis(best_move);
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "Engine of id {:?} reply is: '{:?}'",
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

fn send_best_move(
    self_actor: Addr<dispatcher::EngineDispatcher>,
    best_move: bitboard::BitBoardMove,
) {
    let msg = dispatcher::handler_engine::EngineSendBestMove(best_move);
    self_actor.do_send(msg);
}
