use crate::board::{bitboard, coord, square};

pub trait GenMoves {
    fn gen_moves(
        &self,
        bit_board_type_piece: &bitboard::BitBoard,
        bit_board: &bitboard::BitBoard,
        bit_board_opponent: &bitboard::BitBoard,
    ) -> Vec<PieceMoves>;
    fn gen_moves_for_piece(
        &self,
        index: u8,
        bit_board: &bitboard::BitBoard,
        bit_board_opponent: &bitboard::BitBoard,
    ) -> Option<PieceMoves>;
}

impl GenMoves for square::TypePiece {
    /// gen moves for one piece at index
    fn gen_moves_for_piece(
        &self,
        index: u8,
        bit_board: &bitboard::BitBoard,
        bit_board_opponent: &bitboard::BitBoard,
    ) -> Option<PieceMoves> {
        match self {
            &square::TypePiece::Rook => None,
            &square::TypePiece::Bishop => None,
            &square::TypePiece::Knight => None,
            &square::TypePiece::King => gen_moves_for_king(index, bit_board, bit_board_opponent),
            &square::TypePiece::Queen => None,
            &square::TypePiece::Pawn => None,
        }
    }
    // gen moves for all piece of one type
    fn gen_moves(
        &self,
        bit_board_type_piece: &bitboard::BitBoard,
        bit_board: &bitboard::BitBoard,
        bit_board_opponent: &bitboard::BitBoard,
    ) -> Vec<PieceMoves> {
        let mut moves = Vec::new();
        let mut bb = bit_board_type_piece.0;
        while bb != 0 {
            let lsb = bb.trailing_zeros();
            if let(Some(moves_for_piece)) = self.gen_moves_for_piece(lsb as u8, bit_board, bit_board_opponent) {
                moves.push(moves_for_piece);
            }
            bb &= bb - 1; // Remove lsb
        }
        moves
    }
}

// moves generation are not optimized (as a first implementation)
fn gen_moves_for_king(
    index: u8,
    bit_board: &bitboard::BitBoard,
    bit_board_opponent: &bitboard::BitBoard,
) -> Option<PieceMoves> {
    let is_row_1 = index < 8;
    let is_col_A = index % 8 == 0;
    let is_row_8 = index >= 56;
    let is_col_H = index % 8 == 7;
    let deltas: Vec<i8> = match (is_row_1, is_col_A, is_row_8, is_col_H) {
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
            if (bit_board.0 & (1 << pos) == 0) || (bit_board_opponent.0 & (1 << pos) != 0) {
                moves_bitboard |= 1 << pos; // Set the bit for a valid move
            }
        } else {
            panic!("This code should never be reached.")
        }
    }

    if moves_bitboard == 0 {
        None
    } else {
        Some(PieceMoves {
            index,
            moves: bitboard::BitBoard(moves_bitboard),
        })
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

// POC
fn list_index(bit_board: &bitboard::BitBoard) -> Vec<u8> {
    let mut v = Vec::new();
    let mut bb = bit_board.0;
    while bb != 0 {
        let lsb = bb.trailing_zeros();
        v.push(lsb as u8);
        //perform_action(lsb as usize);

        bb &= bb - 1; // Remove lsb
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitboard::BitBoard;

    #[test]
    fn test_single_bit() {
        assert_eq!(bitboard::BitBoard(1 << 5).trailing_zeros(), 5);
        assert_eq!((bitboard::BitBoard(1 << 0)).trailing_zeros(), 0);
        assert_eq!((bitboard::BitBoard(1 << 15)).trailing_zeros(), 15);
    }

    #[test]
    fn test_zero_value() {
        assert_eq!(bitboard::BitBoard(0u64).trailing_zeros(), 64);
    }

    #[test]
    fn test_multiple_bits() {
        let value = bitboard::BitBoard((1 << 5) | (1 << 3));
        assert_eq!(value.trailing_zeros(), 3);
    }

    #[test]
    fn test_highest_bit() {
        assert_eq!((bitboard::BitBoard(1u64 << 63)).trailing_zeros(), 63);
    }

    #[test]
    fn test_empty_bitboard() {
        let bitboard = BitBoard(0);
        assert_eq!(list_index(&bitboard), vec![]);
    }

    #[test]
    fn test_list_index_single_bit() {
        let bitboard = BitBoard(1 << 5); // bit at position 5
        assert_eq!(list_index(&bitboard), vec![5]);
    }

    #[test]
    fn test_list_index_multiple_bits() {
        let bitboard = BitBoard((1 << 5) | (1 << 15) | (1 << 30)); // bits at positions 5, 15, 30
        let mut result = list_index(&bitboard);
        result.sort(); // Sorting the result to ensure order for comparison
        assert_eq!(result, vec![5, 15, 30]);
    }

    #[test]
    fn test_list_index_bits_at_edges() {
        let bitboard = BitBoard((1 << 0) | (1 << 63)); // bits at positions 0 and 63
        let mut result = list_index(&bitboard);
        result.sort(); // Sorting to ensure consistent order
        assert_eq!(result, vec![0, 63]);
    }

    #[test]
    fn test_king_center_moves() {
        let king_position = 27; // Somewhere in the center of the board
        let bit_board = BitBoard(0); // No friendly pieces blocking
        let bit_board_opponent = BitBoard(0); // No opponent pieces
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent).unwrap();
        assert_eq!(result.index, king_position);
        assert_eq!(result.moves.0, 0x1C141C0000); // Expected moves bitboard for center position
    }

    #[test]
    fn test_king_edge_moves() {
        let king_position = 8; // On the edge (A file)
        let bit_board = BitBoard(0);
        let bit_board_opponent = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent).unwrap();
        assert_eq!(result.index, king_position);
        let expected_moves = (1 << 0) | (1 << 1) | (1 << 9) | (1 << 16) | (1 << 17);
        assert_eq!(result.moves.0, expected_moves); // Expected moves bitboard for an edge position
    }

    #[test]
    fn test_king_corner_moves() {
        let king_position = 0; // Top left corner (A1)
        let bit_board = BitBoard(0);
        let bit_board_opponent = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent).unwrap();
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
        let bit_board_opponent = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent);
        assert!(result.is_none()); // Expect no moves available
    }

    #[test]
    #[should_panic]
    fn test_invalid_king_position() {
        let king_position = 64; // Invalid position
        let bit_board = BitBoard(0);
        let bit_board_opponent = BitBoard(0);
        let _ = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent);
    }
    #[test]
    fn test_king_corner_h1_moves() {
        let king_position = 7; // Top right corner (H1)
        let bit_board = BitBoard(0);
        let bit_board_opponent = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent).unwrap();
        let expected_moves = (1 << 6) | (1 << 14) | (1 << 15); // Moves: G1, H2, G2
        assert_eq!(result.moves.0, expected_moves);
    }

    // Test for the bottom-left corner (A8)
    #[test]
    fn test_king_corner_a8_moves() {
        let king_position = 56; // Bottom left corner (A8)
        let bit_board = BitBoard(0);
        let bit_board_opponent = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent).unwrap();
        let expected_moves = (1 << 48) | (1 << 49) | (1 << 57); // Moves: A7, B7, B8
        assert_eq!(result.moves.0, expected_moves);
    }

    // Test for the bottom-right corner (H8)
    #[test]
    fn test_king_corner_h8_moves() {
        let king_position = 63; // Bottom right corner (H8)
        let bit_board = BitBoard(0);
        let bit_board_opponent = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent).unwrap();
        let expected_moves = (1 << 62) | (1 << 54) | (1 << 55); // Moves: G8, H7, G7
        assert_eq!(result.moves.0, expected_moves);
    }

    // Test for an arbitrary position in row 1 (B1)
    #[test]
    fn test_king_row1_b1_moves() {
        let king_position = 1; // B1
        let bit_board = BitBoard(0);
        let bit_board_opponent = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent).unwrap();
        let expected_moves = (1 << 0) | (1 << 2) | (1 << 8) | (1 << 9) | (1 << 10); // Moves: A1, C1, A2, B2, C2
        assert_eq!(result.moves.0, expected_moves);
    }

    // Test for an arbitrary position in row 8 (G8)
    #[test]
    fn test_king_row8_g8_moves() {
        let king_position = 62; // G8
        let bit_board = BitBoard(0);
        let bit_board_opponent = BitBoard(0);
        let result = gen_moves_for_king(king_position, &bit_board, &bit_board_opponent).unwrap();
        let expected_moves = (1 << 61) | (1 << 63) | (1 << 53) | (1 << 54) | (1 << 55); // Moves: F8, H8, F7, G7, H7
        assert_eq!(result.moves.0, expected_moves);
    }
}
