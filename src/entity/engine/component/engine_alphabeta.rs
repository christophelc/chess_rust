use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use actix::Addr;

use super::engine_logic::{self as logic, Engine};
use super::evaluation::{self, score, stat_eval};
use super::{feature, search_state};
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::engine::component::engine_mat;
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::game_state;
use crate::entity::game::component::square::Switch;
use crate::entity::stat::actor::stat_entity;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

#[derive(Debug)]
pub struct EngineAlphaBeta {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
    zobrist_table: zobrist::Zobrist,
    max_depth: u8,
    engine_mat_solver: engine_mat::EngineMat,
    is_send_best_move: bool,
}
impl EngineAlphaBeta {
    pub fn new(
        debug_actor_opt: Option<debug::DebugActor>,
        zobrist_table: zobrist::Zobrist,
        max_depth: u8,
        is_send_best_move: bool,
    ) -> Self {
        assert!(max_depth >= 1 && max_depth <= search_state::MAX_DEPTH as u8);
        Self {
            id_number: "".to_string(),
            debug_actor_opt,
            zobrist_table: zobrist_table.clone(),
            max_depth: max_depth * 2 - 1,
            engine_mat_solver: engine_mat::EngineMat::new(
                // fIXME: max_depth here should be dynamic
                None,
                zobrist_table,
                max_depth,
            ),
            is_send_best_move,
        }
    }
    pub fn set_id_number(&mut self, id_number: &str) {
        self.id_number = id_number.to_string();
    }

    fn set_preorder(
        m: &bitboard::BitBoardMove,
        last_move_opt: Option<&bitboard::BitBoardMove>,
        is_check: bool,
        is_killer_move: bool,
    ) -> score::PreOrder {
        if let Some(promotion) = m.promotion() {
            return score::PreOrder::Promotion(promotion);
        }
        let preorder = match (is_killer_move, is_check, m.capture().is_some()) {
            (true, _, _) => score::PreOrder::KillerMove,
            (_, true, _) => score::PreOrder::new_mat(m.color().switch()),
            (_, _, true) => {
                let mut delta = score::biased_capture(m.type_piece(), m.capture());
                if let Some(last_move) = last_move_opt {
                    // if capture of the last moved piece, priorize
                    if m.end() == last_move.end() {
                        delta = delta * 2 + 1;
                    };
                }
                score::PreOrder::Capture { delta }
            }
            _ => score::PreOrder::Depth,
        };
        preorder
    }

    // is_asc true => score 3, score 4
    // is asc false => score 4, score 3
    fn get_moves_preordered(
        &self,
        moves: &mut [bitboard::BitBoardMove],
        last_move_opt: Option<&bitboard::BitBoardMove>,
        game: &mut game_state::GameState,
        transposition_table: &mut score::TranspositionScore,
        is_asc: bool,
        current_depth: u8,
        state: &search_state::SearchState,
    ) -> Vec<(score::MoveStatus, score::PreOrder)> {
        if !feature::FEATURE_PREORDER {
            return moves
                .into_iter()
                .map(|m| (score::MoveStatus::from_move(*m), score::PreOrder::Depth))
                .collect();
        }
        let mut moves_status_with_preorder: Vec<(score::MoveStatus, score::PreOrder)> = vec![];
        for m in moves {
            let long_algebric_move = long_notation::LongAlgebricNotationMove::build_from_b_move(*m);
            //println!("playing level 0: {}", long_algebraic_move.cast());
            game.play_moves(&[long_algebric_move], &self.zobrist_table, None, false)
                .unwrap();
            game.update_endgame_status();
            let move_info_opt = transposition_table.get_move_info(&game.last_hash(), 0);
            let preorder = match move_info_opt.map(|move_info| move_info.move_score().clone()) {
                Some(b_move_score) if b_move_score.score().current_depth() == current_depth => {
                    score::PreOrder::CurrentDepthScore(b_move_score.score().clone())
                }
                None => Self::set_preorder(
                    m,
                    last_move_opt,
                    game.check_status().is_check(),
                    state.is_killer_move(current_depth as usize, *m),
                ),
                Some(b_move_score) => {
                    score::PreOrder::PreviousDepthScore(b_move_score.score().clone())
                }
            };
            let move_status = score::MoveStatus::from_move(*m);
            moves_status_with_preorder.push((move_status, preorder));
            game.play_back()
        }
        moves_status_with_preorder.sort_by(|a, b| score::preorder_compare(&a.1, &b.1, is_asc));
        moves_status_with_preorder.into_iter().collect()
    }

    fn alphabeta(
        &self,
        game: &game_state::GameState,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        is_stop: &Arc<AtomicBool>,
    ) -> bitboard::BitBoardMove {
        //let num_cpus = num_cpus::get();
        let mut transposition_table = score::TranspositionScore::default();
        let mut state = search_state::SearchState::new();

        let current_depth = 0;
        let mut stat_eval = stat_eval::StatEval::default();

        let mut game_clone = game.clone();

        let mat_move_opt = self.engine_mat_solver.mat_solver_init(
            game,
            self_actor.clone(),
            stat_actor_opt.clone(),
            self.max_depth,
            &mut stat_eval,
            is_stop,
        );
        if let Some(mat_move) = mat_move_opt {
            return *mat_move.bitboard_move();
        }

        let b_move_score = self.alphabeta_inc_rec(
            "",
            &mut game_clone,
            None,
            current_depth,
            self.max_depth,
            None,
            None,
            self_actor.clone(),
            stat_actor_opt.clone(),
            &mut stat_eval,
            &mut transposition_table,
            &mut state,
            is_stop,
        );

        *b_move_score.bitboard_move()
    }

    fn diff_opt(beta_opt: Option<i32>, alpha_opt: Option<i32>) -> Option<i32> {
        beta_opt.zip(alpha_opt).map(|(beta, alpha)| beta - alpha)
    }
    fn can_null_move(
        game: &game_state::GameState,
        current_depth: u8,
        max_depth: u8,
        m: &bitboard::BitBoardMove,
        alpha_opt: Option<i32>,
        beta_opt: Option<i32>,
        is_max: bool,
    ) -> bool {
        feature::FEATURE_NULL_MOVE_PRUNING
            && m.capture().is_none()
            && is_max
            && beta_opt.is_some()
            && !is_max
            && alpha_opt.is_some()
            && Self::diff_opt(beta_opt, alpha_opt).unwrap_or(0) >= evaluation::HALF_PAWN
            && current_depth >= 2
            && max_depth - current_depth > 3
            && !game.check_status().is_check()
            && !evaluation::is_final(&game)
    }

    pub fn alphabeta_inc_rec(
        &self,
        variant: &str,
        game: &mut game_state::GameState,
        last_move_opt: Option<&bitboard::BitBoardMove>,
        current_depth: u8,
        max_depth: u8,
        alpha_opt: Option<i32>,
        beta_opt: Option<i32>,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        stat_eval: &mut stat_eval::StatEval,
        transposition_table: &mut score::TranspositionScore,
        state: &mut search_state::SearchState,
        is_stop: &Arc<AtomicBool>,
    ) -> score::BitboardMoveScore {
        let mut alpha_opt = alpha_opt;
        let mut beta_opt = beta_opt;
        let mut best_move_score_opt: Option<score::BitboardMoveScore> = None;

        let mut moves = game.gen_moves();
        if is_stop.load(Ordering::Relaxed) {
            tracing::debug!("Interrupt alphabeta for current_depth: {}", current_depth);
            let mv = score::BitboardMoveScore::new(
                *moves.first().unwrap(),
                score::Score::new(0, 0, 0),
                "".to_string(),
            );
            return mv;
        }
        let is_max = game
            .bit_position()
            .bit_position_status()
            .player_turn_white();
        let moves_status = self.get_moves_preordered(
            &mut moves,
            last_move_opt,
            game,
            transposition_table,
            !is_max,
            current_depth,
            state,
        );
        //let v: Vec<String> = moves_status.iter().map(|pm| format!("{} : {:?}", LongAlgebricNotationMove::build_from_b_move(*pm.0.get_move()).cast(), pm.1)).collect();
        //println!("variant: '{}' preorder: {:?}", variant, v);

        // alpha beta
        for (idx, (m_status, preorder)) in moves_status.iter().enumerate() {
            if idx > 1 && is_stop.load(Ordering::Relaxed) {
                tracing::debug!("Loop interrupt alphabeta for current_depth: {}", current_depth);
                break;
            }
    
            let long_algebraic_move =
                long_notation::LongAlgebricNotationMove::build_from_b_move(*m_status.get_move());
            let updated_variant = format!("{} {}", variant, long_algebraic_move.cast());
            let updated_variant = updated_variant.trim();
            // Last move reduction (if we are not too close to max_depth)
            let max_depth = if feature::FEATURE_LMR
                && current_depth > 3
                && max_depth - current_depth > 0
                && idx > 2
                && !preorder.is_special()
                && !game.check_status().is_check()
            {
                max_depth - 1
            } else {
                max_depth
            };
            let score = self.process_move(
                game,
                *m_status.get_move(),
                alpha_opt,
                beta_opt,
                updated_variant,
                self_actor.clone(),
                stat_actor_opt.clone(),
                stat_eval,
                current_depth,
                max_depth,
                is_max,
                transposition_table,
                state,
                is_stop,
            );
            let mut move_score = score::BitboardMoveScore::new(
                *m_status.get_move(),
                score::Score::new(score.value(), score.current_depth(), score.max_depth()),
                m_status.get_variant(),
            );
            move_score.set_variant(&updated_variant);
            //println!("{} : {}", updated_variant, move_score.score());
            if is_max {
                // best_score = max(best_score, score)
                if best_move_score_opt.is_none()
                    || score.is_greater_than(best_move_score_opt.as_ref().unwrap().score())
                {
                    // Send best move
                    best_move_score_opt = Some(move_score);
                    //if current_depth == 0 { println!("{} -> best move: {}", current_depth, best_move_score_opt.as_ref().unwrap()) };
                    if current_depth == 0 {
                        send_best_move(
                            self_actor.clone(),
                            *best_move_score_opt.as_ref().unwrap().bitboard_move(),
                        );
                    }
                }
                // alpha = max(alpha, score)
                if alpha_opt.is_none()
                    || best_move_score_opt.as_ref().unwrap().score().value() > alpha_opt.unwrap()
                {
                    alpha_opt = Some(best_move_score_opt.as_ref().unwrap().score().value());
                    //println!("alpha: '{}' {}", variant, alpha_opt.as_ref().unwrap());
                    // TODO: store alpha
                }
                // beta pruning (alpha >= beta(parent) )
                if *alpha_opt.as_ref().unwrap() == score::SCORE_MAT_WHITE
                    || beta_opt.is_some()
                        && alpha_opt.as_ref().unwrap() >= beta_opt.as_ref().unwrap()
                {
                    if feature::FEATURE_KILLER_MOVE {
                        state.add_killer_move(
                            current_depth as usize,
                            best_move_score_opt
                                .as_ref()
                                .unwrap()
                                .bitboard_move()
                                .clone(),
                        );
                    }
                    // do not update transpositon table
                    return best_move_score_opt.unwrap();
                }
            } else {
                // best_score = min(best_score, score)
                if best_move_score_opt.is_none()
                    || score.is_less_than(best_move_score_opt.as_ref().unwrap().score())
                {
                    // best_score = min(best_score, score)
                    best_move_score_opt = Some(move_score);
                    //if current_depth == 0 { println!("{} -> best move: {}", current_depth, best_move_score_opt.as_ref().unwrap()); }
                    if current_depth == 0 && self.is_send_best_move {
                        send_best_move(
                            self_actor.clone(),
                            *best_move_score_opt.as_ref().unwrap().bitboard_move(),
                        );
                    }
                }
                // beta = min(beta, score)
                if beta_opt.is_none()
                    || best_move_score_opt.as_ref().unwrap().score().value() < beta_opt.unwrap()
                {
                    beta_opt = Some(best_move_score_opt.as_ref().unwrap().score().value());
                    //println!("beta: '{}' {}", variant, beta_opt.as_ref().unwrap());
                    // TODO: store beta
                }
                // alpha pruning (alpha(parent) >= beta)
                if *beta_opt.as_ref().unwrap() == score::SCORE_MAT_BLACK
                    || alpha_opt.is_some()
                        && alpha_opt.as_ref().unwrap() >= beta_opt.as_ref().unwrap()
                {
                    state.add_killer_move(
                        current_depth as usize,
                        best_move_score_opt
                            .as_ref()
                            .unwrap()
                            .bitboard_move()
                            .clone(),
                    );
                    // do not update transpositon table
                    return best_move_score_opt.unwrap();
                }
            }
            /*/
            println!(
                "{} / [{:?}, {:?}]; best: {}",
                updated_variant,
                alpha_opt,
                beta_opt,
                best_move_score_opt.as_ref().unwrap()
            );
            */
        }
        let hash = game.last_hash();
        // FIXME: sometimes, the value is overriden for the same hash, current_depth, max_depth (for a specific depth defined in iddfs).
        // Chekc if this is normal
        if feature::FEATURE_TRANSPOSITION_TABLE {
            transposition_table.set_move_info(
                &hash,
                best_move_score_opt.as_ref().unwrap(),
                score::BoundScore::Exact,
                game.bit_position().bit_position_status().n_half_moves(),
            );
        }

        best_move_score_opt.unwrap()
    }

    fn goal_is_reached(is_max_depth: bool, end_game: game_state::EndGame) -> bool {
        if end_game == game_state::EndGame::None {
            is_max_depth
        } else {
            true
        }
    }

    // return None if no move in the scope of the goal
    fn process_move(
        &self,
        game: &mut game_state::GameState,
        m: bitboard::BitBoardMove,
        alpha_opt: Option<i32>,
        beta_opt: Option<i32>,
        variant: &str,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        stat_eval: &mut stat_eval::StatEval,
        current_depth: u8,
        max_depth: u8,
        is_max: bool,
        transposition_table: &mut score::TranspositionScore,
        state: &mut search_state::SearchState,
        is_stop: &Arc<AtomicBool>,
    ) -> score::Score {
        let long_algebraic_move = long_notation::LongAlgebricNotationMove::build_from_b_move(m);
        // if current_depth >= 0 {
        //    println!("{}", variant);
        // }
        game.play_moves(&[long_algebraic_move], &self.zobrist_table, None, false)
            .unwrap();
        // check if the current position has been already evaluated
        let hash = game.last_hash();
        if let Some(move_info) = transposition_table.get_move_info(&hash, max_depth - current_depth)
        {
            //println!("hit {:?}", move_score);
            if stat_eval.inc_n_transposition_hit() % 1_000_000 == 0 {
                tracing::debug!("hits: {}", stat_eval.n_transposition_hit());
            }
            game.play_back();
            //println!("transposition {}: {} / {} =>  {}: {}", long_algebraic_move.cast(), current_depth, max_depth, move_score.get_variant(), move_score.score());
            return *move_info.move_score().score();
        };

        game.update_endgame_status();
        let score = if game.end_game() == game_state::EndGame::None {
            if !Self::goal_is_reached(current_depth >= max_depth, game.end_game()) {
                //println!("Rec analysis of: {} - {} {} {:?}", variant, current_depth, max_depth, m.capture());
                // null move pruning
                if Self::can_null_move(
                    game,
                    current_depth,
                    max_depth,
                    &m,
                    alpha_opt,
                    beta_opt,
                    is_max,
                ) {
                    // not optimized. By computing first attackers, we will eliminate the need to play a null move first and check if it is valid
                    game.play_null_move(&self.zobrist_table);
                    if game.can_move() {
                        let reduction = 2 + (max_depth - current_depth) / 6;
                        let null_depth = max_depth - reduction;
                        //println!("null move: {} - reduction: {}, current_depth: {}, max_depth: {}", variant, reduction, current_depth, max_depth);
                        let score_after_null_move = self.alphabeta_inc_rec(
                            variant,
                            game,
                            None,
                            current_depth + 1,
                            null_depth,
                            alpha_opt,
                            beta_opt,
                            self_actor.clone(),
                            stat_actor_opt.clone(),
                            stat_eval,
                            transposition_table,
                            state,
                            is_stop,
                        );
                        //println!("end null move {}", variant);
                        game.play_back_null_move();
                        if is_max && score_after_null_move.score().value() >= beta_opt.unwrap() {
                            game.play_back();
                            return score::Score::new(beta_opt.unwrap(), current_depth, max_depth);
                        } else if !is_max
                            && score_after_null_move.score().value() <= alpha_opt.unwrap()
                        {
                            game.play_back();
                            return score::Score::new(alpha_opt.unwrap(), current_depth, max_depth);
                        }
                    } else {
                        game.play_back_null_move();
                    }
                }

                let best_move_score = self.alphabeta_inc_rec(
                    variant,
                    game,
                    Some(&m),
                    current_depth + 1,
                    max_depth,
                    alpha_opt,
                    beta_opt,
                    self_actor.clone(),
                    stat_actor_opt.clone(),
                    stat_eval,
                    transposition_table,
                    state,
                    is_stop,
                );
                let score = best_move_score.score();
                score::Score::new(score.value(), current_depth, max_depth)
            } else {
                //println!("Analysis of: {} - {} {} {:?}", variant, current_depth, max_depth, m.capture());
                // capture (avoid horizon effect) ?
                let mut score_opt: Option<score::Score> = None;
                if feature::FEATURE_CAPTURE_HORIZON
                    && current_depth == max_depth
                    && m.capture().is_some()
                {
                    if let Some(score) = self.evalutate_capture(
                        variant,
                        game,
                        current_depth,
                        max_depth,
                        self_actor.clone(),
                        stat_actor_opt.clone(),
                        stat_eval,
                        m.end(),
                        !is_max,
                    ) {
                        score_opt = Some(score);
                    }
                }
                if feature::FEATURE_CAPTURE_HORIZON
                    && current_depth == max_depth
                    && game.check_status().is_check()
                {
                    score_opt = Some(
                        *self
                            .alphabeta_inc_rec(
                                variant,
                                game,
                                Some(&m),
                                current_depth + 1,
                                max_depth + 1,
                                alpha_opt,
                                beta_opt,
                                self_actor.clone(),
                                stat_actor_opt.clone(),
                                stat_eval,
                                transposition_table,
                                state,
                                is_stop,
                            )
                            .score(),
                    );
                }
                let score = if let Some(score) = score_opt {
                    score
                } else {
                    score::Score::new(
                        evaluation::evaluate_position(game, stat_eval, &stat_actor_opt, self.id()),
                        current_depth,
                        max_depth,
                    )
                };
                //println!("{}/{} {}\t{}", current_depth, max_depth, variant, score);
                transposition_table.set_move_info(
                    &hash,
                    &score::BitboardMoveScore::new(m, score, variant.to_string()),
                    score::BoundScore::Exact,
                    game.bit_position().bit_position_status().n_half_moves(),
                );
                score
            }
        } else {
            evaluation::handle_end_game_scenario(game, current_depth, max_depth)
        };
        game.play_back();
        score
    }

    fn evalutate_capture(
        &self,
        variant: &str,
        game: &mut game_state::GameState,
        current_depth: u8,
        max_depth: u8,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        stat_eval: &mut stat_eval::StatEval,
        square_capture: bitboard::BitIndex,
        is_max: bool,
    ) -> Option<score::Score> {
        let moves = game.gen_moves();
        let mut moves_status: Vec<_> = moves
            .into_iter()
            .filter(|m| m.capture().is_some() && m.end() == square_capture)
            .map(score::MoveStatus::from_move)
            .collect();
        let mut best_score_opt: Option<score::Score> = None;
        // TODO before start: sort capture moves
        for m_status in moves_status.iter_mut() {
            let long_algebraic_move =
                long_notation::LongAlgebricNotationMove::build_from_b_move(*m_status.get_move());
            let updated_variant = format!("{} {}", variant, long_algebraic_move.cast());
            //println!("capture {}", updated_variant);
            game.play_moves(&[long_algebraic_move], &self.zobrist_table, None, false)
                .unwrap();
            game.update_endgame_status();
            let score_opt = self.evalutate_capture(
                &updated_variant,
                game,
                current_depth + 1,
                max_depth + 1,
                self_actor.clone(),
                stat_actor_opt.clone(),
                stat_eval,
                square_capture,
                !is_max,
            );
            let score = if let Some(sc) = &score_opt {
                *sc
            } else {
                score::Score::new(
                    evaluation::evaluate_position(game, stat_eval, &stat_actor_opt, self.id()),
                    current_depth,
                    max_depth,
                )
            };
            //println!("capture {}: {}", updated_variant, score);
            match &best_score_opt {
                Some(best_score) => {
                    if is_max && score.value() > best_score.value()
                        || !is_max && score.value() < best_score.value()
                    {
                        best_score_opt = score_opt;
                    }
                }
                None => best_score_opt = Some(score),
            }
            game.play_back();
        }
        best_score_opt
    }
}
unsafe impl Send for EngineAlphaBeta {}

const ALPHABETA_ENGINE_ID_NAME: &str = "Alphabeta engine";
const ALPHABETA_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineAlphaBeta {
    fn id(&self) -> logic::EngineId {
        let name = format!(
            "{} max_depth {} - {}",
            ALPHABETA_ENGINE_ID_NAME.to_owned(),
            self.max_depth,
            self.id_number
        )
        .trim()
        .to_string();
        let author = ALPHABETA_ENGINE_ID_AUTHOR.to_owned();
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
        if !moves.is_empty() {
            let best_move = self.alphabeta(&game, self_actor.clone(), stat_actor_opt.clone(), is_stop);
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
