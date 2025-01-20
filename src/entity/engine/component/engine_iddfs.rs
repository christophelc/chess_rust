use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use actix::Addr;

use super::engine_logic::{self as logic, Engine};
use super::evaluation::{score, stat_eval};
use super::{engine_alphabeta, engine_mat, feature, search_state};
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::engine::component::evaluation;
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::game_state;
use crate::entity::stat::actor::stat_entity;
use crate::span_debug;
use crate::{entity::game::component::bitboard, monitoring::debug};

fn span_debug() -> tracing::Span {
    span_debug!("engine::component::iddfs")
}

#[derive(Debug, Clone)]
pub struct EngineIddfs {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
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
            max_depth: max_depth * 2 - 1,
            engine_alphabeta: engine_alphabeta::EngineAlphaBeta::new(
                // fIXME: max_depth here should be dynamic
                None,
                zobrist_table.clone(),
                max_depth,
                false,
            ),
            engine_mat_solver: engine_mat::EngineMat::new(
                // fIXME: max_depth here should be dynamic
                None,
                zobrist_table,
                8,
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
        state: &mut search_state::SearchState,
        is_stop: &Arc<AtomicBool>,
    ) {
        let window_aspiration = evaluation::HALF_PAWN;

        if alpha_opt.is_some() && b_move_score.score().value() <= alpha_opt.unwrap()
            || beta_opt.is_some() && b_move_score.score().value() >= beta_opt.unwrap()
        {
            // aspiration window failed
            *b_move_score = self.engine_alphabeta.alphabeta_inc_rec(
                "",
                game,
                None,
                0,
                max_depth,
                None,
                None,
                self_actor.clone(),
                None,
                stat_eval,
                transposition_table,
                state,
                is_stop,
            );
        }
        if let Some(alpha) = alpha_opt {
            *alpha = b_move_score.score().value() - window_aspiration;
        }
        if let Some(beta) = beta_opt {
            *beta = b_move_score.score().value() + window_aspiration;
        }
    }

    pub fn iddfs_init(
        &self,
        game: &game_state::GameState,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        is_stop: &Arc<AtomicBool>,
    ) -> bitboard::BitBoardMove {
        let span = span_debug();
        let _enter = span.enter();

        tracing::info!("Starting IDDFS with max_depth: {}", self.max_depth);

        let mut transposition_table = score::TranspositionScore::default();
        let mut stat_eval = stat_eval::StatEval::default();
        let mut state = search_state::SearchState::new();

        let mut game_clone = game.clone();

        // Vérifiez si un coup rapide est possible avec le mat solver
        if let Some(mat_move) = if feature::FEATURE_MAT_SOLVER {
            tracing::debug!("Attempting mat solver");
            self.engine_mat_solver.mat_solver_init(
                game,
                self_actor.clone(),
                stat_actor_opt.clone(),
                self.max_depth,
                &mut stat_eval,
                is_stop,
            )
        } else {
            None
        } {
            tracing::info!("Mat solver found a move: {:?}", mat_move.bitboard_move());
            return *mat_move.bitboard_move();
        }

        let mut b_move_score_opt: Option<score::BitboardMoveScore> = None;
        let mut alpha_opt: Option<i32> = None;
        let mut beta_opt: Option<i32> = None;

        tracing::info!("Starting iterative deepening search");
        // Boucle principale
        for max_depth in 1..=self.max_depth {
            if is_stop.load(Ordering::Relaxed) {
                tracing::debug!(
                    "Iddf detected interrupt before evaluation at max_depth {}",
                    max_depth
                );
                break;
            }
            tracing::info!("Starting iteration at depth: {}", max_depth);
            let alphabeta_start = std::time::Instant::now();
            // evaluate move with aplha beta
            let mut b_move_score = self.engine_alphabeta.alphabeta_inc_rec(
                "",
                &mut game_clone,
                None,
                0,
                max_depth,
                alpha_opt,
                beta_opt,
                self_actor.clone(),
                None,
                &mut stat_eval,
                &mut transposition_table,
                &mut state,
                is_stop,
            );
            tracing::info!(
                "Completed alphabeta at depth {} in {:?}, positions evaluated: {}, transpositions: {}",
                max_depth,
                alphabeta_start.elapsed(),
                stat_eval.n_positions_evaluated(),
                stat_eval.n_transposition_hit()
            );
            if feature::FEATURE_ASPIRATION_WINDOW {
                tracing::debug!("Applying aspiration window at depth {}", max_depth);
                let window_start = std::time::Instant::now();
                self.aspiration_window(
                    &mut game_clone,
                    &mut transposition_table,
                    self_actor.clone(),
                    &mut stat_eval,
                    &mut alpha_opt,
                    &mut beta_opt,
                    &mut b_move_score,
                    max_depth,
                    &mut state,
                    is_stop,
                );
                tracing::debug!(
                    "Aspiration window completed in {:?}",
                    window_start.elapsed()
                );
            }
            if let Some(stat_actor) = stat_actor_opt.as_ref() {
                tracing::debug!(
                    "Sending statistics update, positions evaluated: {}",
                    stat_eval.n_positions_evaluated()
                );
                let msg = stat_entity::handler_stat::StatUpdate::new(
                    self.id(),
                    stat_eval.n_positions_evaluated(),
                );
                stat_actor.do_send(msg);
            }
            if is_stop.load(Ordering::Relaxed) {
                tracing::debug!(
                    "Iddf detected interrupt after evaluation of max_depth {}",
                    max_depth
                );
                break;
            }
            //println!("best variant found: {}", b_move_score.get_variant());
            send_best_move(self_actor.clone(), *b_move_score.bitboard_move());
            tracing::info!(
                "Depth {} completed - Variant: {:?}, Score: {}, Move: {:?}",
                max_depth,
                b_move_score.get_variant(),
                b_move_score.score().value(),
                b_move_score.bitboard_move()
            );

            if let Some(prev_score) = b_move_score_opt {
                tracing::debug!(
                    "Score improvement: {} -> {}",
                    prev_score.score().value(),
                    b_move_score.score().value()
                );
            }

            b_move_score_opt = Some(b_move_score);
        }
        if is_stop.load(Ordering::Relaxed) {
            tracing::debug!("IDDFS interrupted.");
            if let Some(b_move_score) = b_move_score_opt.as_ref() {
                tracing::debug!("Best move: {}", *b_move_score);
                send_best_move(self_actor.clone(), *b_move_score.bitboard_move());
            } else {
                tracing::debug!("No best move found");
            }
        } else {
            tracing::debug!("IDDFS completed all iterations");
        }
        b_move_score_opt.map_or_else(
            || {
                tracing::error!("No valid move found in IDDFS!");
                panic!("No valid move found before timeout!")
            },
            |score| {
                tracing::info!("Final selected move: {:?}", score.bitboard_move());
                *score.bitboard_move()
            },
        )
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
        is_stop: &Arc<AtomicBool>,
    ) {
        // First generate moves
        let moves = logic::gen_moves(game.bit_position());
        if moves.is_empty() {
            tracing::warn!("No moves available - game might be finished");
            // FIXME: Do nothing. The engine should be put asleep
            panic!("To be implemented. When EndGame detected in game_manager, stop the engines");
        }
        //tracing::info!(max_time = max_time.as_secs());
        let best_move = self.iddfs_init(&game, self_actor.clone(), stat_actor_opt.clone(), is_stop);
        tracing::debug!("Send EngineStopThinking");
        self_actor.do_send(dispatcher::handler_engine::EngineStopThinking::new(
            stat_actor_opt,
        ));
        tracing::debug!("Send EngineEndOfAnalysis");
        let reply = dispatcher::handler_engine::EngineEndOfAnalysis(best_move);
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "Engine of id {:?} reply is: '{:?}'",
                self.id(),
                reply
            )));
        }
        self_actor.do_send(reply);
    }
}

fn send_best_move(
    self_actor: Addr<dispatcher::EngineDispatcher>,
    best_move: bitboard::BitBoardMove,
) {
    let msg = dispatcher::handler_engine::EngineSendBestMove(best_move);
    self_actor.do_send(msg);
}
