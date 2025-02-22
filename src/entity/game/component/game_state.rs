use crate::entity::game::component::bitboard;
use crate::entity::game::component::bitboard::piece_move::{CheckStatus, GenMoves};
use crate::entity::game::component::{
    bitboard::{zobrist, BitBoardMove, BitBoardsWhiteAndBlack},
    square,
};
use crate::ui::notation::{fen, long_notation};

use crate::monitoring::debug;

use super::bitboard::piece_move;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum EndGame {
    #[default]
    None,
    Pat,
    Mat(square::Color),         // color of the player that has lost
    NoPawnAndCapturex50,        // 50 moves rule
    InsufficientMaterial,       // King (+ bishop or knight) vs King (+ bishop or knight)
    Repetition3x,               // 3x the same position
    TimeOutLost(square::Color), // color of the player that has lost
    TimeOutDraw,                // Timeout but only a King, King + Bishop or Knight
    NullAgreement,              // Two players agree to end the game
}
impl EndGame {
    pub fn is_mat(&self) -> bool {
        matches!(self, EndGame::Mat(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct BackMove {
    bitboard_white_and_black_mask: bitboard::BitBoardsWhiteAndBlack,
    bit_position_status_back: bitboard::BitPositionStatus,
}
impl BackMove {
    fn new(
        bitboard_white_and_black_mask: bitboard::BitBoardsWhiteAndBlack,
        bit_position_status_back: bitboard::BitPositionStatus,
    ) -> Self {
        Self {
            bitboard_white_and_black_mask,
            bit_position_status_back,
        }
    }
}
#[derive(Debug, Clone)]
pub struct GameState {
    bit_position: bitboard::BitPosition,
    hash_positions: zobrist::ZobristHistory,
    backup: Vec<BackMove>,
    end_game: EndGame,
}
impl PartialEq for GameState {
    fn eq(&self, other: &Self) -> bool {
        self.bit_position == other.bit_position
            && self.hash_positions == other.hash_positions
            && self.backup.len() == other.backup.len()
            && self.backup.last() == other.backup.last()
            && self.end_game == other.end_game
    }
}
impl GameState {
    pub fn new(position: fen::Position, zobrist_table: &zobrist::Zobrist) -> Self {
        let mut game_state = GameState {
            bit_position: bitboard::BitPosition::from(position),
            hash_positions: zobrist::ZobristHistory::default(),
            backup: vec![],
            end_game: EndGame::None,
        };
        // init moves and game status
        game_state.init_hash_table(zobrist_table);
        game_state
    }
    pub fn set_end_game(&mut self, end_game: EndGame) {
        self.end_game = end_game;
    }
    fn add_hash(&mut self, hash: zobrist::ZobristHash) {
        self.hash_positions.push(hash);
    }
    fn store_backup(
        &mut self,
        bit_boards_white_and_black_masks: BitBoardsWhiteAndBlack,
        bit_position_status_back: bitboard::BitPositionStatus,
    ) {
        self.backup.push(BackMove::new(
            bit_boards_white_and_black_masks,
            bit_position_status_back,
        ))
    }
    // build the hash table
    fn init_hash_table(&mut self, zobrist_table: &zobrist::Zobrist) {
        // reset hash from new position
        self.hash_positions = zobrist::ZobristHistory::default();
        let hash =
            zobrist::ZobristHash::zobrist_hash_from_position(&self.bit_position, zobrist_table);
        self.add_hash(hash);
    }
    pub fn bit_position(&self) -> &bitboard::BitPosition {
        &self.bit_position
    }
    pub fn last_hash(&self) -> zobrist::ZobristHash {
        self.hash_positions
            .list()
            .last()
            .expect("Internal error: No hash position computed")
            .clone()
    }

    pub fn end_game(&self) -> EndGame {
        self.end_game.clone()
    }

    pub fn check_end_game(&self, check_status: CheckStatus, moves_is_empty: bool) -> EndGame {
        let bit_position_status = self.bit_position.bit_position_status();
        if moves_is_empty {
            match check_status {
                CheckStatus::None => EndGame::Pat,
                _ => EndGame::Mat(bit_position_status.player_turn()),
            }
        } else if bit_position_status.n_half_moves() >= 100 {
            EndGame::NoPawnAndCapturex50
        } else if self.check_insufficient_material(self.bit_position.bit_boards_white_and_black()) {
            EndGame::InsufficientMaterial
        } else if self
            .hash_positions
            .check_3x(bit_position_status.n_half_moves())
        {
            EndGame::Repetition3x
        } else {
            EndGame::None
        }
    }

    pub fn gen_control_square(&self) -> (piece_move::ControlSquares, piece_move::ControlSquares) {
        let bit_boards_white_and_black = self.bit_position.bit_boards_white_and_black();
        let control_square_white_with_pawns =
            bit_boards_white_and_black.gen_square_control(&square::Color::White);
        let control_square_black_with_pawns =
            bit_boards_white_and_black.gen_square_control(&square::Color::Black);
        (
            control_square_white_with_pawns,
            control_square_black_with_pawns,
        )
    }
    pub fn gen_moves(&self) -> Vec<BitBoardMove> {
        let bit_position_status = self.bit_position.bit_position_status();
        let color = bit_position_status.player_turn();
        let bit_boards_white_and_black = self.bit_position.bit_boards_white_and_black();
        let check_status = bit_boards_white_and_black.check_status(&color);
        let capture_en_passant = bit_position_status.pawn_en_passant();
        let moves = bit_boards_white_and_black.gen_moves_for_all(
            &color,
            check_status,
            capture_en_passant.as_ref(),
            bit_position_status,
        );
        //self.set_end_game(self.check_end_game(check_status, moves.is_empty()));
        moves
    }
    pub fn check_insufficient_material_for_color(
        &self,
        color: square::Color,
        bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
    ) -> bool {
        let bitboard = bit_boards_white_and_black.bit_board(&color);
        let relevant_pieces = *bitboard.rooks().bitboard()
            | *bitboard.queens().bitboard()
            | *bitboard.pawns().bitboard();
        // one bishop or knight only
        if relevant_pieces.empty() {
            let other = *bitboard.bishops().bitboard() | *bitboard.knights().bitboard();
            other.one_bit_set_max()
        } else {
            false
        }
    }
    fn check_insufficient_material(
        &self,
        bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
    ) -> bool {
        let bitboard_white = bit_boards_white_and_black.bit_board_white();
        let bitboard_black = bit_boards_white_and_black.bit_board_black();
        let white_relevant_pieces = *bitboard_white.rooks().bitboard()
            | *bitboard_white.queens().bitboard()
            | *bitboard_white.pawns().bitboard();
        let black_relevant_pieces = *bitboard_black.rooks().bitboard()
            | *bitboard_black.queens().bitboard()
            | *bitboard_black.pawns().bitboard();
        if white_relevant_pieces.empty() && black_relevant_pieces.empty() {
            let white_other =
                *bitboard_white.bishops().bitboard() | *bitboard_white.knights().bitboard();
            let black_other =
                *bitboard_black.bishops().bitboard() | *bitboard_black.knights().bitboard();
            white_other.one_bit_set_max() && black_other.empty()
                || white_other.empty() && black_other.one_bit_set_max()
        } else {
            false
        }
    }

    // null move is playing no move (switch side only)
    // we assume we check just after if there is a pat
    pub fn play_null_move(&mut self, zobrist_table: &zobrist::Zobrist) {
        let mut hash = self.last_hash();
        // Change player turn
        self.bit_position.change_side();
        // Do not allow capture en passant if we play twice
        self.bit_position
            .bit_position_status_into()
            .set_pawn_en_passant(None);
        hash = hash.xor_player_turn(zobrist_table);
        self.add_hash(hash);
    }
    pub fn play_back_null_move(&mut self) {
        self.hash_positions.pop();
        self.bit_position.change_side();
    }

    pub fn play_back(&mut self) {
        assert!(!self.backup.is_empty());
        let back_info = self.backup.pop().unwrap();
        self.bit_position.play_back(
            back_info.bit_position_status_back,
            back_info.bitboard_white_and_black_mask,
        );
        self.end_game = EndGame::None;
        self.hash_positions.pop();
    }

    // play n moves from the current position
    pub fn play_moves(
        &mut self,
        valid_moves: &[long_notation::LongAlgebricNotationMove],
        zobrist_table: &zobrist::Zobrist,
        debug_actor_opt: Option<debug::DebugActor>,
        is_uci_origin: bool,
    ) -> Result<Vec<BitBoardMove>, String> {
        let mut summary = vec![];
        let mut result: Result<(), String> = Ok(());
        for m in valid_moves {
            let color = self.bit_position.bit_position_status().player_turn();
            match check_move(color, *m, &self.bit_position, is_uci_origin) {
                Err(err) => {
                    println!("{:?}", err);
                    if let Some(debug_actor) = &debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(format!(
                            "play error for move {}: '{}'",
                            m.cast(),
                            err
                        )));
                    }
                    result = Err(err);
                    break;
                }
                Ok(b_move) => {
                    if let Some(debug_actor) = &debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(format!("play: {:?}", b_move)));
                    }
                    let mut hash = self.last_hash();
                    // get information for back playing
                    let (bit_position_black_and_white_before_move, bit_position_status_before_move) =
                        self.bit_position.tuple();
                    self.bit_position
                        .move_piece(&b_move, &mut hash, zobrist_table);
                    let bitboards_masks = self
                        .bit_position
                        .bit_boards_white_and_black()
                        .xor(bit_position_black_and_white_before_move);
                    self.store_backup(bitboards_masks, bit_position_status_before_move);
                    // update hash history
                    self.add_hash(hash);
                    summary.push(b_move);
                    if let Some(debug_actor) = &debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(format!(
                            "{}",
                            self.bit_position.to().chessboard()
                        )));
                    }
                }
            }
        }
        result.map(|_| summary)
    }

    pub fn check_status(&self) -> CheckStatus {
        let color = self.bit_position().bit_position_status().player_turn();
        self.bit_position()
            .bit_boards_white_and_black()
            .check_status(&color)
    }
    pub fn can_move(&self) -> bool {
        let color = self.bit_position().bit_position_status().player_turn();
        let check_status = self.check_status();
        self.bit_position().bit_boards_white_and_black().can_move(
            &color,
            check_status,
            self.bit_position()
                .bit_position_status()
                .pawn_en_passant()
                .as_ref(),
            self.bit_position().bit_position_status(),
        )
    }
    pub fn update_endgame_status(&mut self) {
        let can_move = self.can_move();
        let end_game = self.check_end_game(self.check_status(), !can_move);
        self.set_end_game(end_game);
    }
}

// The start square must contain a piece
fn check_move(
    player_turn: square::Color,
    m: long_notation::LongAlgebricNotationMove,
    bitboard_position: &bitboard::BitPosition,
    is_move_uci_origin: bool,
) -> Result<BitBoardMove, String> {
    let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
    let start_square = bit_boards_white_and_black.peek(m.start());
    let end_square = bit_boards_white_and_black.peek(m.end());
    match (start_square, end_square) {
        (square::Square::Empty, _) => Err(format!("empty start square {}", m.start().value())),
        (square::Square::NonEmpty(piece), square::Square::Empty) => {
            let b_move = BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion());
            if is_move_uci_origin {
                check_move_level2(b_move, bitboard_position)
            } else {
                Ok(b_move)
            }
        },
        (square::Square::NonEmpty(piece), square::Square::NonEmpty(capture)) if capture.color() != piece.color() => {
            let b_move = BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), Some(capture.type_piece()), m.opt_promotion());
            if is_move_uci_origin {
                check_move_level2(b_move, bitboard_position)
            } else {
                Ok(b_move)
            }
        },
        (square::Square::NonEmpty(_), square::Square::NonEmpty(_)) => Err(format!("Invalid move from {} to {} since the destination square contains a piece of the same color as the piece played." , m.start().value(), m.end().value())),
    }
}
fn check_move_level2(
    b_move: BitBoardMove,
    bitboard_position: &bitboard::BitPosition,
) -> Result<BitBoardMove, String> {
    let color = bitboard_position.bit_position_status().player_turn();
    let bit_position_status = bitboard_position.bit_position_status();
    let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
    let check_status = bit_boards_white_and_black.check_status(&color);
    let capture_en_passant = bit_position_status.pawn_en_passant();
    let moves = bit_boards_white_and_black.gen_moves_for_all(
        &color,
        check_status,
        capture_en_passant.as_ref(),
        bit_position_status,
    );
    if moves.iter().any(|m| *m == b_move) {
        Ok(b_move)
    } else {
        let possible_moves_for_piece: Vec<String> = moves
            .iter()
            .filter(|m| m.start() == b_move.start())
            .map(|m| {
                long_notation::LongAlgebricNotationMove::new(m.start(), m.end(), m.promotion())
                    .cast()
            })
            .collect();
        let invalid_move = long_notation::LongAlgebricNotationMove::new(
            b_move.start(),
            b_move.end(),
            b_move.promotion(),
        )
        .cast();
        Err(format!(
            "The move {} is invalid. Valid moves for this piece are: {:?}",
            invalid_move, possible_moves_for_piece
        ))
    }
}

#[cfg(test)]
mod tests {

    use crate::entity::game::component::bitboard::zobrist::Zobrist;
    use crate::entity::game::component::{bitboard, square};
    use crate::monitoring::debug;
    use crate::ui::notation::fen::{self, EncodeUserInput};
    use crate::ui::notation::long_notation;

    #[test]
    fn test_position() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let fen_pos = fen::FEN_START_POSITION;
        let position = fen::Fen::decode(fen_pos).expect("Failed to decode FEN");
        let moves = vec![
            "h2h3", "h7h6", "f2f4", "b8c6", "d2d4", "c6b8", "g1f3", "b7b5", "h1g1", "h8h7", "g2g3",
            "d7d6", "c2c3", "c8e6", "b2b3", "e6g4", "c1a3", "b8a6", "d1d3", "c7c5", "h3h4", "a8c8",
        ];
        let zobrist_table = Zobrist::new();
        let mut game = super::GameState::new(position, &zobrist_table);
        let valid_moves: Vec<long_notation::LongAlgebricNotationMove> = moves
            .into_iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_str(m).unwrap())
            .collect();
        let _ = game.play_moves(&valid_moves, &zobrist_table, debug_actor_opt.clone(), true);
        //println!("{}", game.bit_position().to().chessboard());
        let fen_pos_final = fen::Fen::encode(&game.bit_position.to()).unwrap();
        let fen_pos_final_expected =
            "2rqkbn1/p3pppr/n2p3p/1pp5/3P1PbP/BPPQ1NP1/P3P3/RN2KBR1 w Q - 1 12";
        assert_eq!(fen_pos_final, fen_pos_final_expected);
        let moves = game.gen_moves();
        let algebric_moves_expected = vec![
            "g1h1", "g1g2", "f1g2", "f1h3", "a3c1", "a3b2", "a3b4", "a3c5", "b1d2", "f3d2", "f3h2",
            "f3e5", "f3g5", "e1d1", "e1d2", "e1f2", "d3d1", "d3c2", "d3d2", "d3e3", "d3c4", "d3e4",
            "d3b5", "d3f5", "d3g6", "d3h7", "e2e3", "e2e4", "b3b4", "c3c4", "d4c5", "d4d5", "f4f5",
            "h4h5",
        ];
        let algebric_moves: Vec<String> = moves
            .into_iter()
            .map(|b_move| long_notation::LongAlgebricNotationMove::build_from_b_move(b_move).cast())
            .collect();
        assert_eq!(algebric_moves, algebric_moves_expected);
    }
    #[test]
    fn test_pat() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let fen_pos = "8/4k3/8/1N3pP1/2R2P2/P5K1/P2B4/8 b - - 0 48";
        let position = fen::Fen::decode(fen_pos).expect("Failed to decode FEN");
        let moves = vec!["e7e6", "c4c7", "e6d5", "c7d7", "d5e4"];
        let zobrist_table = Zobrist::new();
        let mut game = super::GameState::new(position, &zobrist_table);
        let valid_moves: Vec<long_notation::LongAlgebricNotationMove> = moves
            .into_iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_str(m).unwrap())
            .collect();
        let _ = game.play_moves(&valid_moves, &zobrist_table, debug_actor_opt.clone(), true);
        println!("{}", game.bit_position().to().chessboard());
        game.play_null_move(&zobrist_table);
        let moves = game.gen_moves();
        assert!(moves.is_empty());
        assert!(!game.can_move());
        assert_eq!(game.end_game(), super::EndGame::None);
        game.update_endgame_status();
        assert_eq!(game.end_game(), super::EndGame::Pat)
    }

    #[test]
    fn test_play_back() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let fen_pos = fen::FEN_START_POSITION;
        let position = fen::Fen::decode(fen_pos).expect("Failed to decode FEN");
        let moves = vec!["h2h3", "h7h6", "f2f4"];
        let zobrist_table = Zobrist::new();
        let mut game = super::GameState::new(position, &zobrist_table);
        let valid_moves: Vec<long_notation::LongAlgebricNotationMove> = moves
            .into_iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_str(m).unwrap())
            .collect();
        let _ = game.play_moves(&valid_moves, &zobrist_table, debug_actor_opt.clone(), true);
        game.play_back();
        game.play_back();
        game.play_back();
        let bit_position = game.bit_position();
        assert!(*bit_position == bitboard::BitPosition::from(position))
    }

    #[test]
    fn test_play_null_move_back() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let fen_pos = fen::FEN_START_POSITION;
        let position = fen::Fen::decode(fen_pos).expect("Failed to decode FEN");
        let zobrist_table = Zobrist::new();
        let mut game = super::GameState::new(position, &zobrist_table);
        // play white twice
        let moves = vec!["e2e4", "e7e5", "f1e2", "f8a3", "b2a3"];
        let valid_moves: Vec<long_notation::LongAlgebricNotationMove> = moves
            .into_iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_str(m).unwrap())
            .collect();
        let _ = game.play_moves(&valid_moves, &zobrist_table, debug_actor_opt.clone(), true);
        let color = game.bit_position().bit_position_status().player_turn();
        // It should be black to play
        assert_eq!(color, square::Color::Black);
        // Let us do a null move
        game.play_null_move(&zobrist_table);
        // It is again white to play
        let color = game.bit_position().bit_position_status().player_turn();
        assert_eq!(color, square::Color::White);
        // check move c1b2 is valid
        let moves: Vec<String> = game
            .gen_moves()
            .iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_b_move(*m).cast())
            .collect();
        let mv = "c1b2";
        assert!(moves.iter().any(|m| m == mv));
        // play the move Bb2
        let valid_moves = long_notation::LongAlgebricNotationMove::build_from_str(mv).unwrap();
        let _ = game.play_moves(
            &vec![valid_moves],
            &zobrist_table,
            debug_actor_opt.clone(),
            true,
        );
        assert_eq!(
            game.bit_position().bit_position_status().player_turn(),
            square::Color::Black
        );
        // play back last white move Bb2
        game.play_back();
        assert_eq!(
            game.bit_position().bit_position_status().player_turn(),
            square::Color::White
        );
        game.play_back_null_move();
        assert_eq!(
            game.bit_position().bit_position_status().player_turn(),
            square::Color::Black
        );
        // play back white move bxa3
        game.play_back();
        // play back black move Ba3
        game.play_back();
        println!("{}", game.bit_position().to().chessboard());
        assert_eq!(
            game.bit_position().bit_position_status().player_turn(),
            square::Color::Black
        );
        let moves: Vec<String> = game
            .gen_moves()
            .iter()
            .map(|m| long_notation::LongAlgebricNotationMove::build_from_b_move(*m).cast())
            .collect();
        let mv = "f8c5";
        println!("{}", moves.join(","));
        assert!(moves.iter().any(|m| m == mv))
    }
}
