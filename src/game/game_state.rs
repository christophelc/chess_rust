use crate::board::bitboard::piece_move::{CheckStatus, GenMoves};
use crate::board::bitboard::{zobrist, BitBoardMove, BitBoardsWhiteAndBlack};
use crate::board::{bitboard, fen, square};
use crate::uci::notation::{self, LongAlgebricNotationMove};

#[derive(Debug, Default, Clone, PartialEq)]
pub enum EndGame {
    #[default]
    None,
    Pat,
    Mat(square::Color),
    NoPawnAndCapturex50,  // 50 moves rule
    InsufficientMaterial, // King (+ bishop or knight) vs King (+ bishop or knight)
    Repetition3x,         // 3x the same position
    TimeOutLost(square::Color),
    TimeOutDraw,   // Timeout but only a King, King + Bishop or Knight
    NullAgreement, // Two players agree to end the game
}

#[derive(Debug, Clone)]
pub struct GameState {
    bit_position: bitboard::BitPosition,
    // Once a move is played, we update moves for the next player
    moves: Vec<BitBoardMove>,
    hash_positions: zobrist::ZobristHistory,
    end_game: EndGame,
}

impl GameState {
    pub fn new(position: fen::Position, zobrist_table: &zobrist::Zobrist) -> Self {
        let mut game_state = GameState {
            bit_position: bitboard::BitPosition::from(position),
            moves: vec![],
            hash_positions: zobrist::ZobristHistory::default(),
            end_game: EndGame::None,
        };
        // init moves and game status
        game_state.update_moves();
        game_state.init_hash_table(zobrist_table);
        game_state
    }
    pub fn set_end_game(&mut self, end_game: EndGame) {
        self.end_game = end_game;
    }
    fn add_hash(&mut self, hash: zobrist::ZobristHash) {
        self.hash_positions.push(hash);
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

    pub fn update_moves(&mut self) {
        let bit_position_status = self.bit_position.bit_position_status();
        let color = bit_position_status.player_turn();
        let bit_boards_white_and_black = self.bit_position.bit_boards_white_and_black();
        let check_status = bit_boards_white_and_black.check_status(&color);
        let capture_en_passant = bit_position_status.pawn_en_passant();
        self.moves = bit_boards_white_and_black.gen_moves_for_all(
            &color,
            check_status,
            &capture_en_passant,
            bit_position_status,
        );
        if self.moves.is_empty() {
            match check_status {
                CheckStatus::None => self.end_game = EndGame::Pat,
                _ => self.end_game = EndGame::Mat(color),
            }
        } else if bit_position_status.n_half_moves() >= 100 {
            self.end_game = EndGame::NoPawnAndCapturex50
        } else if self.check_insufficient_material(bit_boards_white_and_black) {
            self.end_game = EndGame::InsufficientMaterial
        } else if self
            .hash_positions
            .check_3x(bit_position_status.n_half_moves())
        {
            self.end_game = EndGame::Repetition3x
        }
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

    #[allow(dead_code)]
    fn show(&self) {
        println!("{}", self.bit_position.to().chessboard());
    }
    // play n moves from the current position
    pub fn play_moves(
        &mut self,
        valid_moves: Vec<LongAlgebricNotationMove>,
        zobrist_table: &zobrist::Zobrist,
        is_debug: bool,
    ) -> Result<Vec<BitBoardMove>, String> {
        let mut summary = vec![];
        let mut result: Result<(), String> = Ok(());
        for m in valid_moves {
            let color = self.bit_position.bit_position_status().player_turn();
            match check_move(color, m, &self.bit_position) {
                Err(err) => {
                    result = Err(err);
                    break;
                }
                Ok(b_move) => {
                    if is_debug {
                        println!("play: {:?}", b_move)
                    };
                    let mut hash = self.last_hash();
                    self.bit_position
                        .move_piece(&b_move, &mut hash, zobrist_table);
                    // update hash history
                    self.add_hash(hash);
                    summary.push(b_move);
                    if is_debug {
                        self.show()
                    };
                    self.update_moves();
                }
            }
        }
        result.map(|_| summary)
    }
}

// The start square must contain a piece
fn check_move(
    player_turn: square::Color,
    m: notation::LongAlgebricNotationMove,
    bitboard_position: &bitboard::BitPosition,
) -> Result<BitBoardMove, String> {
    let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
    let start_square = bit_boards_white_and_black.peek(m.start());
    let end_square = bit_boards_white_and_black.peek(m.end());
    match (start_square, end_square) {
        (square::Square::Empty, _) => Err(format!("empty start square {}", m.start().value())),
        (square::Square::NonEmpty(piece), square::Square::Empty) => {
            let b_move = BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion());
            check_move_level2(b_move, bitboard_position)
        },
        (square::Square::NonEmpty(piece), square::Square::NonEmpty(capture)) if capture.color() != piece.color() => {
            let b_move = BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), Some(capture.type_piece()), m.opt_promotion());
            check_move_level2(b_move, bitboard_position)
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
        &capture_en_passant,
        bit_position_status,
    );
    if moves.iter().any(|m| *m == b_move) {
        Ok(b_move)
    } else {
        let possible_moves_for_piece: Vec<String> = moves
            .iter()
            .filter(|m| m.start() == b_move.start())
            .map(|m| {
                notation::LongAlgebricNotationMove::new(m.start(), m.end(), m.promotion()).cast()
            })
            .collect();
        let invalid_move = notation::LongAlgebricNotationMove::new(
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

    use crate::{
        board::{bitboard::zobrist::Zobrist, fen::EncodeUserInput},
        fen,
        uci::notation,
    };

    #[test]
    fn test_position() {
        let fen_pos = fen::FEN_START_POSITION;
        let position = fen::Fen::decode(fen_pos).expect("Failed to decode FEN");
        let moves = vec![
            "h2h3", "h7h6", "f2f4", "b8c6", "d2d4", "c6b8", "g1f3", "b7b5", "h1g1", "h8h7", "g2g3",
            "d7d6", "c2c3", "c8e6", "b2b3", "e6g4", "c1a3", "b8a6", "d1d3", "c7c5", "h3h4", "a8c8",
        ];
        let zobrist_table = Zobrist::new();
        let mut game = super::GameState::new(position, &zobrist_table);
        let valid_moves: Vec<notation::LongAlgebricNotationMove> = moves
            .into_iter()
            .map(|m| notation::LongAlgebricNotationMove::build_from_str(m).unwrap())
            .collect();
        let _ = game.play_moves(valid_moves, &zobrist_table, false);
        //println!("{}", game.bit_position().to().chessboard());
        let fen_pos_final = fen::Fen::encode(&game.bit_position.to()).unwrap();
        let fen_pos_final_expected =
            "2rqkbn1/p3pppr/n2p3p/1pp5/3P1PbP/BPPQ1NP1/P3P3/RN2KBR1 w Q - 1 12";
        assert_eq!(fen_pos_final, fen_pos_final_expected);
        let moves = game.moves;
        let algebric_moves_expected = vec![
            "g1h1", "g1g2", "f1g2", "f1h3", "a3c1", "a3b2", "a3b4", "a3c5", "b1d2", "f3d2", "f3h2",
            "f3e5", "f3g5", "e1d1", "e1d2", "e1f2", "d3d1", "d3c2", "d3d2", "d3e3", "d3c4", "d3e4",
            "d3b5", "d3f5", "d3g6", "d3h7", "e2e3", "e2e4", "b3b4", "c3c4", "d4c5", "d4d5", "f4f5",
            "h4h5",
        ];
        let algebric_moves: Vec<String> = moves
            .into_iter()
            .map(|b_move| notation::LongAlgebricNotationMove::build_from_b_move(b_move).cast())
            .collect();
        assert_eq!(algebric_moves, algebric_moves_expected);
    }
}
