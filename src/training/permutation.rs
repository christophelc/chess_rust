use std::collections::HashSet;

use crate::{entity::game::component::{bitboard::{zobrist::{self, ZobristHash}, BitPosition}, game_state::GameState, square::{self, Piece, TypePiece}}, ui::notation::fen::Position};
use itertools::Itertools; 

#[derive(Debug, Clone)]
pub struct Pieces {
    piece: Piece,
    n: u8
}
impl Pieces {
    pub fn new(piece: Piece, n: u8) -> Self {
        assert!(n >= 1, "{}", format!("Invalid number of piece for {}", n));
        assert!(piece.type_piece() != TypePiece::King || n == 1, "Only one king");
        Self {
            piece,
            n
        }
    }
    pub fn piece(&self) -> Piece {
        self.piece
    }
    pub fn n(&self) -> u8 {
        self.n
    }
}

#[derive(Debug)]
pub struct SetPieces {
    pieces: Vec<Piece>
}
impl SetPieces {
    fn check_king(pieces: &Pieces, color: square::Color) -> bool {
        pieces.piece.type_piece() == TypePiece::King && 
        pieces.n() == 1 && 
        pieces.piece.color() == color
    }

    pub fn from(l_pieces: &[Pieces]) -> Self {
        let is_king_white_valid = l_pieces
        .into_iter()
        .find(|iter| Self::check_king(*iter, square::Color::White))
        .is_some();
        let is_king_black_valid = l_pieces
        .into_iter()
        .find(|iter| Self::check_king(*iter, square::Color::Black))
        .is_some();
        assert!(is_king_white_valid, "Missing white king");
        assert!(is_king_black_valid, "Missing black king");
        let distinct_pieces = l_pieces.into_iter().map(|pieces| pieces.piece).collect::<HashSet<Piece>>();
        assert!(distinct_pieces.len() == l_pieces.len(), "Duplicate pieces in set");
        
        let mut set_of_pieces = Vec::<Piece>::new();
        l_pieces.into_iter().for_each(|iter| {
            set_of_pieces.extend(std::iter::repeat(iter.piece().clone()).take(iter.n() as usize))
        });
        Self {
            pieces: set_of_pieces
        }
    }
}

pub struct BitPositionIterator {
    permutations: itertools::Permutations<std::vec::IntoIter<u8>>,
    set: SetPieces,
    zobrist_table: zobrist::Zobrist,
}

// Iterator that generates valid BitPositions from a SetPieces.
// It generates all permutations of piece placements and filters out invalid positions.
// Filtering is done by check_valid_bitboard function.
// Caveat: Duplicate positions may be generated if the SetPieces contains identical pieces
impl BitPositionIterator {
    pub fn new(set: SetPieces) -> Self {
        let piece_count = set.pieces.len();
        let all_squares: Vec<u8> = (0..64).collect();

        let permutations = all_squares.into_iter().permutations(piece_count);

        Self {
            permutations,
            set,
            zobrist_table: zobrist::Zobrist::new(),
        }
    }
}

impl Iterator for BitPositionIterator {
    type Item = BitPosition;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(squares) = self.permutations.next() {
            // we assume it is player turn white.
            let mut bit_position = BitPosition::empty();
            let mut zobrist_hash = ZobristHash::default();

            for (piece, &square) in self.set.pieces.iter().zip(squares.iter()) {
                bit_position.add_piece_without_control(piece.clone(), square, &mut zobrist_hash, &self.zobrist_table);
            }

            if check_valid_bitboard(&bit_position, &self.zobrist_table) {
                return Some(bit_position);
            }
            // else continue
        }
        None
    }
}

pub fn check_valid_bitboard(bit_position: &BitPosition, zobrist_table: &zobrist::Zobrist) -> bool {
    // The other side should not be in check
    let mut bit_position_other_side = bit_position.clone();
    bit_position_other_side.change_side();
    let game_state = GameState::new(bit_position_other_side.to(), zobrist_table);
    let check_status = game_state.check_status();
    !check_status.is_check()
}

pub fn generate_positions_from(set: SetPieces) -> impl Iterator<Item = BitPosition> {
    BitPositionIterator::new(set)
}


#[cfg(test)]
mod tests {
    use crate::{training::permutation::square::Color, ui::notation::fen::{self, EncodeUserInput}};
    use super::*;

    #[test]
fn test_valid_set_with_kings() {
        let white_king = Pieces::new(Piece::new(TypePiece::King, Color::White), 1);
        let black_king = Pieces::new(Piece::new(TypePiece::King, Color::Black), 1);
        let white_pawn = Pieces::new(Piece::new(TypePiece::Pawn, Color::White), 8);
        let black_knight = Pieces::new(Piece::new(TypePiece::Knight, Color::Black), 2);

        let pieces = vec![white_king, black_king, white_pawn, black_knight];

        let set_pieces = SetPieces::from(&pieces);

        // Check number of total pieces
        assert_eq!(set_pieces.pieces.len(), 1 + 1 + 8 + 2);

        // Count number of white kings
        let white_kings = set_pieces.pieces.iter().filter(|p| p.type_piece() == TypePiece::King && p.color() == Color::White).count();
        assert_eq!(white_kings, 1);

        // Count number of black knights
        let black_knights = set_pieces.pieces.iter().filter(|p| p.type_piece() == TypePiece::Knight && p.color() == Color::Black).count();
        assert_eq!(black_knights, 2);
    }

    #[test]
    #[should_panic(expected = "Duplicate pieces in set")]    
    fn test_invalid_set() {
        let white_king = Pieces::new(Piece::new(TypePiece::King, Color::White), 1);
        let black_king = Pieces::new(Piece::new(TypePiece::King, Color::Black), 1);
        let white_pawn = Pieces::new(Piece::new(TypePiece::Pawn, Color::White), 8);        
        let pieces = vec![white_king, black_king, white_pawn.clone(), white_pawn];        
        let set_pieces = SetPieces::from(&pieces);        
    }

    #[test]
    #[should_panic(expected = "Missing white king")]
    fn test_missing_white_king_should_panic() {
        let black_king = Pieces::new(Piece::new(TypePiece::King, Color::Black), 1);
        let black_pawn = Pieces::new(Piece::new(TypePiece::Pawn, Color::Black), 5);
        let pieces = vec![black_king, black_pawn];

        let _ = SetPieces::from(&pieces); // Should panic
    }

    #[test]
    #[should_panic(expected = "Missing black king")]
    fn test_missing_black_king_should_panic() {
        let white_king = Pieces::new(Piece::new(TypePiece::King, Color::White), 1);
        let white_rook = Pieces::new(Piece::new(TypePiece::Rook, Color::White), 2);
        let pieces = vec![white_king, white_rook];

        let _ = SetPieces::from(&pieces); // Should panic
    }

    #[test]
    #[should_panic(expected = "Only one king")]
    fn test_multiple_kings_should_panic() {
        let two_white_kings = Pieces::new(Piece::new(TypePiece::King, Color::White), 2);
        let black_king = Pieces::new(Piece::new(TypePiece::King, Color::Black), 1);
        let pieces = vec![two_white_kings, black_king];

        let _ = SetPieces::from(&pieces); // constructor panic
    }

    #[test]
    #[should_panic(expected = "Duplicate pieces in set")]
    fn test_duplicate_pieces_should_panic() {
        let white_king = Pieces::new(Piece::new(TypePiece::King, Color::White), 1);
        let black_king = Pieces::new(Piece::new(TypePiece::King, Color::Black), 1);
        let pawns = Pieces::new(Piece::new(TypePiece::Pawn, Color::White), 1);
        let duplicate_pawns = Pieces::new(Piece::new(TypePiece::Pawn, Color::White), 1);

        let pieces = vec![white_king, black_king, pawns, duplicate_pawns];

        let _ = SetPieces::from(&pieces); // constructor panic
    }

    #[test]
    fn test_generate_positions_king_rook_vs_king() {
        let white_king = Piece::new(TypePiece::King, Color::White);
        let white_rook = Piece::new(TypePiece::Rook, Color::White);
        let black_king = Piece::new(TypePiece::King, Color::Black);

        let set = SetPieces {
            pieces: vec![white_king, white_rook, black_king],
        };

        let mut positions = generate_positions_from(set);
        
        // Take the first valid position
        let first = positions.next();
        assert!(first.is_some(), "No valid position generated");

        let bit_position = first.unwrap();
        let white_pieces = bit_position
            .bit_boards_white_and_black()
            .bit_board_white()
            .concat_bit_boards();
        let black_pieces = bit_position
            .bit_boards_white_and_black()
            .bit_board_black()
            .concat_bit_boards();
        assert_eq!(white_pieces.count_ones(), 2, "Expected 2 white pieces");
        assert_eq!(black_pieces.count_ones(), 1, "Expected 1 black piece");
    }

    #[test]
    fn test_invalid_position_kings_too_close() {
        let zobrist_table = zobrist::Zobrist::new();
        let fen_invalid = "Kk6/8/8/8/8/8/8/R7 w q - 0 1"; 
        let position = fen::Fen::decode(fen_invalid).expect("Failed to decode FEN");
        let bit_position = &BitPosition::from(position);
        let is_valid = check_valid_bitboard(bit_position, &zobrist_table);
        assert!(!is_valid);
    }
    #[test]
    fn test_invalid_position_opponent_in_check() {
        let zobrist_table = zobrist::Zobrist::new();
        let fen_invalid = "K1k5/8/8/8/8/8/8/2R5 w - - 0 1";
        let position = fen::Fen::decode(fen_invalid).expect("Failed to decode FEN");
        let bit_position = &BitPosition::from(position);
        let is_valid = check_valid_bitboard(bit_position, &zobrist_table);
        assert!(!is_valid);
    }
}