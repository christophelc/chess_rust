use actix::Addr;

use super::engine_logic::{self as logic, Engine};
use super::{score, stat_eval};
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::engine::component::engine_mat;
use crate::entity::game::component::bitboard::{piece_move, zobrist};
use crate::entity::game::component::square::Switch;
use crate::entity::game::component::{game_state, square};
use crate::entity::stat::actor::stat_entity;
use crate::entity::stat::component::stat_data;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

#[derive(Debug)]
pub struct EngineAlphaBeta {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
    zobrist_table: zobrist::Zobrist,
    max_depth: u8,
    engine_mat_solver: engine_mat::EngineMat,
}
impl EngineAlphaBeta {
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
            engine_mat_solver: engine_mat::EngineMat::new(
                // fIXME: max_depth here should be dynamic
                None,
                zobrist_table,
                max_depth,
            ),
        }
    }
    pub fn set_id_number(&mut self, id_number: &str) {
        self.id_number = id_number.to_string();
    }

    fn set_preorder(m: &bitboard::BitBoardMove, is_check: bool) -> score::PreOrder {
        if let Some(promotion) = m.promotion() {
            return score::PreOrder::Promotion(promotion);
        }
        let mut preorder = score::PreOrder::Depth;
        if is_check {
            // we want the opponent to be check and mat
            preorder = score::PreOrder::new_mat(m.color().switch());
        } else if m.capture().is_some() {
            let delta = score::biased_capture(m.type_piece(), m.capture());
            preorder = score::PreOrder::Capture { delta };
        }
        preorder
    }

    // is_asc true => score 3, score 4
    // is asc false => score 4, score 3
    fn get_moves_preordered(
        &self,
        moves: &mut [bitboard::BitBoardMove],
        game: &mut game_state::GameState,
        transposition_table: &mut score::TranspositionScore,
        is_asc: bool,
    ) -> Vec<score::MoveStatus> {
        let mut moves_status_with_preorder: Vec<(score::MoveStatus, score::PreOrder)> = vec![];
        for m in moves {
            let long_algebraic_move =
                long_notation::LongAlgebricNotationMove::build_from_b_move(*m);
            //println!("playing level 0: {}", long_algebraic_move.cast());
            game.play_moves(&[long_algebraic_move], &self.zobrist_table, None, false)
                .unwrap();
            game.update_endgame_status();
            let preorder = match transposition_table.get_move_score(&game.last_hash(), 0) {
                None => Self::set_preorder(m, game.check_status().is_check()),
                Some(b_move_score) => score::PreOrder::PreviousScore(b_move_score.score().clone()),
            };
            let move_status = score::MoveStatus::from_move(*m);
            moves_status_with_preorder.push((move_status, preorder));
            game.play_back()
        }
        moves_status_with_preorder.sort_by(|a, b| score::preorder_compare(&a.1, &b.1, is_asc));
        moves_status_with_preorder
            .into_iter()
            .map(|t| t.0)
            .collect()
    }

    fn alphabeta(
        &self,
        game: &game_state::GameState,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
    ) -> bitboard::BitBoardMove {
        //let num_cpus = num_cpus::get();
        let mut transposition_table = score::TranspositionScore::default();
        let current_depth = 0;
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

        let b_move_score = self.alphabeta_inc_rec(
            "",
            &mut game_clone,
            current_depth,
            self.max_depth,
            None,
            None,
            self_actor.clone(),
            stat_actor_opt.clone(),
            &mut stat_eval,
            &mut transposition_table,
        );

        *b_move_score.bitboard_move()
    }

    pub fn alphabeta_inc_rec(
        &self,
        variant: &str,
        game: &mut game_state::GameState,
        current_depth: u8,
        max_depth: u8,
        alpha_opt: Option<i32>,
        beta_opt: Option<i32>,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        stat_eval: &mut stat_eval::StatEval,
        transposition_table: &mut score::TranspositionScore,
    ) -> score::BitboardMoveScore {
        let mut alpha_opt = alpha_opt;
        let mut beta_opt = beta_opt;
        let mut best_move_score_opt: Option<score::BitboardMoveScore> = None;

        let mut moves = game.gen_moves();
        let is_max = game
            .bit_position()
            .bit_position_status()
            .player_turn_white();
        let moves_status =
            self.get_moves_preordered(&mut moves, game, transposition_table, !is_max);

        for m_status in &moves_status {
            let long_algebraic_move =
                long_notation::LongAlgebricNotationMove::build_from_b_move(*m_status.get_move());
            let updated_variant = format!("{} {}", variant, long_algebraic_move.cast());
            let score = self.process_move(
                game,
                *m_status.get_move(),
                alpha_opt,
                beta_opt,
                &updated_variant,
                self_actor.clone(),
                stat_actor_opt.clone(),
                stat_eval,
                current_depth,
                max_depth,
                is_max,
                transposition_table,
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
                    if current_depth == 0 {
                        send_best_move(
                            self_actor.clone(),
                            *best_move_score_opt.as_ref().unwrap().bitboard_move(),
                        );
                    }
                }
                // alpha = max(alpha, score)
                if alpha_opt.is_none()
                    || best_move_score_opt.as_ref().unwrap().score().value() >= alpha_opt.unwrap()
                {
                    alpha_opt = Some(best_move_score_opt.as_ref().unwrap().score().value());
                }
                // beta pruning (alphz >= beta)
                if *alpha_opt.as_ref().unwrap() == i32::MAX
                    || beta_opt.is_some()
                        && alpha_opt.as_ref().unwrap() >= beta_opt.as_ref().unwrap()
                {
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
                    if current_depth == 0 {
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
                }
                // alpha pruning (alpha >= beta)
                if *beta_opt.as_ref().unwrap() == i32::MIN
                    || alpha_opt.is_some()
                        && alpha_opt.as_ref().unwrap() >= beta_opt.as_ref().unwrap()
                {
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
        transposition_table.set_move_score(&hash, best_move_score_opt.as_ref().unwrap());

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
    ) -> score::Score {
        let long_algebraic_move = long_notation::LongAlgebricNotationMove::build_from_b_move(m);
        game.play_moves(&[long_algebraic_move], &self.zobrist_table, None, false)
            .unwrap();
        // if current_depth >= 0 && max_depth >= 5 {
        //     println!("{}", variant);
        // }
        // check if the current position has been already evaluated
        let hash = game.last_hash();
        if let Some(move_score) =
            transposition_table.get_move_score(&hash, max_depth - current_depth)
        {
            //println!("hit {:?}", move_score);
            if stat_eval.inc_n_transposition_hit() % 1_000_000 == 0 {
                println!("hits: {}", stat_eval.n_transposition_hit());
            }
            game.play_back();
            //println!("transposition {}: {} / {} =>  {}: {}", long_algebraic_move.cast(), current_depth, max_depth, move_score.get_variant(), move_score.score());
            return move_score.score().clone();
        };

        game.update_endgame_status();
        let score = if game.end_game() == game_state::EndGame::None {
            if !Self::goal_is_reached(current_depth >= max_depth, game.end_game()) {
                let best_move_score = self.alphabeta_inc_rec(
                    variant,
                    game,
                    current_depth + 1,
                    max_depth,
                    alpha_opt,
                    beta_opt,
                    self_actor.clone(),
                    stat_actor_opt.clone(),
                    stat_eval,
                    transposition_table,
                );
                let score = best_move_score.score();
                score::Score::new(score.value(), current_depth, max_depth)
            } else {
                // capture (avoid horizon effect) ?
                let mut score_opt: Option<score::Score> = None;
                if current_depth == max_depth && m.capture().is_some() {
                    if let Some(score) = self.evalutate_capture(
                        variant,
                        game,
                        current_depth,
                        max_depth,
                        self_actor.clone(),
                        stat_actor_opt.clone(),
                        stat_eval,
                        m.end(),
                        is_max,
                    ) {
                        score_opt = Some(score);
                    }
                }
                if let Some(score) = score_opt {
                    let updated_variant = format!("{} {}", variant, long_algebraic_move.cast());
                    transposition_table.set_move_score(
                        &hash,
                        &score::BitboardMoveScore::new(m, score.clone(), updated_variant),
                    );
                    score
                } else {
                    score::Score::new(
                        evaluate_position(game, stat_eval, &stat_actor_opt, self.id()),
                        current_depth,
                        max_depth,
                    )
                }
            }
        } else {
            handle_end_game_scenario(game, current_depth, max_depth)
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
                sc.clone()
            } else {
                score::Score::new(
                    evaluate_position(game, stat_eval, &stat_actor_opt, self.id()),
                    current_depth,
                    max_depth,
                )
            };
            //println!("capture {} : {}", updated_variant, score.value());
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
    ) {
        // First generate moves
        let moves = logic::gen_moves(game.bit_position());
        if !moves.is_empty() {
            let best_move = self.alphabeta(&game, self_actor.clone(), stat_actor_opt.clone());
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

fn handle_end_game_scenario(
    game: &game_state::GameState,
    current_depth: u8,
    max_depth: u8,
) -> score::Score {
    // check the color for the last move
    let is_turn_white = !game
        .bit_position()
        .bit_position_status()
        .player_turn_white();
    match game.end_game() {
        game_state::EndGame::Mat(_) => {
            // If the game ends in a checkmate, it is a favorable outcome for the player who causes the checkmate.
            if is_turn_white {
                score::Score::new(i32::MAX, current_depth, max_depth)
            } else {
                score::Score::new(i32::MIN, current_depth, max_depth)
            }
        }
        game_state::EndGame::TimeOutLost(square::Color::White) => {
            // If the current player loses by timeout, it is an unfavorable outcome.
            score::Score::new(i32::MIN, current_depth, max_depth)
        }
        game_state::EndGame::TimeOutLost(_) => {
            // If the opponent times out, it is a favorable outcome for the current player.
            score::Score::new(i32::MAX, current_depth, max_depth)
        }
        _ => {
            // In other cases (stalemate, etc.), it might be neutral or need specific scoring based on the game rules.
            score::Score::new(0, current_depth, max_depth)
        }
    }
}

fn evaluate_position(
    game: &game_state::GameState,
    stat_eval: &mut stat_eval::StatEval,
    stat_actor_opt: &Option<stat_entity::StatActor>,
    engine_id: logic::EngineId,
) -> i32 {
    if stat_eval.inc_n_positions_evaluated() % stat_data::SEND_STAT_EVERY_N_POSITION_EVALUATED == 0
    {
        if let Some(stat_actor) = stat_actor_opt {
            let msg = stat_entity::handler_stat::StatUpdate::new(
                engine_id,
                stat_eval.n_positions_evaluated(),
            );
            stat_actor.do_send(msg);
        }
        stat_eval.reset_n_positions_evaluated();
    }
    // check if can win or insufficient material
    let player_turn = game.bit_position().bit_position_status().player_turn();
    let player_can_win = check_can_win(
        game.bit_position()
        .bit_boards_white_and_black()
        .bit_board(&player_turn)
    );
    let player_opponent_can_win = check_can_win(
        game.bit_position()
        .bit_boards_white_and_black()
        .bit_board(&player_turn.switch())
    );    
    let default_score = evaluate_static_position(game.bit_position()) 
        + evaluate_dynamic_position(game.gen_control_square());
    let bonus = if player_turn == square::Color::White { 100000 } else { -100000 };
    match (player_can_win, player_opponent_can_win) {
        // both can win
        (true, true) => default_score,
        // no one can win
        (false, false) => 0,
        // only current player can win
        (true, false) => default_score + bonus,
        // only opponent can win
        (false, true) => default_score - bonus,
    }

}

fn check_can_win(bitboards: &bitboard::BitBoards) -> bool {
    let (n_rooks, n_knights, n_bishops, n_queens, n_pawns) = count_material_one_side(bitboards);
    n_rooks != 0 || n_queens != 0 || n_pawns != 0 || (n_knights + n_bishops >= 2)
}

fn evaluate_dynamic_position(
    control_squares: (piece_move::ControlSquares, piece_move::ControlSquares),
) -> i32 {
    let (control_squares_white, control_squares_black) = control_squares;
    (evaluate_dynamic_position_one_side(control_squares_white)
        - evaluate_dynamic_position_one_side(control_squares_black))
        * 10
}

fn evaluate_dynamic_position_one_side(control_squares: piece_move::ControlSquares) -> i32 {
    let mut score = 0;
    for piece_moves in control_squares.moves() {
        let n_squares_control_except_pawns = piece_moves.moves().count_ones();
        let n_sqaures_control_pawns = control_squares.panws_control().count_ones();
        score += (n_squares_control_except_pawns + n_sqaures_control_pawns) as i32;
    }
    score
}

fn count_material_one_side(bitboards: &bitboard::BitBoards) -> (u32, u32, u32, u32, u32) {
    let n_rooks = bitboards.rooks().bitboard().iter().count() as u32;
    let n_knights = bitboards.knights().bitboard().iter().count() as u32;
    let n_bishops = bitboards.bishops().bitboard().iter().count() as u32;
    let n_queens = bitboards.queens().bitboard().iter().count() as u32;
    let n_pawns = bitboards.pawns().bitboard().iter().count() as u32;
    (n_rooks, n_knights, n_bishops, n_queens, n_pawns)
}

fn evaluate_static_position_one_side(bitboards: &bitboard::BitBoards) -> u32 {
    let (n_rooks, n_knights, n_bishops, n_queens, n_pawns) = count_material_one_side(bitboards);
    n_rooks * 5 + n_knights * 3 + n_bishops * 3 + n_queens * 10 + n_pawns
}

// evaluate from white perspective
fn evaluate_static_position(bit_position: &bitboard::BitPosition) -> i32 {
    let score_current = evaluate_static_position_one_side(
        bit_position.bit_boards_white_and_black().bit_board_white(),
    );
    let score_opponent = evaluate_static_position_one_side(
        bit_position.bit_boards_white_and_black().bit_board_black(),
    );
    // println!("{}", bit_position.to().chessboard());
    // println!("{:?} / {:?}", score_current, score_opponent);
    (score_current as i32 - score_opponent as i32) * 100
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix::Actor;

    use crate::entity::engine::actor::engine_dispatcher as dispatcher;
    use crate::{
        entity::{
            engine::component::engine_alphabeta,
            game::{
                actor::game_manager,
                component::{bitboard, player, square::TypePiece},
            },
            uci::actor::uci_entity,
        },
        monitoring::debug,
        ui::notation::long_notation,
    };

    use super::evaluate_static_position_one_side;

    #[test]
    fn test_evaluation_one_side() {
        let mut bitboards = bitboard::BitBoards::default();
        bitboards.xor_piece(TypePiece::Rook, bitboard::BitBoard::new(1));
        bitboards.xor_piece(TypePiece::Pawn, bitboard::BitBoard::new(2));
        let score = evaluate_static_position_one_side(&bitboards);
        assert_eq!(score, 6);
    }

    use crate::entity::game::component::game_state;
    #[cfg(test)]
    async fn get_game_state(
        game_manager_actor: &game_manager::GameManagerActor,
    ) -> Option<game_state::GameState> {
        let result_or_error = game_manager_actor
            .send(game_manager::handler_game::GetGameState)
            .await;
        result_or_error.unwrap()
    }

    // FIXME: remove sleep
    #[ignore]
    #[actix::test]
    async fn test_game_end() {
        const ALPHABETA_DEPTH: u8 = 2;

        //let debug_actor_opt: Option<debug::DebugActor> = None;
        let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 b8a6 f1a6 b7a6 d2d4 d7d5 e4e5 c7c6 g1f3 a8b8 e1g1 c8g4 d1d3 b8b4 c2c3 b4a4 b2b3 a4a5 c1d2 g4f3 g2f3 a5b5 c3c4 b5b7 c4d5 d8d5 d3c3 b7b5 d2e3 d5f3 c3c6 f3c6 b1a3 b5b4 a1c1 c6e6 a3c4 b4b5 f1d1 b5b4 d4d5 e6g4 g1f1 b4b7 d5d6 g4h3 f1g1 h3g4 g1f1 g4h3 f1e1 h3h2 d6e7 g8f6", "go"];
        let uci_reader = Box::new(uci_entity::UciReadVecStringWrapper::new(&inputs));
        let mut game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
        //let mut engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let mut engine_player1 = engine_alphabeta::EngineAlphaBeta::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            ALPHABETA_DEPTH,
        );
        engine_player1.set_id_number("white");
        let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
            Arc::new(engine_player1),
            debug_actor_opt.clone(),
            None,
        );
        //let mut engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let mut engine_player2 = engine_alphabeta::EngineAlphaBeta::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            ALPHABETA_DEPTH,
        );
        engine_player2.set_id_number("black");
        let engine_player2_dispatcher = dispatcher::EngineDispatcher::new(
            Arc::new(engine_player2),
            debug_actor_opt.clone(),
            None,
        );
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1_dispatcher.start()),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2_dispatcher.start(),
        };
        let players = player::Players::new(player1, player2);
        game_manager.set_players(players);
        let game_manager_actor = game_manager.start();
        let uci_entity = uci_entity::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
            None,
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let r = uci_entity_actor
                .send(uci_entity::handler_read::ReadUserInput)
                .await;
            println!("{:?}", r);
        }
        actix::clock::sleep(std::time::Duration::from_secs(100)).await;
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let game = game_opt.as_ref().unwrap();
        let moves = game.gen_moves();
        let moves: Vec<String> = (*moves
            .into_iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_b_move(m).cast())
            .collect::<Vec<String>>())
        .to_vec();
        assert!(!moves.contains(&"h3h2".to_string()));
    }
}
