use super::{coord, fen::{Position, PositionStatus}, square};

pub struct BitPosition {
    bit_board: BitBoard,
    bit_position_status: BitPositionStatus,
}

impl BitPosition {
    pub fn new() -> Self {
        BitPosition {
            bit_board: BitBoard::new(),
            bit_position_status: BitPositionStatus::new(),
        }
    }
}
pub struct BitBoard {
    // TODO

}

impl BitBoard {
    pub fn new() -> Self {
        BitBoard {}
    }
}
pub struct BitPositionStatus {
    flags: u8,
    pawn_en_passant: i8, // 1 byte for the en passant square (-1 if None, 0-63 if Some)
    n_half_moves: u16, 
    n_moves: u16,
}

impl BitPositionStatus {
    const CASTLING_WHITE_QUEEN_SIDE: u8 = 0b0000_0001;
    const CASTLING_WHITE_KING_SIDE: u8 = 0b0000_0010;
    const CASTLING_BLACK_QUEEN_SIDE: u8 = 0b0000_0100;
    const CASTLING_BLACK_KING_SIDE: u8 = 0b0000_1000;
    const PLAYER_TURN_WHITE: u8 = 0b0001_0000;

    pub fn new() -> Self {
        BitPositionStatus {
            flags: 0,
            pawn_en_passant: -1, 
            n_half_moves: 0, 
            n_moves: 0,   
        }
    }
    pub fn castling_white_queen_side(&self) -> bool {
        (self.flags & Self::CASTLING_WHITE_QUEEN_SIDE) != 0
    }
    pub fn castling_white_king_side(&self) -> bool {
        (self.flags & Self::CASTLING_WHITE_KING_SIDE) != 0
    }

    pub fn castling_black_queen_side(&self) -> bool {
        (self.flags & Self::CASTLING_BLACK_QUEEN_SIDE) != 0
    }

    pub fn castling_black_king_side(&self) -> bool {
        (self.flags & Self::CASTLING_BLACK_KING_SIDE) != 0
    }

    pub fn player_turn_white(&self) -> bool {
        (self.flags & Self::PLAYER_TURN_WHITE) != 0
    }

    pub fn pawn_en_passant(&self) -> Option<u8> {
        if self.pawn_en_passant < 0 || self.pawn_en_passant > 63 {
            None
        } else {
            Some(self.pawn_en_passant as u8)
        }
    }    
    
    pub fn n_half_moves(&self) -> u16 {
        self.n_half_moves
    }

    pub fn n_moves(&self) -> u16 {
        self.n_moves
    }

    // Setters
    pub fn set_castling_white_queen_side(&mut self, value: bool) {
        if value {
            self.flags |= Self::CASTLING_WHITE_QUEEN_SIDE;
        } else {
            self.flags &= !Self::CASTLING_WHITE_QUEEN_SIDE;
        }
    }

    pub fn set_castling_white_king_side(&mut self, value: bool) {
        if value {
            self.flags |= Self::CASTLING_WHITE_KING_SIDE;
        } else {
            self.flags &= !Self::CASTLING_WHITE_KING_SIDE;
        }
    }

    pub fn set_castling_black_queen_side(&mut self, value: bool) {
        if value {
            self.flags |= Self::CASTLING_BLACK_QUEEN_SIDE;
        } else {
            self.flags &= !Self::CASTLING_BLACK_QUEEN_SIDE;
        }
    }

    pub fn set_castling_black_king_side(&mut self, value: bool) {
        if value {
            self.flags |= Self::CASTLING_BLACK_KING_SIDE;
        } else {
            self.flags &= !Self::CASTLING_BLACK_KING_SIDE;
        }
    }

    pub fn set_player_turn_white(&mut self, value: bool) {
        if value {
            self.flags |= Self::PLAYER_TURN_WHITE;
        } else {
            self.flags &= !Self::PLAYER_TURN_WHITE;
        }
    }    
    pub fn set_pawn_en_passant(&mut self, value: Option<i8>) {
        self.pawn_en_passant = match value {
            Some(square) if square >= 0 && square <= 63 => square, // Only valid squares (0-63) are allowed
            _ => -1, // If None or invalid square
        };
    }    

    pub fn set_n_half_moves(&mut self, value: u16) {
        self.n_half_moves = value;
    }

    pub fn set_n_moves(&mut self, value: u16) {
        self.n_moves = value;
    }

    pub fn from(status: &PositionStatus) -> Self {
        let mut bp = BitPositionStatus::new();
        bp.set_castling_white_queen_side(status.castling_white_queen_side());
        bp.set_castling_white_king_side(status.castling_white_king_side());
        bp.set_castling_black_queen_side(status.castling_black_queen_side());
        bp.set_castling_black_king_side(status.castling_black_king_side());
        bp.set_player_turn_white(status.player_turn() == square::Color::White);
        bp.set_pawn_en_passant(encode_pawn_en_passant(status.pawn_en_passant()));
        bp.set_n_half_moves(status.n_half_moves());
        bp.set_n_moves(status.n_moves());
        bp
    }

    pub fn to(&self) -> PositionStatus {
        let mut bp = PositionStatus::new();
        bp.set_castling_white_queen_side(self.castling_black_queen_side());
        bp.set_castling_white_king_side(self.castling_white_king_side());
        bp.set_castling_black_queen_side(self.castling_black_queen_side());
        bp.set_castling_black_king_side(self.castling_black_king_side());
        let player_turn = if self.player_turn_white() {
            square::Color::White 
        } 
        else {
            square::Color::Black
        };
        bp.set_player_turn(player_turn);
        bp.set_pawn_en_passant(decode_pawn_en_passant(self.pawn_en_passant()));
        bp.set_n_half_moves(self.n_half_moves());
        bp.set_n_moves(self.n_moves());
        bp
    }
}   

fn encode_pawn_en_passant(maybe_coord: Option<coord::Coord>) -> Option<i8> {
    maybe_coord.map(|coord| {
        (coord.get_y() * 8) as i8 + (coord.get_x() as i8)
    })
}

fn decode_pawn_en_passant(maybe_index: Option<u8>) -> Option<coord::Coord> {
    maybe_index.map_or(None, |index| {
        coord::Coord::from((index % 8 + 65) as char, index / 8 + 1).ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_position_status_from() {
        // Create a PositionStatus with some specific values
        let mut status = PositionStatus::new();
        status.set_castling_white_queen_side(true);
        status.set_castling_white_king_side(false);
        status.set_castling_black_queen_side(true);
        status.set_castling_black_king_side(false);
        status.set_player_turn(square::Color::Black);
        status.set_pawn_en_passant(Some(coord::Coord::from('e', 3).unwrap()));
        status.set_n_half_moves(25);
        status.set_n_moves(50);

        // Convert to BitPositionStatus
        let bit_status = BitPositionStatus::from(&status);

        // Verify the values are correctly set in BitPositionStatus
        assert_eq!(bit_status.castling_white_queen_side(), true);
        assert_eq!(bit_status.castling_white_king_side(), false);
        assert_eq!(bit_status.castling_black_queen_side(), true);
        assert_eq!(bit_status.castling_black_king_side(), false);
        assert_eq!(bit_status.player_turn_white(), false);
        assert_eq!(bit_status.pawn_en_passant(), Some(20)); // e3 -> 20
        assert_eq!(bit_status.n_half_moves(), 25);
        assert_eq!(bit_status.n_moves(), 50);
    }

    #[test]
    fn test_bit_position_status_to() {
        // Create a BitPositionStatus with some specific values
        let mut bit_status = BitPositionStatus::new();
        bit_status.set_castling_white_queen_side(true);
        bit_status.set_castling_white_king_side(false);
        bit_status.set_castling_black_queen_side(true);
        bit_status.set_castling_black_king_side(false);
        bit_status.set_player_turn_white(false);
        bit_status.set_pawn_en_passant(Some(20)); // e3 -> 20
        bit_status.set_n_half_moves(25);
        bit_status.set_n_moves(50);

        // Convert to PositionStatus
        let status = bit_status.to();

        // Verify the values are correctly set in PositionStatus
        assert_eq!(status.castling_white_queen_side(), true);
        assert_eq!(status.castling_white_king_side(), false);
        assert_eq!(status.castling_black_queen_side(), true);
        assert_eq!(status.castling_black_king_side(), false);
        assert_eq!(status.player_turn(), square::Color::Black);
        assert_eq!(status.pawn_en_passant(), Some(coord::Coord::from('e', 3).unwrap()));
        assert_eq!(status.n_half_moves(), 25);
        assert_eq!(status.n_moves(), 50);
    }
}
