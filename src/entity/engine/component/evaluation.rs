use super::{engine_logic as logic, feature};
use crate::entity::game::component::bitboard::piece_move::{self, table};
use crate::entity::game::component::square::Switch;
use crate::entity::game::component::{bitboard, game_state, square};
use crate::entity::stat::actor::stat_entity;
use crate::entity::stat::component::stat_data;

pub mod score;
pub mod stat_eval;

const FACTOR_PAWN_BASE: i32 = 1000;
pub const HALF_PAWN: i32 = FACTOR_PAWN_BASE / 2;
const FACTOR_CONTROL_SQUARES: i32 = 10;

const BITBOARD_CENTER: u64 = table::MASK_COL_D & table::MASK_ROW_4
    | table::MASK_COL_D & table::MASK_ROW_5
    | table::MASK_COL_E & table::MASK_ROW_4
    | table::MASK_COL_E & table::MASK_ROW_5;

pub fn handle_end_game_scenario(
    game: &game_state::GameState,
    current_depth: u8,
    max_depth: u8,
) -> score::Score {
    match game.end_game() {
        game_state::EndGame::Mat(square::Color::Black) => {
            score::Score::new(score::SCORE_MAT_WHITE, current_depth, max_depth)
        }
        game_state::EndGame::Mat(square::Color::White) => {
            score::Score::new(score::SCORE_MAT_BLACK, current_depth, max_depth)
        }
        game_state::EndGame::TimeOutLost(square::Color::White) =>
        // If the current player loses by timeout, it is an unfavorable outcome.
        {
            score::Score::new(score::SCORE_MAT_BLACK, current_depth, max_depth)
        }
        game_state::EndGame::TimeOutLost(_) =>
        // If the opponent times out, it is a favorable outcome for the current player.
        {
            score::Score::new(score::SCORE_MAT_WHITE, current_depth, max_depth)
        }
        _ => {
            // In other cases (stalemate, etc.), it might be neutral or need specific scoring based on the game rules.
            score::Score::new(0, current_depth, max_depth)
        }
    }
}

pub fn evaluate_position(
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
    let (player_can_win, player_opponent_can_win) = if feature::FEATURE_CANNOT_WIN_FORCE_NULL {
        let player_can_win = check_can_win(
            game.bit_position()
                .bit_boards_white_and_black()
                .bit_board(&player_turn),
        );
        let player_opponent_can_win = check_can_win(
            game.bit_position()
                .bit_boards_white_and_black()
                .bit_board(&player_turn.switch()),
        );
        (player_can_win, player_opponent_can_win)
    } else {
        (true, true)
    };
    let is_start_game = game.bit_position().bit_position_status().n_half_moves() <= 20;
    let default_score = evaluate_static_position(game.bit_position())
        + evaluate_dynamic_position(game.gen_control_square(), is_start_game);
    let bonus = if player_turn == square::Color::White {
        100000
    } else {
        -100000
    };
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

pub fn is_final(game: &game_state::GameState) -> bool {
    let b_white_black = game.bit_position().bit_boards_white_and_black();
    let (n_rooks_w, n_knights_w, n_bishops_w, n_queens_w, n_pawns_w) =
        count_material_one_side(b_white_black.bit_board_white());
    let (n_rooks_b, n_knights_b, n_bishops_b, n_queens_b, n_pawns_b) =
        count_material_one_side(b_white_black.bit_board_white());
    (n_rooks_w + n_rooks_b) * 5
        + (n_knights_w + n_knights_b + n_bishops_w + n_bishops_b) * 3
        + (n_queens_w + n_queens_b) * 10
        + (n_pawns_w + n_pawns_b)
        <= 13
}

fn check_can_win(bitboards: &bitboard::BitBoards) -> bool {
    let (n_rooks, n_knights, n_bishops, n_queens, n_pawns) = count_material_one_side(bitboards);
    n_rooks != 0 || n_queens != 0 || n_pawns != 0 || (n_knights + n_bishops >= 2)
}

fn evaluate_dynamic_position(
    control_squares: (piece_move::ControlSquares, piece_move::ControlSquares),
    is_start_game: bool,
) -> i32 {
    let (control_squares_white, control_squares_black) = control_squares;
    (evaluate_dynamic_position_one_side(control_squares_white, is_start_game)
        - evaluate_dynamic_position_one_side(control_squares_black, is_start_game))
        * FACTOR_CONTROL_SQUARES
}

fn evaluate_dynamic_position_one_side(
    control_squares: piece_move::ControlSquares,
    is_start_game: bool,
) -> i32 {
    let mut score = 0;
    let mask = bitboard::BitBoard::new(if is_start_game {
        BITBOARD_CENTER
    } else {
        u64::MAX
    });
    for piece_moves in control_squares.moves() {
        let control = *piece_moves.moves() & mask;
        let n_squares_control_except_pawns = control.count_ones();
        score += n_squares_control_except_pawns;
    }
    let n_squares_control_pawns = (control_squares.panws_control() & mask).count_ones();
    score += score * 2 + n_squares_control_pawns;
    score as i32
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
    (score_current as i32 - score_opponent as i32) * FACTOR_PAWN_BASE
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix::Actor;

    use crate::entity::engine::actor::engine_dispatcher as dispatcher;
    use crate::entity::engine::component::evaluation::{
        self, BITBOARD_CENTER, FACTOR_CONTROL_SQUARES,
    };
    use crate::entity::game::component::bitboard::{piece_move, zobrist};
    use crate::ui::notation::fen::{self, EncodeUserInput};
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

    use super::{evaluate_dynamic_position_one_side, evaluate_static_position_one_side};

    #[test]
    fn test_evaluation_one_side() {
        let mut bitboards = bitboard::BitBoards::default();
        bitboards.xor_piece(TypePiece::Rook, bitboard::BitBoard::new(1));
        bitboards.xor_piece(TypePiece::Pawn, bitboard::BitBoard::new(2));
        let score = evaluate_static_position_one_side(&bitboards);
        assert_eq!(score, 6);
    }

    use crate::entity::game::component::{game_state, square};
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

    #[test]
    fn test_control_squares() {
        // White queen in b3
        let fen = "7k/8/8/8/8/1Q6/8/7K w - - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let zobrist_table = zobrist::Zobrist::new();
        let game = game_state::GameState::new(position, &zobrist_table);

        let (control_white, control_black) = game.gen_control_square();
        let is_start_game = true;
        let sc_white = evaluate_dynamic_position_one_side(control_white, is_start_game);
        assert_eq!(sc_white, 3);
        let sc_black = evaluate_dynamic_position_one_side(control_black, is_start_game);
        assert_eq!(sc_black, 0);

        // The queen controls only the square d5
        let (control_white, control_black) = game.gen_control_square();
        let is_start_game = true;
        let score =
            evaluation::evaluate_dynamic_position((control_white, control_black), is_start_game);
        assert_eq!(score, 3 * FACTOR_CONTROL_SQUARES);

        // The queen control plenty squares
        let (control_white, control_black) = game.gen_control_square();
        let is_start_game = false;
        let score =
            evaluation::evaluate_dynamic_position((control_white, control_black), is_start_game);
        assert_eq!(score, 23 * 3 * FACTOR_CONTROL_SQUARES);
    }
}
