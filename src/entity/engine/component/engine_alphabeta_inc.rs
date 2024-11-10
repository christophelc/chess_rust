use actix::Addr;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::time::{SystemTime, UNIX_EPOCH};

use super::engine_logic::{self as logic, Engine};
use super::score;
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::game::component::bitboard::piece_move::GenMoves;
use crate::entity::game::component::bitboard::zobrist;
use crate::entity::game::component::game_state;
use crate::entity::game::component::square::Switch;
use crate::entity::stat::actor::stat_entity;
use crate::entity::stat::component::stat_data;
use crate::ui::notation::long_notation;
use crate::{entity::game::component::bitboard, monitoring::debug};

// IDDFS algorithm with alphabeta
#[derive(Debug)]
pub struct EngineAlphaBetaIterative {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
    zobrist_table: zobrist::Zobrist,
    max_depth: u8,
}
impl EngineAlphaBetaIterative {
    pub fn new(
        debug_actor_opt: Option<debug::DebugActor>,
        zobrist_table: zobrist::Zobrist,
        max_depth: u8,
    ) -> Self {
        assert!(max_depth >= 1);
        Self {
            id_number: "".to_string(),
            debug_actor_opt,
            zobrist_table,
            max_depth: max_depth * 2 - 1,
        }
    }
    pub fn set_id_number(&mut self, id_number: &str) {
        self.id_number = id_number.to_string();
    }

    fn alphabeta_inc(
        &self,
        game: &game_state::GameState,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
    ) -> bitboard::BitBoardMove {
        // initialisation
        let moves_score = self.alphabeta_subtree(
            game,
            self_actor.clone(),
            stat_actor_opt.clone(),
            0,
        );
        let best_move_score = moves_score.first().unwrap();
        // take one move at random in the best moves
        let moves: Vec<&score::BitboardMoveScore> = moves_score.iter().filter(|m_score| m_score.score().value() == best_move_score.score().value()).collect();
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut rng = StdRng::seed_from_u64(seed);        
        let index = rng.gen_range(0..moves.len());
        moves[index].bitboard_move().clone()
    }
    fn alphabeta_subtree(
        &self,
        game: &game_state::GameState,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        current_depth: u8,
    ) -> Vec<score::BitboardMoveScore> {
        //let num_cpus = num_cpus::get();
        let num_cpus= 1; // no parallelism
        // for each level 1 move, we add a node parent
        let chunks = self.prepare_tree_level_1(game, num_cpus);
        let results: Vec<_> = chunks
            .clone()
            .into_par_iter()
            .map(|chunk| {
                let mut game_clone = game.clone();
                let mut n_positions_evaluated: u64 = 0;
                let bitboard_move_score = self.alphabeta_inc_rec_init(
                    "",
                    &mut game_clone,
                    current_depth,
                    self.max_depth,
                    &chunk,
                    self_actor.clone(),
                    stat_actor_opt.clone(),
                    &mut n_positions_evaluated,
                );
                let (best_move, score) = (
                    bitboard_move_score.bitboard_move().clone(),
                    bitboard_move_score.score().clone(),
                );
                // FIXME: send graph to actor
                if self.debug_actor_opt.is_some() {
                    //Self::display_tree(&graph, root_node_id, 0);
                }
                //Self::write_tree(&graph_clone, node_id);
                score::BitboardMoveScore::new(best_move, score)
            })
            .collect();
        results
        //Self::merge_best_move_per_branch(&results).unwrap()
    }

    fn alphabeta_inc_rec_init(
        &self,
        variant: &str,
        game: &mut game_state::GameState,
        current_depth: u8,
        max_depth: u8,
        moves: &[bitboard::BitBoardMove],
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        n_positions_evaluated: &mut u64,
    ) -> score::BitboardMoveScore {
        let mut moves_status: Vec<score::MoveStatus> = moves.iter().map(|m| score::MoveStatus::NotEvaluated(m.clone())).collect();
        let mut best_move_opt: Option<score::BitboardMoveScore> = None;
        for max_depth_iter in 0..=max_depth-1 {
            println!("--------------------------");
            println!("max_depth:  {}", max_depth_iter);
            println!("--------------------------");
            //println!("variant {} - current_depth {} / max_depth_iter: {} - alpha_beta {:?}", variant, current_depth, max_depth_iter, alpha_beta_opt);
            moves_status.sort_by(score::compare_move_status);
            let best_move_score_opt = self.alphabeta_inc_rec(
                variant,
                game,
                current_depth,
                max_depth_iter,
                &mut moves_status,
                None,
                self_actor.clone(),
                stat_actor_opt.clone(),
                n_positions_evaluated,
            );
            let sc: Vec<String> = moves_status
                .iter()
                .flat_map(|m_status| m_status.get_score())
                .map(|m_score| m_score.clone().to_string())
                .collect();
            //println!("{} {}", variant, sc.join(", "));
            let best_move_score = best_move_score_opt.unwrap();
            best_move_opt = Some(best_move_score.clone());
        }
        best_move_opt.unwrap()
    }

    fn alphabeta_inc_rec(
        &self,
        variant: &str,
        game: &mut game_state::GameState,
        current_depth: u8,
        max_depth: u8,
        moves_status: &mut [score::MoveStatus],
        alpha_beta_opt_level_prec: Option<score::Score>,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        n_positions_evaluated: &mut u64,
    ) -> Option<score::BitboardMoveScore> {
        let mut best_move_score_opt: Option<score::BitboardMoveScore> = None;
        let mut is_mode_prune = false;
        let mut alpha_beta_opt_level_current: Option<score::Score> = None;
        for m_status in moves_status {
            if !is_mode_prune {
                let score = self.process_move(
                    game,
                    *m_status.get_move(),
                    //max_score_opt.clone(),
                    alpha_beta_opt_level_current.clone(),
                    variant,
                    self_actor.clone(),
                    stat_actor_opt.clone(),
                    n_positions_evaluated,
                    current_depth,
                    max_depth,
                );
                let move_score = score::BitboardMoveScore::new(*m_status.get_move(), score.clone());                    
                *m_status = score::MoveStatus::Evaluated(move_score.clone());
                if best_move_score_opt.is_none() || score.is_better_than(best_move_score_opt.as_ref().unwrap().score()) {
                    alpha_beta_opt_level_current = Some(move_score.score().clone());                    
                    // Send best move
                    best_move_score_opt = Some(move_score);
                    send_best_move(self_actor.clone(), best_move_score_opt.as_ref().unwrap().bitboard_move().clone());
                    if current_depth == 0 {
                        let m = best_move_score_opt.as_ref().unwrap().clone();
                        let long_algebraic_move = long_notation::LongAlgebricNotationMove::build_from_b_move(m.bitboard_move().clone());
                        let updated_variant = format!("{} {}", variant, long_algebraic_move.cast());                
                        println!("best move:{}:{}", updated_variant, m.score().value());
                    }
                    if score.value() == i32::MAX {
                        // useless to continue explore other moves
                        is_mode_prune = true;
                    }
                    if let Some(alpha_beta) = &alpha_beta_opt_level_prec {
                        if alpha_beta.value() > -score.value() {
                            // alpha_beta pruning
                            is_mode_prune = true;                        
                        }
                    }
                }
            } else {
                if current_depth < max_depth {           
                    *m_status = score::MoveStatus::Pruned(*m_status.get_move());
                }
            }
        }
        best_move_score_opt
    }

    fn prepare_tree_level_1(
        &self,
        game: &game_state::GameState,
        num_cpus: usize,
    ) -> Vec<Vec<bitboard::BitBoardMove>> {
        let moves = game.gen_moves();
        //println!("cpus: {}", num_cpus);
        let mut chunk_size = moves.len() / num_cpus;
        if chunk_size <= 10 {
            chunk_size = 10;
        }
        let chunks: Vec<Vec<bitboard::BitBoardMove>> = moves
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        chunks
    }
    fn process_move(
        &self,
        game: &mut game_state::GameState,
        m: bitboard::BitBoardMove,
        alpha_beta_opt: Option<score::Score>,
        variant: &str,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        stat_actor_opt: Option<stat_entity::StatActor>,
        n_positions_evaluated: &mut u64,
        current_depth: u8,
        max_depth: u8,
    ) -> score::Score {
        let long_algebraic_move = long_notation::LongAlgebricNotationMove::build_from_b_move(m);
        let updated_variant = format!("{} {}", variant, long_algebraic_move.cast());
        if current_depth == 0 {
            println!("{}", updated_variant);
        }
        game.play_moves(&[long_algebraic_move], &self.zobrist_table, None, false)
            .unwrap();
        update_game_status(game);
        let score = if game.end_game() == game_state::EndGame::None {
            let mut moves: Vec<bitboard::BitBoardMove> = vec![];
            if current_depth < max_depth {
                moves = game.gen_moves();
                let mut moves_status: Vec<score::MoveStatus> = moves.iter().map(|m| score::MoveStatus::NotEvaluated(m.clone())).collect();                
                let move_score_opt = self.alphabeta_inc_rec(
                    &updated_variant,
                    game,
                    current_depth + 1,
                    max_depth,
                    &mut moves_status,
                    alpha_beta_opt,
                    self_actor.clone(),
                    stat_actor_opt.clone(),
                    n_positions_evaluated,
                );
                let score = move_score_opt.unwrap().score().opposite();
                score::Score::new(score.value(), score.path_length() + 1)
            } else {
                // evaluate position for the side that has just played the last move
                let score = score::Score::new(
                    evaluate_position(game, n_positions_evaluated, &stat_actor_opt, self.id()),
                    current_depth,
                );
                //println!("{}/{} < {} > - alpha_beta {:?}", current_depth, max_depth, updated_variant, alpha_beta_opt);                
                // we want to evaluate for the current color (before move)
                score
            }
        } else {
            handle_end_game_scenario(&game, current_depth)
        };

        game.play_back();
        if current_depth == max_depth {
          //println!("{}:{}", updated_variant, score);        
        }
        score
    }
}
unsafe impl Send for EngineAlphaBetaIterative {}

const ALPHABETA_INC_ENGINE_ID_NAME: &str = "Alphabeta incremental engine";
const ALPHABETA_INC_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

impl logic::Engine for EngineAlphaBetaIterative {
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
        let moves = logic::gen_moves(&game.bit_position());
        if !moves.is_empty() {
            let best_move = self.alphabeta_inc(&game, self_actor.clone(), stat_actor_opt.clone());
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

fn handle_end_game_scenario(game: &game_state::GameState, current_depth: u8) -> score::Score {
    match game.end_game() {
        game_state::EndGame::Mat(_) => {
            // If the game ends in a checkmate, it is a favorable outcome for the player who causes the checkmate.
            score::Score::new(i32::MAX, current_depth)
        }
        game_state::EndGame::TimeOutLost(color)
            if color == game.bit_position().bit_position_status().player_turn() =>
        {
            // If the current player loses by timeout, it is an unfavorable outcome.
            score::Score::new(i32::MIN, current_depth)
        }
        game_state::EndGame::TimeOutLost(_) => {
            // If the opponent times out, it is a favorable outcome for the current player.
            score::Score::new(i32::MAX, current_depth)
        }
        _ => {
            // In other cases (stalemate, etc.), it might be neutral or need specific scoring based on the game rules.
            score::Score::new(0, current_depth)
        }
    }
}

fn update_game_status(game: &mut game_state::GameState) {
    let color = game.bit_position().bit_position_status().player_turn();
    let check_status = game
        .bit_position()
        .bit_boards_white_and_black()
        .check_status(&color);
    let can_move = game.bit_position().bit_boards_white_and_black().can_move(
        &color,
        check_status,
        game.bit_position()
            .bit_position_status()
            .pawn_en_passant()
            .as_ref(),
        game.bit_position().bit_position_status(),
    );
    let end_game = game.check_end_game(check_status, !can_move);
    game.set_end_game(end_game);
}

fn evaluate_position(
    game: &mut game_state::GameState,
    n_positions_evaluated: &mut u64,
    stat_actor_opt: &Option<stat_entity::StatActor>,
    engine_id: logic::EngineId,
) -> i32 {
    *n_positions_evaluated += 1;
    if *n_positions_evaluated % stat_data::SEND_STAT_EVERY_N_POSITION_EVALUATED == 0 {
        if let Some(stat_actor) = stat_actor_opt {
            let msg = stat_entity::handler_stat::StatUpdate::new(engine_id, *n_positions_evaluated);
            stat_actor.do_send(msg);
        }
        *n_positions_evaluated = 0;
    }
    evaluate(game.bit_position()) 
}

fn evaluate_one_side(bitboards: &bitboard::BitBoards) -> u32 {
    let n_rooks = bitboards.rooks().bitboard().iter().count() as u32;
    let n_knights = bitboards.knights().bitboard().iter().count() as u32;
    let n_bishops = bitboards.bishops().bitboard().iter().count() as u32;
    let n_queens = bitboards.queens().bitboard().iter().count() as u32;
    let n_pawns = bitboards.pawns().bitboard().iter().count() as u32;
    let score = n_rooks * 5 + n_knights * 3 + n_bishops * 3 + n_queens * 10 + n_pawns;
    //println!("r:{} n:{} b:{} q:{} p:{} -> {}", n_rooks, n_knights, n_bishops, n_queens, n_pawns, score);
    score
}

fn evaluate(bit_position: &bitboard::BitPosition) -> i32 {
    let color = bit_position.bit_position_status().player_turn().switch();
    let score_current =
        evaluate_one_side(bit_position.bit_boards_white_and_black().bit_board(&color));
    let score_opponent = evaluate_one_side(
        bit_position
            .bit_boards_white_and_black()
            .bit_board(&color.switch()),
    );
    // println!("{}", bit_position.to().chessboard());
    // println!("{:?} / {:?}", score_current, score_opponent);
    let score = score_current as i32 - score_opponent as i32;
    score
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix::Actor;

    use crate::entity::engine::actor::engine_dispatcher as dispatcher;
    use crate::{
        entity::{
            engine::component::engine_alphabeta_inc,
            game::{
                actor::game_manager,
                component::{bitboard, player, square::TypePiece},
            },
            uci::actor::uci_entity,
        },
        monitoring::debug,
        ui::notation::long_notation,
    };

    use super::evaluate_one_side;

    #[test]
    fn test_evaluation_one_side() {
        let mut bitboards = bitboard::BitBoards::default();
        bitboards.xor_piece(TypePiece::Rook, bitboard::BitBoard::new(1));
        bitboards.xor_piece(TypePiece::Pawn, bitboard::BitBoard::new(2));
        let score = evaluate_one_side(&bitboards);
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
        const ALPHABETA_INC_DEPTH: u8 = 2;

        //let debug_actor_opt: Option<debug::DebugActor> = None;
        let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 b8a6 f1a6 b7a6 d2d4 d7d5 e4e5 c7c6 g1f3 a8b8 e1g1 c8g4 d1d3 b8b4 c2c3 b4a4 b2b3 a4a5 c1d2 g4f3 g2f3 a5b5 c3c4 b5b7 c4d5 d8d5 d3c3 b7b5 d2e3 d5f3 c3c6 f3c6 b1a3 b5b4 a1c1 c6e6 a3c4 b4b5 f1d1 b5b4 d4d5 e6g4 g1f1 b4b7 d5d6 g4h3 f1g1 h3g4 g1f1 g4h3 f1e1 h3h2 d6e7 g8f6", "go"];
        let uci_reader = Box::new(uci_entity::UciReadVecStringWrapper::new(&inputs));
        let mut game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
        //let mut engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let mut engine_player1 = engine_alphabeta_inc::EngineAlphaBetaIterative::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            ALPHABETA_INC_DEPTH,
        );
        engine_player1.set_id_number("white");
        let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
            Arc::new(engine_player1),
            debug_actor_opt.clone(),
            None,
        );
        //let mut engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let mut engine_player2 = engine_alphabeta_inc::EngineAlphaBetaIterative::new(
            debug_actor_opt.clone(),
            game_manager.zobrist_table(),
            ALPHABETA_INC_DEPTH,
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
