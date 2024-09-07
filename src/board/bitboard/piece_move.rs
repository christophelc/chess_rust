mod table;
use table::table_bishop;
use table::table_rook;

use crate::board;
use crate::board::{
    bitboard, square::{self, TypePiece},
};
use super::BitBoard;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckStatus {
    SimpleCheck { attacker: square::TypePiece, attacker_index: u8 } ,
    DoubleCheck,
    NoCheck,
}


impl board::fen::Position {
    /// check status from a position
    pub fn check_status(&self) -> CheckStatus {
        let bit_position = board::bitboard::BitPosition::from(self.clone());        
        bit_position.bit_boards_white_and_black().check_status(&self.status().player_turn())
    }
}

pub trait GenMoves {
    fn gen_moves(
        &self,
        type_piece: &TypePiece,
        color: &square::Color,        
        check_status: CheckStatus,
        bit_board_type_piece: &bitboard::BitBoard,
        capture_en_passant: &Option<u64>,        
    ) -> Vec<PieceMoves>;

    fn check_status(
        &self,
        color: &square::Color
    ) -> CheckStatus;    
}

impl GenMoves for bitboard::BitBoardsWhiteAndBlack {
    /// gen moves for one piece at index
    // gen moves for all piece of one type
    fn gen_moves(
        &self,
        type_piece: &TypePiece,
        color: &square::Color,
        check_status: CheckStatus,        
        bit_board_type_piece: &bitboard::BitBoard,
        capture_en_passant: &Option<u64>,        
    ) -> Vec<PieceMoves> {
        let (bit_board, bit_board_opponent) = if *color == square::Color::White {
            (self.bit_board_white().concat_bit_boards(), self.bit_board_black().concat_bit_boards())
        } else {
            (self.bit_board_black().concat_bit_boards(), self.bit_board_white().concat_bit_boards())
        };
        let mut moves = Vec::new();

        // TODO: 
        // if check: generate moves for king and remove attacked squares
        //   - if double check: return
        //   - if simple check: generate moves to capture the attacker
        // else:
        let mut bb = bit_board_type_piece.value();
        while bb != 0 {
            let lsb = bb.trailing_zeros();
            if let Some(moves_for_piece) =
                gen_moves_for_piece(type_piece, color, lsb as u8, &bit_board, &bit_board_opponent, capture_en_passant)
            {
                moves.push(moves_for_piece);
            }
            bb &= bb - 1; // Remove lsb
        }
        moves
    }

    /// check if the current king of color 'color' is under check
    fn check_status(
        &self,
        color: &square::Color
    ) -> CheckStatus {
        let (bit_board, bit_board_opponent) = if *color == square::Color::White {
            (self.bit_board_white(), self.bit_board_black())
        } else {
            (self.bit_board_black(), self.bit_board_white())
        };        
        let king_index = bit_board.king().index();
        let king_bit_board = BitBoard::new(1 << king_index);
        // Generate king moves as if it were a rook, bishop, knight, pawn
        let king_as_rook = self.gen_moves(&square::TypePiece::Rook , &color, CheckStatus::NoCheck, &king_bit_board, &None).get(0).map(|m| m.moves().value()).unwrap_or(0);
        let king_as_bishop = self.gen_moves(&square::TypePiece::Bishop , &color, CheckStatus::NoCheck, &king_bit_board, &None).get(0).map(|m| m.moves().value()).unwrap_or(0);        
        let king_as_knight = self.gen_moves(&square::TypePiece::Knight , &color, CheckStatus::NoCheck, &king_bit_board, &None).get(0).map(|m| m.moves().value()).unwrap_or(0);                
        let king_as_pawn = gen_pawn_squares_attacked(king_index, &color, &bit_board_opponent.concat_bit_boards());

        // Intersect king moves with rooks, bishops, knights, pawns bitboards of the opposite color
        let rook_attackers= king_as_rook & bit_board_opponent.rooks().value();
        let bishop_attackers= king_as_bishop & bit_board_opponent.bishops().value();        
        let queen_attackers: u64 = (king_as_rook | king_as_bishop) & bit_board_opponent.queens().value();
        let knight_attackers: u64 = king_as_knight & bit_board_opponent.knights().value();        
        // TODO: generate pawn opponent capture for king and intersect with opponent panws bitboard
        let pawn_attackers: u64 = king_as_pawn & bit_board_opponent.pawns().value();
        match (sign(rook_attackers), sign(bishop_attackers), sign(queen_attackers), sign(knight_attackers), sign(pawn_attackers)) {
            (0, 0, 0, 0, 0) => CheckStatus::NoCheck,
            (1, 0, 0, 0, 0) => CheckStatus::SimpleCheck { attacker: (TypePiece::Rook), attacker_index: (rook_attackers.trailing_zeros() as u8) },
            (0, 1, 0, 0, 0) => CheckStatus::SimpleCheck { attacker: (TypePiece::Bishop), attacker_index: (bishop_attackers.trailing_zeros() as u8) },            
            (0, 0, 1, 0, 0) => CheckStatus::SimpleCheck { attacker: (TypePiece::Queen), attacker_index: (queen_attackers.trailing_zeros() as u8) },                        
            (0, 0, 0, 1, 0) => CheckStatus::SimpleCheck { attacker: (TypePiece::Knight), attacker_index: (knight_attackers.trailing_zeros() as u8) },     
            (0, 0, 0, 0, 1) => CheckStatus::SimpleCheck { attacker: (TypePiece::Pawn), attacker_index: (pawn_attackers.trailing_zeros() as u8) },                 
            _ => CheckStatus::DoubleCheck
        }        
    }
}

fn sign(u: u64) -> u8 {
    if u == 0 { 0 } else { 1 }
}

fn gen_moves_for_piece(
    type_piece: &TypePiece,
    color: &square::Color,    
    index: u8,
    bit_board: &bitboard::BitBoard, // color for piece at index
    bit_board_opponent: &bitboard::BitBoard, // opponent color
    capture_en_passant: &Option<u64>,    
) -> Option<PieceMoves> {
    match type_piece {
        &square::TypePiece::Rook => gen_moves_for_rook(index, bit_board, bit_board_opponent),
        &square::TypePiece::Bishop => gen_moves_for_bishop(index, bit_board, bit_board_opponent),
        &square::TypePiece::Knight => gen_moves_for_knight(index, bit_board, bit_board_opponent),
        &square::TypePiece::King => gen_moves_for_king(index, bit_board_opponent),
        &square::TypePiece::Queen => gen_moves_for_queen(index, bit_board, bit_board_opponent),
        &square::TypePiece::Pawn => gen_moves_for_pawn(index, color, bit_board, bit_board_opponent, capture_en_passant),
    }
}

fn moves_non_empty(
    index: u8,
    moves_bitboard: u64,
    bit_board: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let moves_bitboard = moves_bitboard & !bit_board.value();
    if moves_bitboard == 0 {
        None
    } else {
        Some(PieceMoves {
            index,
            moves: bitboard::BitBoard(moves_bitboard),
        })
    }
}

// moves generation are not optimized (as a first implementation)
fn gen_moves_for_king(index: u8, bit_board: &bitboard::BitBoard) -> Option<PieceMoves> {
    let is_row_1 = index < 8;
    let is_col_a = index % 8 == 0;
    let is_row_8 = index >= 56;
    let is_col_h = index % 8 == 7;
    let deltas: Vec<i8> = match (is_row_1, is_col_a, is_row_8, is_col_h) {
        // No edges or corners
        (false, false, false, false) => vec![-9, -8, -7, -1, 1, 7, 8, 9],
        // Single edges
        (false, false, false, true) => vec![-9, -8, -1, 7, 8],
        (false, false, true, false) => vec![-9, -8, -7, -1, 1],
        (false, true, false, false) => vec![-8, -7, 1, 8, 9],
        (true, false, false, false) => vec![-1, 1, 7, 8, 9],
        // Corners
        (true, true, false, false) => vec![1, 8, 9],
        (true, false, false, true) => vec![-1, 7, 8],
        (false, true, true, false) => vec![-8, -7, 1],
        (false, false, true, true) => vec![-9, -8, -1],
        // incompatible conditions: code never reached
        _ => vec![],
    };
    let mut moves_bitboard: u64 = 0;
    for &delta in deltas.iter() {
        let new_pos = index as i8 + delta;
        if new_pos >= 0 && new_pos < 64 {
            // Ensure within board bounds
            let pos = new_pos as u8;
            moves_bitboard |= 1 << pos;
        } else {
            panic!("This code should never be reached.")
        }
    }
    moves_non_empty(index, moves_bitboard, bit_board)
}

fn gen_moves_for_knight(
    index: u8,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let deltas: [(i8, i8); 8] = [
        (-1, -2),
        (-1, 2),
        (1, -2),
        (1, 2),
        (-2, -1),
        (-2, 1),
        (2, -1),
        (2, 1),
    ];
    let row: i8 = (index / 8) as i8;
    let col: i8 = (index % 8) as i8;
    let mut moves_bitboard: u64 = 0;
    for (dx, dy) in deltas {
        let x = col + dx;
        let y = row + dy;
        if x >= 0 && x < 8 && y >= 0 && y < 8 {
            moves_bitboard |= 1 << ((x + y * 8) as u8)
        }
    }
    moves_non_empty(index, moves_bitboard, bit_board)
}

fn gen_moves_for_rook_horizontal(index: u8, blockers_h: u64, mask_h: u64) -> u64 {
    let blockers_h = blockers_h & mask_h;
    let col = index % 8;
    let index_col_a = index - col;
    let blockers_first_row = (blockers_h << index_col_a) as u8;
    (table_rook::table_rook_h(col, blockers_first_row) as u64) << index_col_a
}

fn gen_moves_for_rook_vertical(index: u8, blockers_v: u64, mask_v: u64) -> u64 {
    table::table_rook::table_rook_v(index, blockers_v, mask_v)    
}


fn gen_moves_for_rook(
    index: u8,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let col = index % 8;
    let mask_h = 255 << (index + 7 - col);
    let mask_v = table::MASK_COL_A << (index % 8);    
    let blockers = bit_board.value() | bit_board_opponent.value();

    let moves_horizontal = gen_moves_for_rook_horizontal(index, blockers, mask_h);
    let moves_vertical = gen_moves_for_rook_vertical(index, blockers, mask_v);
    let moves_bitboard = moves_horizontal | moves_vertical;
    moves_non_empty(index, moves_bitboard, bit_board)
}

fn gen_moves_for_bishop(
    index: u8,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let blockers = bit_board.value() | bit_board_opponent.value();
    let moves_bitboard = table_bishop::bishop_moves(index, blockers);
    moves_non_empty(index, moves_bitboard, bit_board)
}

fn gen_moves_for_queen(
    index: u8,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,    
) -> Option<PieceMoves> {
    let rook_moves = gen_moves_for_rook(index, bit_board, bit_board_opponent);
    let bishop_moves = gen_moves_for_bishop(index, bit_board, bit_board_opponent);
    match (rook_moves, bishop_moves) {
        (None, None) => None,
        (left, None) => left,
        (None, right) => right,
        (Some(left), Some(right)) => Some(PieceMoves {
            index,
            moves: bitboard::BitBoard(left.moves().value() | right.moves().value()),
        })
    }    
}

fn gen_pawn_squares_attacked(
    index: u8,
    color: &square::Color,
    bit_board_opponent: &bitboard::BitBoard,
) -> u64 {
    let col = index % 8;    
    let mut moves: u64 = 0;
    match color {
        square::Color::White => {
            // capture up left
            if col > 0 {
                let to = 1 << (index + 7);
                if (to & bit_board_opponent.value()) != 0 {
                    moves |= to;
                }
            }
            // capture up right            
            if col < 7 {
                let to = 1 << (index + 9);            
                if (to & bit_board_opponent.value()) != 0 {
                    moves |= to;
                }
            }
        },
        square::Color::Black => {
            if col > 0 {
                // catpure left down
                let to = 1 << (index - 9);
                if (to & bit_board_opponent.value()) != 0 {
                    moves |= to;
                }
            }
            // catpure right down            
            if col < 7 {
                let to = 1 << (index - 7);            
                if (to & bit_board_opponent.value()) != 0 {
                    moves |= to;
                }
            }
        },                
    }
    moves
}
fn gen_pawn_non_attecker_moves(
    index: u8,
    color: &square::Color,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
    capture_en_passant: &Option<u64>,    
) -> u64 {
    let row = index / 8;
    let blockers = bit_board.value() | bit_board_opponent.value();
    let mut moves: u64 = 0;
    match color {
        square::Color::White => {
            // up x 1
            let to = 1 << (index + 8);
            if to & blockers == 0 {
                moves |= to;
                // up x 2
                if row == 1 {
                    let to = 1 << (index + 16);
                    if to & blockers == 0 {
                        moves |= to;
                    }
                }
            }
            // capture en passant
            if let Some(en_passant) = capture_en_passant {
                let en_passant_idx = en_passant.trailing_zeros() as u8;
                if index + 7 == en_passant_idx || index + 9 == en_passant_idx {
                    moves |= 1 << en_passant_idx;
                }
            }
        },
        square::Color::Black => {
            // down x 1
            let to = 1 << (index - 8);
            if to & blockers == 0 {
                moves |= to;
                // down x 2
                if row == 6 {
                    let to = to >> 8;
                    if to & blockers == 0 {
                        moves |= to;
                    }
                }
            }
            // capture en passant
            if let Some(en_passant) = capture_en_passant {
                let en_passant_idx = en_passant.trailing_zeros() as u8;
                if index - 7 == en_passant_idx || index - 9 == en_passant_idx {
                    moves |= 1 << en_passant_idx;
                }
            }            
        },        
    }    
    moves
}

fn gen_moves_for_pawn(
    index: u8,
    color: &square::Color,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
    capture_en_passant: &Option<u64>,
) -> Option<PieceMoves> {

    match gen_pawn_non_attecker_moves(index, color, bit_board, bit_board_opponent, capture_en_passant) | gen_pawn_squares_attacked(index, color, bit_board_opponent) {
        0 => None,
        to => Some(PieceMoves::new(index, to)),
    }
}

#[derive(Debug)]
pub struct PieceMoves {
    /// where is the piece
    index: u8,
    /// BitBoard representing all possible moves    
    moves: bitboard::BitBoard,
}
impl PieceMoves {
    pub fn index(&self) -> u8 {
        self.index
    }
    pub fn moves(&self) -> &bitboard::BitBoard {
        &self.moves
    }
    pub fn new(index: u8, moves: u64) -> Self {
        PieceMoves {
            index,
            moves: bitboard::BitBoard(moves),
        }
    }
}

struct MovesPerTypePiece {
    rooks_moves: Vec<PieceMoves>,
    bishops_moves: Vec<PieceMoves>,
    knights_moves: Vec<PieceMoves>,
    king_moves: Option<PieceMoves>,
    queens_moves: Vec<PieceMoves>,
    pawns_moves: Vec<PieceMoves>,
}
impl MovesPerTypePiece {
    pub fn new() -> Self {
        MovesPerTypePiece {
            rooks_moves: Vec::new(),
            bishops_moves: Vec::new(),
            knights_moves: Vec::new(),
            king_moves: None,
            queens_moves: Vec::new(),
            pawns_moves: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitboard::BitBoard;

    fn list_index(bit_board: &bitboard::BitBoard) -> Vec<u8> {
        let mut v = Vec::new();
        let mut bb = bit_board.0;
        while bb != 0 {
            let lsb = bb.trailing_zeros();
            v.push(lsb as u8);
            bb &= bb - 1; // Remove lsb
        }
        v
    }

    ////////////////////////////////////////////////////////
    /// Bit operations tests
    ////////////////////////////////////////////////////////    ///
    #[test]
    #[ignore]
    fn poc() {
        println!("poc");
        let matrix = 63 << 1;
        let column = 2;
        let mask: u64 = 0x0101010101010101 << column;
        let column_bits = matrix & mask;
        let shifted_column_bits = column_bits >> 1;
        println!("{}", bitboard::BitBoard(mask));
        println!("{}", bitboard::BitBoard(shifted_column_bits));
    }

    #[test]
    #[ignore]
    fn test_single_bit() {
        assert_eq!(bitboard::BitBoard(1 << 5).trailing_zeros(), 5);
        assert_eq!((bitboard::BitBoard(1 << 0)).trailing_zeros(), 0);
        assert_eq!((bitboard::BitBoard(1 << 15)).trailing_zeros(), 15);
    }

    #[test]
    #[ignore]
    fn test_zero_value() {
        assert_eq!(bitboard::BitBoard(0u64).trailing_zeros(), 64);
    }

    #[test]
    #[ignore]
    fn test_multiple_bits() {
        let value = bitboard::BitBoard((1 << 5) | (1 << 3));
        assert_eq!(value.trailing_zeros(), 3);
    }

    #[test]
    #[ignore]
    fn test_highest_bit() {
        assert_eq!((bitboard::BitBoard(1u64 << 63)).trailing_zeros(), 63);
    }

    #[test]
    #[ignore]
    fn test_empty_bitboard() {
        let bitboard = BitBoard(0);
        assert_eq!(list_index(&bitboard), vec![]);
    }

    #[test]
    #[ignore]
    fn test_list_index_single_bit() {
        let bitboard = BitBoard(1 << 5); // bit at position 5
        assert_eq!(list_index(&bitboard), vec![5]);
    }

    #[test]
    #[ignore]
    fn test_list_index_multiple_bits() {
        let bitboard = BitBoard((1 << 5) | (1 << 15) | (1 << 30)); // bits at positions 5, 15, 30
        let mut result = list_index(&bitboard);
        result.sort(); // Sorting the result to ensure order for comparison
        assert_eq!(result, vec![5, 15, 30]);
    }

    #[test]
    #[ignore]
    fn test_list_index_bits_at_edges() {
        let bitboard = BitBoard((1 << 0) | (1 << 63)); // bits at positions 0 and 63
        let mut result = list_index(&bitboard);
        result.sort(); // Sorting to ensure consistent order
        assert_eq!(result, vec![0, 63]);
    }

    ////////////////////////////////////////////////////////
    /// knight moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_king_center_moves() {
        let king_position = 27; // Somewhere in the center of the board
        let bit_board = BitBoard(0); // No friendly pieces blocking
        let result = gen_moves_for_king(king_position, &bit_board).unwrap();
        assert_eq!(result.index, king_position);
        assert_eq!(result.moves.0, 0x1C141C0000); // Expected moves bitboard for center position
    }

    #[test]
    fn test_king_edge_moves() {
        let king_position = 8; // On the edge (A file)
        let bit_board = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board).unwrap();
        assert_eq!(result.index, king_position);
        let expected_moves = (1 << 0) | (1 << 1) | (1 << 9) | (1 << 16) | (1 << 17);
        assert_eq!(result.moves.0, expected_moves); // Expected moves bitboard for an edge position
    }

    #[test]
    fn test_king_corner_moves() {
        let king_position = 0; // Top left corner (A1)
        let bit_board = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board).unwrap();
        assert_eq!(result.index, king_position);
        let expected_moves = (1 << 1) | (1 << 8) | (1 << 9);
        assert_eq!(result.moves.0, expected_moves); // Expected moves bitboard for corner position
    }

    #[test]
    fn test_king_blocked_by_friendly_pieces() {
        let king_position = 27; // Center of the board
        let bit_board = BitBoard(
            (1 << 18)
                | (1 << 19)
                | (1 << 20)
                | (1 << 26)
                | (1 << 28)
                | (1 << 34)
                | (1 << 35)
                | (1 << 36),
        );
        let result = gen_moves_for_king(king_position, &bit_board);
        assert!(result.is_none()); // Expect no moves available
    }

    #[test]
    #[should_panic]
    fn test_invalid_king_position() {
        let king_position = 64; // Invalid position
        let bit_board = BitBoard(0);
        let _ = gen_moves_for_king(king_position, &bit_board);
    }
    #[test]
    fn test_king_corner_h1_moves() {
        let king_position = 7; // Top right corner (H1)
        let bit_board = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board).unwrap();
        let expected_moves = (1 << 6) | (1 << 14) | (1 << 15); // Moves: G1, H2, G2
        assert_eq!(result.moves.0, expected_moves);
    }

    // Test for the bottom-left corner (A8)
    #[test]
    fn test_king_corner_a8_moves() {
        let king_position = 56; // Bottom left corner (A8)
        let bit_board = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board).unwrap();
        let expected_moves = (1 << 48) | (1 << 49) | (1 << 57); // Moves: A7, B7, B8
        assert_eq!(result.moves.0, expected_moves);
    }

    // Test for the bottom-right corner (H8)
    #[test]
    fn test_king_corner_h8_moves() {
        let king_position = 63; // Bottom right corner (H8)
        let bit_board = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board).unwrap();
        let expected_moves = (1 << 62) | (1 << 54) | (1 << 55); // Moves: G8, H7, G7
        assert_eq!(result.moves.0, expected_moves);
    }

    // Test for an arbitrary position in row 1 (B1)
    #[test]
    fn test_king_row1_b1_moves() {
        let king_position = 1; // B1
        let bit_board = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board).unwrap();
        let expected_moves = (1 << 0) | (1 << 2) | (1 << 8) | (1 << 9) | (1 << 10); // Moves: A1, C1, A2, B2, C2
        assert_eq!(result.moves.0, expected_moves);
    }

    // Test for an arbitrary position in row 8 (G8)
    #[test]
    fn test_king_row8_g8_moves() {
        let king_position = 62; // G8
        let bit_board = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board).unwrap();
        let expected_moves = (1 << 61) | (1 << 63) | (1 << 53) | (1 << 54) | (1 << 55); // Moves: F8, H8, F7, G7, H7
        assert_eq!(result.moves.0, expected_moves);
    }

    ////////////////////////////////////////////////////////
    /// knight moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn knight_center_moves() {
        let knight_index = 27u8; // Position at center of the board (d4)
        let empty_board = BitBoard::new(0);
        let opponent_board = BitBoard::new(0);

        let moves = gen_moves_for_knight(knight_index, &empty_board, &opponent_board).unwrap();
        // Moves from d4 are to e2, f3, f5, e6, c6, b5, b3, c2 (calculating their respective bit positions)
        let expected_moves =
            1 << 10 | 1 << 12 | 1 << 17 | 1 << 21 | 1 << 33 | 1 << 37 | 1 << 42 | 1 << 44;
        assert_eq!(moves.moves().0, expected_moves);
    }

    #[test]
    fn knight_corner_moves() {
        let knight_index = 0u8; // Position at a1
        let empty_board = BitBoard::new(0);
        let opponent_board = BitBoard::new(0);

        let moves = gen_moves_for_knight(knight_index, &empty_board, &opponent_board).unwrap();
        // Moves from a1 are to b3 and c2
        let expected_moves = 1 << 10 | 1 << 17;
        assert_eq!(moves.moves().0, expected_moves); // Moves from a1 should be limited to b3 and c2
    }

    #[test]
    fn knight_edge_moves() {
        let knight_index = 8u8; // Position at a2
        let empty_board = BitBoard::new(0);
        let opponent_board = BitBoard::new(0);

        let moves = gen_moves_for_knight(knight_index, &empty_board, &opponent_board).unwrap();
        // Moves from a2 are to b4, c3, and c1
        let expected_moves = 1 << 2 | 1 << 18 | 1 << 25;
        assert_eq!(moves.moves().0, expected_moves); // Valid moves from a2
    }

    #[test]
    fn knight_moves_with_blockages() {
        let knight_index = 27u8; // d4 again for center moves
                                 // Block e6 and c2 with own pieces
        let own_pieces = BitBoard::new(1 << 17 | 1 << 44); // Block e6 and b3
        let opponent_board = BitBoard::new(0);

        let moves = gen_moves_for_knight(knight_index, &own_pieces, &opponent_board).unwrap();
        // Adjusted for blockages, valid moves are to e2, f3, f5, c6, b5, b3, c2
        let expected_moves = 1 << 10 | 1 << 12 | 1 << 21 | 1 << 33 | 1 << 37 | 1 << 42;
        assert_eq!(moves.moves().value(), expected_moves);
    }

    #[test]
    fn knight_capture_moves() {
        let knight_index = 27u8; // d4
        let empty_board = BitBoard::new(0);
        // Block e6 and c2 with own pieces
        let opponent_pieces = BitBoard::new(1 << 17 | 1 << 44); // Block e6 and b3

        let moves = gen_moves_for_knight(knight_index, &empty_board, &opponent_pieces).unwrap();
        // Includes potential captures, valid moves are e2, f3, f5, e6, c6, b5, b3, c2
        let expected_moves =
            1 << 10 | 1 << 12 | 1 << 17 | 1 << 21 | 1 << 33 | 1 << 37 | 1 << 42 | 1 << 44;
        assert_eq!(moves.moves().0, expected_moves); // Includes potential captures
    }

    ////////////////////////////////////////////////////////
    /// Rook moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_rook_no_blockers() {
        let index = 8; // Position of the rook at the start of the second row
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 254 << 8 | (1 | 1 << 16 | 1 << 24 | 1 << 32 | 1 << 40 | 1 << 48 | 1 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_to_left() {
        let index = 17; // Rook on the third row, second column
        let bit_board = bitboard::BitBoard(1 << index | 1 << 16);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 252 << 16 | (2 | 2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_to_left() {
        let index = 17; // Rook on the third row, second column
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(1 << 16);
        let expected = 253 << 16 | (2 | 2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_to_right() {
        let index = 17; // Rook on the third row, second column
        let bit_board = bitboard::BitBoard(1 << index | 1 << 23);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 125 << 16 | (2 | 2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        println!("{}", bitboard::BitBoard(result));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_to_right() {
        let index = 17; // Rook on the third row, second column
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(1 << 23);
        let expected = 253 << 16 | (2 | 2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_on_both_sides() {
        let index = 18; // Rook on the third row, third column
        let bit_board = bitboard::BitBoard(1 << index | 1 << 16 | 1 << 23);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 122 << 16 | (4 | 4 << 8 | 4 << 24 | 4 << 32 | 4 << 40 | 4 << 48 | 4 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_on_both_sides() {
        let index = 18; // Rook on the third row, third column
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(1 << 16 | 1 << 23);
        let expected = 251 << 16 | (4 | 4 << 8 | 4 << 24 | 4 << 32 | 4 << 40 | 4 << 48 | 4 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_on_first_column() {
        let index = 24; // Rook at the start of the fourth row
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 254 << 24 | (1 | 1 << 8 | 1 << 16 | 1 << 32 | 1 << 40 | 1 << 48 | 1 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_on_last_column() {
        let index = 31; // Rook at the end of the fourth row
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 127 << 24
            | (128 | 128 << 8 | 128 << 16 | 128 << 32 | 128 << 40 | 128 << 48 | 128 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_full_row_of_blockers() {
        let index = 40; // Rook somewhere in the middle of the fifth row
        let bit_board = bitboard::BitBoard(255 << 40 | 1 << 32 | 1 << 48);
        let bit_board_opponent = bitboard::BitBoard(0);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent);
        assert!(result.is_none());
    }

    #[test]
    fn test_rook_blockers_to_up() {
        let index = 17; // Rook on the third row, second column
        let bit_board = bitboard::BitBoard(1 << index | 1 << 25);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 253 << 16 | (2 | 2 << 8);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_to_down() {
        let index = 17; // Rook on the third row, second column
        let bit_board = bitboard::BitBoard(1 << index | 1 << 9);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 253 << 16 | (2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_to_up() {
        let index = 17; // Rook on the third row, second column
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(1 << 25);
        let expected = 253 << 16 | (2 | 2 << 8 | 2 << 24);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rook_blockers_opponent_to_down() {
        let index = 17; // Rook on the third row, second column
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(1 << 9);
        let expected = 253 << 16 | (2 << 8 | 2 << 24 | 2 << 32 | 2 << 40 | 2 << 48 | 2 << 56);
        let result = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    ////////////////////////////////////////////////////////
    /// Bishop moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_bishop_blockers() {
        let index = 20;
        let bit_board = bitboard::BitBoard(1 << index | 1 << 34);
        let bit_board_opponent = bitboard::BitBoard(1 << 6);
        let expected =
            1 << 2 | (1 << 6 | 1 << 11 | 1 << 13 | 1 << 27 | 1 << 29 | 1 << 38 | 1 << 47);
        let result = gen_moves_for_bishop(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_queen_no_blockers() {
        let index = 8; // Position of the rook at the start of the second row
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(0);
        let result_rook = gen_moves_for_rook(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        let result_bishop = gen_moves_for_bishop(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;        
        let expected = result_rook | result_bishop;
        let result = gen_moves_for_queen(index, &bit_board, &bit_board_opponent)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    ////////////////////////////////////////////////////////
    /// Pawn moves
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_pawn_white_no_blockers() {
        let index = 20;
        let capture_en_passant: Option<u64> = None;
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected =1 << 28;
        let result = gen_moves_for_pawn(index, &square::Color::White, &bit_board, &bit_board_opponent, &capture_en_passant)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_pawn_white_row1_no_blockers() {
        let index = 10;
        let capture_en_passant: Option<u64> = None;
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 1 << 18 | 1 << 26;
        let result = gen_moves_for_pawn(index, &square::Color::White, &bit_board, &bit_board_opponent, &capture_en_passant)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_pawn_white_capture() {
        let index = 18;
        let capture_en_passant: Option<u64> = None;
        let bit_board = bitboard::BitBoard(1 << index | 1 << (index + 8));
        let bit_board_opponent = bitboard::BitBoard(1 << (index + 7) | 1 << (index + 9));
        let expected = 1 << 25 | 1 << 27;
        let result = gen_moves_for_pawn(index, &square::Color::White, &bit_board, &bit_board_opponent, &capture_en_passant)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_pawn_white_row1_col_a() {
        let index = 8;
        let capture_en_passant: Option<u64> = None;
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(1 << 17);
        let expected = 1 << 16 | 1 << 17 | 1 << 24;
        let result = gen_moves_for_pawn(index, &square::Color::White, &bit_board, &bit_board_opponent, &capture_en_passant)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_pawn_black_capture() {
        let index = 50;
        let capture_en_passant: Option<u64> = None;
        let bit_board = bitboard::BitBoard(1 << index | 1 << (index - 8));
        let bit_board_opponent = bitboard::BitBoard(1 << (index - 7) | 1 << (index - 9));
        let expected = 1 << 41 | 1 << 43;
        // bitboard::BitBoard(result)
        println!("\n{}", bit_board_opponent);        
        let result = gen_moves_for_pawn(index, &square::Color::Black, &bit_board, &bit_board_opponent, &capture_en_passant)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_pawn_black_row6() {
        let index = 50;
        let capture_en_passant: Option<u64> = None;
        let bit_board = bitboard::BitBoard(1 << index);
        let bit_board_opponent = bitboard::BitBoard(0);
        let expected = 1 << 42 | 1 << 34;
        let result = gen_moves_for_pawn(index, &square::Color::Black, &bit_board, &bit_board_opponent, &capture_en_passant)
            .unwrap()
            .moves()
            .0;
        assert_eq!(result, expected);
    }
    ////////////////////////////////////////////////////////
    /// Check status
    ////////////////////////////////////////////////////////   
    use board::fen::EncodeUserInput;

    #[test]
    fn test_check_rook() {
        let fen = "knbqrbnr/8/8/8/8/8/8/RNBQKBNR w KQkq - 0 1";
        let position = board::fen::FEN::decode(fen).expect("Failed to decode FEN");
        let check_status = position.check_status();
        let expected = CheckStatus::SimpleCheck { attacker: TypePiece::Rook, attacker_index: 60 };
        assert_eq!(check_status, expected);
    }
    #[test]
    fn test_check_bishop() {
        let fen = "bnbqkbnn/8/8/8/8/8/8/RNBQRBNK w KQkq - 0 1";
        let position = board::fen::FEN::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::SimpleCheck { attacker: TypePiece::Bishop, attacker_index: 56 };
        assert_eq!(check_status, expected);
    }    
    #[test]
    fn test_check_queen() {
        let fen = "qnbbkbnn/8/8/8/8/8/8/RNBQRBNK w KQkq - 0 1";
        let position = board::fen::FEN::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::SimpleCheck { attacker: TypePiece::Queen, attacker_index: 56 };
        assert_eq!(check_status, expected);
    }        
    #[test]
    fn test_check_knight() {
        let fen = "qnbbkbnn/8/8/8/8/8/5nPP/RNBQRBNK w KQkq - 0 1";
        let position = board::fen::FEN::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::SimpleCheck { attacker: TypePiece::Knight, attacker_index: 13 };
        assert_eq!(check_status, expected);
    }            
    #[test]
    fn test_check_pawn() {
        let fen = "qnbbkbnn/8/8/8/8/8/6pP/RNBQRBNK w KQkq - 0 1";
        let position = board::fen::FEN::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::SimpleCheck { attacker: TypePiece::Pawn, attacker_index: 14 };
        assert_eq!(check_status, expected);
    }        
    #[test]
    fn test_double_check() {
        let fen = "bnbqkbnr/8/8/8/8/8/8/RNBQRBNK w KQkq - 0 1";
        let position = board::fen::FEN::decode(fen).expect("Failed to decode FEN");
        println!("{}", position.chessboard());
        let check_status = position.check_status();
        let expected = CheckStatus::DoubleCheck;
        assert_eq!(check_status, expected);
    }    
}
