/// Zobrish hash
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::fmt;

use crate::entity::game::component::{bitboard, square};

#[derive(Debug, Clone)]
pub struct Zobrist {
    piece_square: [[u64; 64]; 12], // 6 pièces * 2 colors, 64 squares
    castling_rights: [u64; 4],
    en_passant: [u64; 64],
    side_to_move: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ZobristHistory {
    hashes: Vec<ZobristHash>,
}
impl fmt::Display for ZobristHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use the iterator and join the formatted hashes with commas
        let formatted_hashes: Vec<String> =
            self.hashes.iter().map(|hash| format!("{}", hash)).collect();
        write!(f, "[{}]", formatted_hashes.join(", "))
    }
}
impl ZobristHistory {
    // look for the same position 3x between last position and (last - n_half_moves) position
    pub fn check_3x(&self, n_half_moves: u16) -> bool {
        match self.hashes.last() {
            _ if n_half_moves < 8 => false,
            Some(hash) => {
                let len = self.hashes.len();
                let start_index = len.saturating_sub((n_half_moves + 1) as usize); // Avoids underflow
                self.hashes[start_index..]
                    .iter()
                    .step_by(2)
                    .filter(|&h| *h == *hash)
                    .count()
                    >= 3
            }
            _ => false,
        }
    }
    pub fn list(&self) -> &Vec<ZobristHash> {
        &self.hashes
    }
    pub fn push(&mut self, hash: ZobristHash) {
        self.hashes.push(hash);
    }
    pub fn pop(&mut self) {
        assert!(!self.hashes.is_empty());
        self.hashes.pop();
    }
}
impl PartialEq for ZobristHistory {
    fn eq(&self, other: &Self) -> bool {
        self.hashes.last() == other.hashes.last()
    }
}

impl Default for Zobrist {
    fn default() -> Self {
        Zobrist {
            piece_square: [[0; 64]; 12],
            castling_rights: [0; 4],
            en_passant: [0; 64],
            side_to_move: 0,
        }
    }
}

impl Zobrist {
    pub fn init(mut self) -> Zobrist {
        // seed for debug
        let seed: u64 = 123;
        let mut rng = StdRng::seed_from_u64(seed);
        // Generate random values for each piece and square
        for piece in 0..12 {
            for square in 0..64 {
                self.piece_square[piece][square] = rng.gen();
            }
        }
        // Generate values for castle rights and en passant capture
        for i in 0..4 {
            self.castling_rights[i] = rng.gen();
        }
        for i in 0..64 {
            self.en_passant[i] = rng.gen();
        }
        // player turn
        self.side_to_move = rng.gen();
        self
    }
    pub fn new() -> Self {
        Zobrist::default().init()
    }
}

fn piece_to_index(piece: square::Piece) -> usize {
    let idx = piece.type_piece() as usize;
    match piece.color() {
        square::Color::White => idx,
        square::Color::Black => idx + 6,
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ZobristHash(u64);

impl fmt::Display for ZobristHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:0x}", self.0)
    }
}

impl ZobristHash {
    pub fn value(&self) -> u64 {
        self.0
    }
    pub fn zobrist_hash_from_position(
        bit_position: &bitboard::BitPosition,
        zobrist: &Zobrist,
    ) -> Self {
        let mut zobrist_hash = ZobristHash::default();
        let bit_boards_white_and_black = bit_position.bit_boards_white_and_black();
        let status = bit_position.bit_position_status();

        // Calculer le hash des bitboards
        for square_idx in 0..64 {
            if let square::Square::NonEmpty(piece) =
                bit_boards_white_and_black.peek(bitboard::BitIndex::new(square_idx))
            {
                zobrist_hash = zobrist_hash.xor_piece(zobrist, piece, square_idx as usize)
            }
        }

        if status.castling_white_king_side() {
            zobrist_hash = zobrist_hash.xor_castling_white_king_side(zobrist);
        }
        if status.castling_white_queen_side() {
            zobrist_hash = zobrist_hash.xor_castling_white_queen_side(zobrist);
        }
        if status.castling_black_king_side() {
            zobrist_hash = zobrist_hash.xor_castling_black_king_side(zobrist);
        }
        if status.castling_black_queen_side() {
            zobrist_hash = zobrist_hash.xor_castling_black_queen_side(zobrist);
        }

        if let Some(ep_square) = status.pawn_en_passant() {
            zobrist_hash = zobrist_hash.xor_en_passant(ep_square, zobrist);
        }

        if !status.player_turn_white() {
            zobrist_hash = zobrist_hash.xor_player_turn(zobrist)
        }

        zobrist_hash
    }

    pub fn xor_piece(
        &self,
        zobrist: &Zobrist,
        piece_add_or_remove: square::Piece,
        square_idx: usize,
    ) -> Self {
        let hash = self.0;
        let piece_index = piece_to_index(piece_add_or_remove);
        ZobristHash(hash ^ zobrist.piece_square[piece_index][square_idx])
    }
    pub fn xor_castling_white_king_side(&self, zobrist: &Zobrist) -> Self {
        let hash = self.0;
        ZobristHash(hash ^ zobrist.castling_rights[0])
    }
    pub fn xor_castling_white_queen_side(&self, zobrist: &Zobrist) -> Self {
        let hash = self.0;
        ZobristHash(hash ^ zobrist.castling_rights[1])
    }
    pub fn xor_castling_black_king_side(&self, zobrist: &Zobrist) -> Self {
        let hash = self.0;
        ZobristHash(hash ^ zobrist.castling_rights[2])
    }
    pub fn xor_castling_black_queen_side(&self, zobrist: &Zobrist) -> Self {
        let hash = self.0;
        ZobristHash(hash ^ zobrist.castling_rights[3])
    }
    pub fn xor_en_passant(&self, ep_square: bitboard::BitIndex, zobrist: &Zobrist) -> Self {
        let hash = self.0;
        ZobristHash(hash ^ zobrist.en_passant[ep_square.value() as usize])
    }
    pub fn xor_player_turn(&self, zobrist: &Zobrist) -> Self {
        let hash = self.0;
        ZobristHash(hash ^ zobrist.side_to_move)
    }
}

#[cfg(test)]
mod tests {
    use super::Zobrist;

    #[test]
    fn test_zobrist_check() {
        let zobrist = Zobrist::new();
        assert!(zobrist.castling_rights.iter().any(|elem| *elem != 0));
        assert!(zobrist.en_passant.iter().any(|elem| *elem != 0));
        assert_ne!(zobrist.side_to_move, 0);
        assert!(zobrist
            .piece_square
            .iter()
            .flat_map(|piece| piece.iter())
            .any(|elem| *elem != 0));
    }
}
