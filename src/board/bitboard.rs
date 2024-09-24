pub mod piece_move;

use super::{
    coord,
    fen::{Position, PositionStatus},
    square::{self, Switch, TypePiecePromotion},
    ChessBoard,
};
use piece_move::table;
use std::{
    fmt, ops::BitAnd, ops::BitOr, ops::BitOrAssign, ops::BitXor, ops::Not, ops::Shl, ops::Shr,
};

#[derive(Debug)]
pub enum Castle {
    Short,
    Long,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct BitBoardMove {
    color: square::Color,
    type_piece: TypePiece,
    start: BitIndex,
    end: BitIndex,
    capture: Option<TypePiece>,
    promotion: Option<TypePiecePromotion>,
}
impl BitBoardMove {
    pub fn new(
        color: Color,
        type_piece: TypePiece,
        start: BitIndex,
        end: BitIndex,
        capture: Option<TypePiece>,
        promotion: Option<TypePiecePromotion>,
    ) -> Self {
        BitBoardMove {
            color,
            type_piece,
            start,
            end,
            capture,
            promotion,
        }
    }
    pub fn start(&self) -> BitIndex {
        self.start
    }
    pub fn end(&self) -> BitIndex {
        self.end
    }
    pub fn type_piece(&self) -> TypePiece {
        self.type_piece
    }
    pub fn capture(&self) -> Option<TypePiece> {
        self.capture
    }
    pub fn promotion(&self) -> Option<TypePiecePromotion> {
        self.promotion
    }
    pub fn is_capture_en_passant(&self) -> bool {
        self.type_piece == TypePiece::Pawn
            && self.capture.is_none()
            && (self.start.col() != self.end.col())
    }
    pub fn check_castle(&self) -> Option<Castle> {
        let mut castle: Option<Castle> = None;
        if self.type_piece() == TypePiece::King {
            let delta_col = (self.end.0 as i8) - (self.start.0 as i8);
            if delta_col == 2 {
                castle = Some(Castle::Short);
            };
            if delta_col == -2 {
                castle = Some(Castle::Long);
            }
        }
        castle
    }
    pub fn from(
        color: Color,
        type_piece: TypePiece,
        start: BitIndex,
        end: BitIndex,
        bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
    ) -> Vec<Self> {
        let bit_boards_opponent = bit_boards_white_and_black.bit_board(&color.switch());
        let b_to: BitBoard = end.bitboard();
        let mut capture: Option<TypePiece> = None;
        if (*bit_boards_opponent.rooks().bitboard() & b_to).non_empty() {
            capture = Some(TypePiece::Rook);
        } else if (*bit_boards_opponent.bishops().bitboard() & b_to).non_empty() {
            capture = Some(TypePiece::Bishop);
        } else if (*bit_boards_opponent.knights().bitboard() & b_to).non_empty() {
            capture = Some(TypePiece::Knight);
        } else if (*bit_boards_opponent.queens().bitboard() & b_to).non_empty() {
            capture = Some(TypePiece::Queen);
        } else if (*bit_boards_opponent.pawns().bitboard() & b_to).non_empty() {
            capture = Some(TypePiece::Pawn);
        // should not be possible except in Blitz
        } else if (*bit_boards_opponent.king().bitboard() & b_to).non_empty() {
            capture = Some(TypePiece::King);
        }
        if type_piece == TypePiece::Pawn
            && ((color == Color::White
                && (end.bitboard() & BitBoard(table::MASK_ROW_7)).non_empty())
                || (color == Color::Black
                    && (end.bitboard() & BitBoard(table::MASK_ROW_0)).non_empty()))
        {
            vec![
                Self::new(
                    color,
                    type_piece,
                    start,
                    end,
                    capture,
                    Some(TypePiecePromotion::Rook),
                ),
                Self::new(
                    color,
                    type_piece,
                    start,
                    end,
                    capture,
                    Some(TypePiecePromotion::Bishop),
                ),
                Self::new(
                    color,
                    type_piece,
                    start,
                    end,
                    capture,
                    Some(TypePiecePromotion::Knight),
                ),
                Self::new(
                    color,
                    type_piece,
                    start,
                    end,
                    capture,
                    Some(TypePiecePromotion::Queen),
                ),
            ]
        } else {
            vec![Self::new(color, type_piece, start, end, capture, None)]
        }
    }
}

#[derive(PartialEq)]
pub struct BitPosition {
    bit_boards_white_and_black: BitBoardsWhiteAndBlack,
    bit_position_status: BitPositionStatus,
}

fn pos2index(u: u64) -> u8 {
    u.trailing_zeros() as u8
}

impl BitPosition {
    pub fn move_piece(self, b_move: &BitBoardMove) -> BitPosition {
        let bit_boards_white_and_black = self.bit_boards_white_and_black.move_piece(b_move);
        let bit_board_pawn_opponent = match b_move.color {
            Color::White => *bit_boards_white_and_black.bit_board_black.pawns.bitboard(),
            Color::Black => *bit_boards_white_and_black.bit_board_white.pawns.bitboard(),
        };
        BitPosition {
            bit_boards_white_and_black,
            bit_position_status: update_status(
                b_move,
                &bit_board_pawn_opponent,
                self.bit_position_status,
            ),
        }
    }
    pub fn bit_boards_white_and_black(&self) -> &BitBoardsWhiteAndBlack {
        &self.bit_boards_white_and_black
    }
    pub fn bit_position_status(&self) -> &BitPositionStatus {
        &self.bit_position_status
    }
    pub fn from(position: Position) -> Self {
        let bit_position_status = BitPositionStatus::from(position.status());
        let bit_position = BitBoardsWhiteAndBlack::from(position.into_chessboard());
        BitPosition {
            bit_boards_white_and_black: bit_position,
            bit_position_status,
        }
    }
    pub fn to(&self) -> Position {
        let chessboard = self.bit_boards_white_and_black.to();
        let status = self.bit_position_status.to();
        Position::build(chessboard, status)
    }
}

fn update_status(
    b_move: &BitBoardMove,
    bit_board_pawn_opponent: &BitBoard,
    bit_position_status: BitPositionStatus,
) -> BitPositionStatus {
    let mut bit_position_status = bit_position_status;
    // move of a rook
    match b_move.type_piece {
        TypePiece::Rook => {
            if b_move.start.0 == 1 || b_move.start.0 == 56 {
                bit_position_status.set_castling_queen_side(b_move.color, false)
            }
            if b_move.start.0 == 7 || b_move.start.0 == 63 {
                bit_position_status.set_castling_king_side(b_move.color, false)
            }
        }
        TypePiece::King => bit_position_status.disable_castling(b_move.color),
        TypePiece::Pawn => {
            let mut capture_en_passant: Option<i8> = None;
            let dir: i8 = if b_move.color == Color::White { 8 } else { -8 };
            if b_move.start.0 as i8 + dir + dir == b_move.end.0 as i8
                && (*bit_board_pawn_opponent & b_move.end.right().bitboard()).non_empty()
                || (*bit_board_pawn_opponent & b_move.end.left().bitboard()).non_empty()
            {
                capture_en_passant = Some(b_move.start.0 as i8 + dir);
            }
            bit_position_status.set_pawn_en_passant(capture_en_passant);
        }
        _ => {}
    }
    // Change player turn
    bit_position_status.set_player_turn_white(b_move.color == Color::Black);
    // n_moves
    if b_move.color == Color::Black {
        bit_position_status.inc_n_moves();
    }
    // half_moves
    if b_move.capture.is_none() && b_move.type_piece != TypePiece::Pawn {
        bit_position_status.inc_n_half_moves();
    } else {
        bit_position_status.reset_n_half_moves();
    }

    bit_position_status
}

#[derive(Debug, PartialEq, Clone)]
pub struct BitBoardsWhiteAndBlack {
    bit_board_white: BitBoards,
    bit_board_black: BitBoards,
}

impl BitBoardsWhiteAndBlack {
    pub fn peek(&self, index: BitIndex) -> Square {
        let bit_square = index.bitboard();
        let mut square: Square = Square::Empty;
        if (*self.bit_board_white.rooks.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Rook, Color::White)
        };
        if (*self.bit_board_white.bishops.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Bishop, Color::White)
        };
        if (*self.bit_board_white.knights.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Knight, Color::White)
        };
        if (*self.bit_board_white.queens.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Queen, Color::White)
        };
        if (*self.bit_board_white.king.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::King, Color::White)
        };
        if (*self.bit_board_white.pawns.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Pawn, Color::White)
        };
        if (*self.bit_board_black.rooks.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Rook, Color::Black)
        };
        if (*self.bit_board_black.bishops.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Bishop, Color::Black)
        };
        if (*self.bit_board_black.knights.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Knight, Color::Black)
        };
        if (*self.bit_board_black.queens.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Queen, Color::Black)
        };
        if (*self.bit_board_black.king.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::King, Color::Black)
        };
        if (*self.bit_board_black.pawns.bitboard() & bit_square).non_empty() {
            square = Square::build_piece(TypePiece::Pawn, Color::Black)
        };
        square
    }
    pub fn remove_piece(
        self,
        color: &square::Color,
        type_piece: TypePiece,
        index: BitIndex,
    ) -> BitBoardsWhiteAndBlack {
        let mask_remove = index.bitboard();
        match color {
            square::Color::White => BitBoardsWhiteAndBlack {
                bit_board_white: self.bit_board_white.remove_piece(type_piece, mask_remove),
                ..self
            },
            square::Color::Black => BitBoardsWhiteAndBlack {
                bit_board_black: self.bit_board_black.remove_piece(type_piece, mask_remove),
                ..self
            },
        }
    }
    pub fn move_piece(self, b_move: &BitBoardMove) -> BitBoardsWhiteAndBlack {
        let mut mask_remove = BitBoard::default();
        if b_move.capture.is_some() {
            mask_remove = b_move.end.bitboard();
        } else if b_move.type_piece == TypePiece::Pawn && b_move.start.col() != b_move.end.col() {
            // en passant
            mask_remove = BitIndex(b_move.start.first_col().0 + b_move.end.col()).bitboard();
        };
        let new_bitboards = match b_move.color {
            square::Color::White => BitBoardsWhiteAndBlack {
                bit_board_white: self.bit_board_white.move_piece(
                    b_move.type_piece,
                    b_move.start,
                    b_move.end,
                    b_move.promotion,
                ),
                bit_board_black: if mask_remove.non_empty() {
                    let capture = b_move.capture.unwrap_or(square::TypePiece::Pawn);
                    self.bit_board_black.remove_piece(capture, mask_remove)
                } else {
                    self.bit_board_black
                },
            },
            square::Color::Black => BitBoardsWhiteAndBlack {
                bit_board_black: self.bit_board_black.move_piece(
                    b_move.type_piece,
                    b_move.start,
                    b_move.end,
                    b_move.promotion,
                ),
                bit_board_white: if mask_remove.non_empty() {
                    let capture = b_move.capture.unwrap_or(square::TypePiece::Pawn);
                    self.bit_board_white.remove_piece(capture, mask_remove)
                } else {
                    self.bit_board_white
                },
            },
        };
        match b_move.check_castle() {
            Some(Castle::Short) => {
                let b_move = BitBoardMove {
                    type_piece: TypePiece::Rook,
                    start: b_move.end().right(),
                    end: b_move.end().left(),
                    ..*b_move
                };
                new_bitboards.move_piece(&b_move)
            }
            Some(Castle::Long) => {
                let b_move = BitBoardMove {
                    type_piece: TypePiece::Rook,
                    start: b_move.end().left().left(),
                    end: b_move.end().right(),
                    ..*b_move
                };
                new_bitboards.move_piece(&b_move)
            }
            None => new_bitboards,
        }
    }
    pub fn bit_board(&self, color: &Color) -> &BitBoards {
        match color {
            Color::White => self.bit_board_white(),
            Color::Black => self.bit_board_black(),
        }
    }
    pub fn bit_board_white(&self) -> &BitBoards {
        &self.bit_board_white
    }
    pub fn bit_board_black(&self) -> &BitBoards {
        &self.bit_board_black
    }
    pub fn from(board: ChessBoard) -> Self {
        let mut bit_board_white = BitBoards::new();
        let mut bit_board_black = BitBoards::new();
        for (idx, square) in board.iter().enumerate() {
            match square {
                square::Square::NonEmpty(piece) => {
                    let bd: &mut BitBoards = match piece.color() {
                        square::Color::White => &mut bit_board_white,
                        square::Color::Black => &mut bit_board_black,
                    };
                    let byte = 1u64 << (idx as u8);
                    match piece.type_piece() {
                        square::TypePiece::Rook => bd.rooks |= byte,
                        square::TypePiece::Bishop => bd.bishops |= byte,
                        square::TypePiece::Knight => bd.knights |= byte,
                        square::TypePiece::King => bd.king |= byte,
                        square::TypePiece::Queen => bd.queens |= byte,
                        square::TypePiece::Pawn => bd.pawns |= byte,
                    }
                }
                square::Square::Empty => {}
            }
        }
        BitBoardsWhiteAndBlack {
            bit_board_white,
            bit_board_black,
        }
    }
    pub fn to(&self) -> ChessBoard {
        let mut chessboard = ChessBoard::new();
        for type_piece in TypePiece::ALL {
            let bitboard = self.bit_board_white.get_bitboard(type_piece);
            for coord in bitboard.list_non_empty_squares() {
                chessboard.add(coord, type_piece, Color::White);
            }
            let bitboard = self.bit_board_black.get_bitboard(type_piece);
            for coord in bitboard.list_non_empty_squares() {
                chessboard.add(coord, type_piece, Color::Black);
            }
        }
        chessboard
    }
}

#[derive(Debug, PartialEq)]
pub enum Direction {
    BishopTopLeftBottomRight,
    BishopBottomLeftTopRight,
    RookHorizontal,
    RookVertical,
    None,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct BitIndex(u8);
impl BitIndex {
    pub fn bitboard(&self) -> BitBoard {
        BitBoard::new(1u64 << self.0)
    }
    pub fn new(index: u8) -> Self {
        BitIndex(index)
    }
    #[cfg(test)]
    pub fn union(indexes: Vec<u8>) -> Vec<Self> {
        indexes.iter().map(|idx| Self::new(*idx)).collect()
    }
    pub fn direction(&self, to: BitIndex) -> Direction {
        let index = (to.0 as i8 - self.0 as i8).abs();
        match index {
            _ if index % 7 == 0 => Direction::BishopTopLeftBottomRight,
            _ if index % 9 == 0 => Direction::BishopBottomLeftTopRight,
            _ if index % 8 == 0 => Direction::RookVertical,
            _ if to.row() == self.row() => Direction::RookHorizontal,
            _ => Direction::None,
        }
    }
    pub fn first_col(&self) -> BitIndex {
        BitIndex(self.0 - self.col())
    }
    pub fn right(&self) -> BitIndex {
        BitIndex(self.0 + 1)
    }
    pub fn left(&self) -> BitIndex {
        BitIndex(self.0 - 1)
    }
    pub fn up(&self) -> BitIndex {
        BitIndex(self.0 + 8)
    }
    pub fn upx2(&self) -> BitIndex {
        BitIndex(self.0 + 16)
    }
    pub fn down(&self) -> BitIndex {
        BitIndex(self.0 - 8)
    }
    pub fn downx2(&self) -> BitIndex {
        BitIndex(self.0 - 16)
    }
    pub fn row(&self) -> u8 {
        self.0 / 8
    }
    pub fn col(&self) -> u8 {
        self.0 % 8
    }
    pub fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct BitBoard(u64);
pub struct BitIterator {
    bitboard: BitBoard,
}
impl Iterator for BitIterator {
    type Item = BitIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let bb = self.bitboard.value();
        if bb != 0 {
            let lsb = self.bitboard.index();
            self.bitboard = BitBoard(bb & (bb - 1));
            Some(lsb)
        } else {
            None
        }
    }
}

impl BitBoard {
    pub fn remove(&self, mask_remove: BitBoard) -> BitBoard {
        *self ^ mask_remove
    }
    fn switch(&self, mask_switch: BitBoard, mask_promotion: BitBoard) -> Self {
        (*self ^ mask_switch) | mask_promotion
    }
    pub fn iter(&self) -> BitIterator {
        BitIterator {
            bitboard: BitBoard::new(self.0),
        }
    }
    pub fn value(&self) -> u64 {
        self.0
    }
    pub fn non_empty(&self) -> bool {
        self.0 != 0
    }
    pub fn empty(&self) -> bool {
        self.0 == 0
    }
    // contains zero or one piece max
    pub fn one_bit_set_max(&self) -> bool {
        let one_bit_set = self.non_empty() && (self.0 & (self.0 - 1)) == 0;
        one_bit_set || self.empty()
    }
    pub fn index(&self) -> BitIndex {
        BitIndex::new(pos2index(self.0))
    }

    pub fn new(value: u64) -> Self {
        BitBoard(value)
    }

    #[cfg(test)]
    fn build(vec: Vec<BitIndex>) -> Self {
        vec.into_iter()
            .fold(Self::default(), |acc, index| acc | index.bitboard())
    }

    fn list_non_empty_squares(&self) -> Vec<coord::Coord> {
        let mut coords = Vec::new();
        for i in (0..8).rev() {
            // iterate over the ranks in reverse (from 7 to 0)
            for j in 0..8 {
                let index = i * 8 + j;
                let bit = (self.0 >> index) & 1;
                if bit == 1 {
                    coords.push(coord::Coord::from((j + b'A') as char, i + 1).unwrap())
                }
            }
        }
        coords
    }
}
impl fmt::Display for BitBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::new();
        for rank in (0..8).rev() {
            // iterate over the ranks in reverse (from 7 to 0)
            for file in 0..8 {
                let index = rank * 8 + file;
                let bit = (self.0 >> index) & 1;
                output.push_str(&format!("{} ", bit));
            }
            output.pop(); // Remove the trailing space
            output.push('\n');
        }
        write!(f, "{}", output)
    }
}
impl BitOrAssign<BitBoard> for BitBoard {
    fn bitor_assign(&mut self, rhs: BitBoard) {
        self.0 |= rhs.0;
    }
}
impl BitOr for BitBoard {
    type Output = BitBoard;

    fn bitor(self, rhs: BitBoard) -> BitBoard {
        BitBoard(self.0 | rhs.0)
    }
}

impl BitAnd for BitBoard {
    type Output = BitBoard;

    fn bitand(self, rhs: BitBoard) -> BitBoard {
        BitBoard(self.0 & rhs.0)
    }
}
impl BitXor for BitBoard {
    type Output = BitBoard;

    fn bitxor(self, rhs: BitBoard) -> BitBoard {
        BitBoard(self.0 ^ rhs.0)
    }
}
impl Shl<u8> for BitBoard {
    type Output = BitBoard;

    fn shl(self, rhs: u8) -> BitBoard {
        BitBoard(self.0 << rhs)
    }
}
impl Shl<BitIndex> for BitBoard {
    type Output = BitBoard;

    fn shl(self, rhs: BitIndex) -> BitBoard {
        BitBoard(self.0 << rhs.0)
    }
}
impl Shr<u8> for BitBoard {
    type Output = BitBoard;

    fn shr(self, rhs: u8) -> BitBoard {
        BitBoard(self.0 >> rhs)
    }
}
impl Shr<BitIndex> for BitBoard {
    type Output = BitBoard;

    fn shr(self, rhs: BitIndex) -> BitBoard {
        BitBoard(self.0 >> rhs.0)
    }
}
impl Not for BitBoard {
    type Output = BitBoard;

    fn not(self) -> BitBoard {
        BitBoard(!self.0)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BitBoards {
    rooks: piece_move::RooksBitBoard,
    bishops: piece_move::BishopsBitBoard,
    knights: piece_move::KnightsBitBoard,
    king: piece_move::KingBitBoard,
    queens: piece_move::QueensBitBoard,
    pawns: piece_move::PawnsBitBoard,
}
impl BitBoards {
    pub fn remove_piece(self, type_piece: square::TypePiece, mask_remove: BitBoard) -> BitBoards {
        match type_piece {
            TypePiece::Rook => BitBoards {
                rooks: self.rooks.remove(mask_remove),
                ..self
            },
            TypePiece::Bishop => BitBoards {
                bishops: self.bishops.remove(mask_remove),
                ..self
            },
            TypePiece::Knight => BitBoards {
                knights: self.knights.remove(mask_remove),
                ..self
            },
            TypePiece::Queen => BitBoards {
                queens: self.queens.remove(mask_remove),
                ..self
            },
            TypePiece::King => BitBoards {
                king: self.king.remove(mask_remove),
                ..self
            },
            TypePiece::Pawn => BitBoards {
                pawns: self.pawns.remove(mask_remove),
                ..self
            },
        }
    }

    pub fn move_piece(
        self,
        type_piece: square::TypePiece,
        start: BitIndex,
        end: BitIndex,
        promotion: Option<TypePiecePromotion>,
    ) -> BitBoards {
        let (
            mask,
            mask_promotion_rook,
            mask_promotion_bishop,
            mask_promotion_knight,
            mask_promotion_queen,
        ) = match promotion {
            None => (
                start.bitboard() | end.bitboard(),
                BitBoard::default(),
                BitBoard::default(),
                BitBoard::default(),
                BitBoard::default(),
            ),
            Some(TypePiecePromotion::Rook) => (
                start.bitboard(),
                end.bitboard(),
                BitBoard::default(),
                BitBoard::default(),
                BitBoard::default(),
            ),
            Some(TypePiecePromotion::Bishop) => (
                start.bitboard(),
                BitBoard::default(),
                end.bitboard(),
                BitBoard::default(),
                BitBoard::default(),
            ),
            Some(TypePiecePromotion::Knight) => (
                start.bitboard(),
                BitBoard::default(),
                BitBoard::default(),
                end.bitboard(),
                BitBoard::default(),
            ),
            Some(TypePiecePromotion::Queen) => (
                start.bitboard(),
                BitBoard::default(),
                BitBoard::default(),
                BitBoard::default(),
                end.bitboard(),
            ),
        };
        let bitboards = match type_piece {
            TypePiece::Rook => BitBoards {
                rooks: self.rooks().switch(mask, mask_promotion_rook),
                ..self
            },
            TypePiece::Bishop => BitBoards {
                bishops: self.bishops.switch(mask, mask_promotion_bishop),
                ..self
            },
            TypePiece::Knight => BitBoards {
                knights: self.knights.switch(mask, mask_promotion_knight),
                ..self
            },
            TypePiece::Queen => BitBoards {
                queens: self.queens.switch(mask, mask_promotion_queen),
                ..self
            },
            TypePiece::King => BitBoards {
                king: self.king.switch(mask, BitBoard::default()),
                ..self
            },
            TypePiece::Pawn => BitBoards {
                pawns: self.pawns.switch(mask, BitBoard::default()),
                ..self
            },
        };
        match promotion {
            None => bitboards,
            Some(p_type_piece) if type_piece.equals(p_type_piece) => bitboards,
            Some(TypePiecePromotion::Rook) => BitBoards {
                rooks: bitboards
                    .rooks
                    .switch(BitBoard::default(), mask_promotion_rook),
                ..bitboards
            },
            Some(TypePiecePromotion::Bishop) => BitBoards {
                bishops: bitboards
                    .bishops
                    .switch(BitBoard::default(), mask_promotion_bishop),
                ..bitboards
            },
            Some(TypePiecePromotion::Knight) => BitBoards {
                knights: bitboards
                    .knights
                    .switch(BitBoard::default(), mask_promotion_knight),
                ..bitboards
            },
            Some(TypePiecePromotion::Queen) => BitBoards {
                queens: bitboards
                    .queens
                    .switch(BitBoard::default(), mask_promotion_queen),
                ..bitboards
            },
        }
    }
    pub fn rooks(&self) -> &piece_move::RooksBitBoard {
        &self.rooks
    }
    pub fn knights(&self) -> &piece_move::KnightsBitBoard {
        &self.knights
    }
    pub fn bishops(&self) -> &piece_move::BishopsBitBoard {
        &self.bishops
    }
    pub fn queens(&self) -> &piece_move::QueensBitBoard {
        &self.queens
    }
    pub fn king(&self) -> &piece_move::KingBitBoard {
        &self.king
    }
    pub fn pawns(&self) -> &piece_move::PawnsBitBoard {
        &self.pawns
    }
    pub fn concat_bit_boards(&self) -> BitBoard {
        BitBoard(
            self.rooks.bitboard().value()
                | self.bishops.bitboard().value()
                | self.knights.bitboard().value()
                | self.king.bitboard().value()
                | self.queens.bitboard().value()
                | self.pawns.bitboard().value(),
        )
    }
    pub fn get_bitboard(&self, type_piece: TypePiece) -> BitBoard {
        match type_piece {
            TypePiece::Rook => *self.rooks.bitboard(),
            TypePiece::Bishop => *self.bishops.bitboard(),
            TypePiece::Knight => *self.knights.bitboard(),
            TypePiece::King => *self.king.bitboard(),
            TypePiece::Queen => *self.queens.bitboard(),
            TypePiece::Pawn => *self.pawns.bitboard(),
        }
    }
}
impl fmt::Display for BitBoards {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rooks:\n{}", self.rooks.bitboard())?;
        write!(f, "bishops:\n{}", self.bishops.bitboard())?;
        write!(f, "knights:\n{}", self.knights.bitboard())?;
        write!(f, "king:\n{}", self.king.bitboard())?;
        write!(f, "queen:\n{}", self.queens.bitboard())?;
        write!(f, "pawns:\n{}", self.pawns.bitboard())
    }
}

impl BitBoards {
    pub fn new() -> Self {
        BitBoards {
            rooks: piece_move::RooksBitBoard::default(),
            bishops: piece_move::BishopsBitBoard::default(),
            knights: piece_move::KnightsBitBoard::default(),
            king: piece_move::KingBitBoard::default(),
            queens: piece_move::QueensBitBoard::default(),
            pawns: piece_move::PawnsBitBoard::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BitPositionStatus {
    flags: u8,
    pawn_en_passant: i8, // 1 byte for the en passant square (-1 if None, 0-63 if Some)
    n_half_moves: u16,
    n_moves: u16,
}

impl BitPositionStatus {
    // is castle a valid move to consider ?
    const CASTLING_WHITE_QUEEN_SIDE: u8 = 0b0000_0001;
    const CASTLING_WHITE_KING_SIDE: u8 = 0b0000_0010;
    const CASTLING_BLACK_QUEEN_SIDE: u8 = 0b0000_0100;
    const CASTLING_BLACK_KING_SIDE: u8 = 0b0000_1000;
    const PLAYER_TURN_WHITE: u8 = 0b0001_0000;

    // masks for castle to check empty squares
    const MASK_CASTLING_KINGSIDE_WHITE_1: u8 = 5;
    const MASK_CASTLING_KINGSIDE_WHITE_2: u8 = 6;
    const MASK_CASTLING_KINGSIDE_WHITE: u64 = 0x60;
    const MASK_CASTLING_QUEENSIDE_WHITE_1: u8 = 1;
    const MASK_CASTLING_QUEENSIDE_WHITE_2: u8 = 2;
    const MASK_CASTLING_QUEENSIDE_WHITE_3: u8 = 3;
    const MASK_CASTLING_QUEENSIDE_WHITE: u64 = 0x0E;
    const MASK_CASTLING_KINGSIDE_BLACK_1: u8 = 61;
    const MASK_CASTLING_KINGSIDE_BLACK_2: u8 = 62;
    const MASK_CASTLING_KINGSIDE_BLACK: u64 = 0x6000000000000000;
    const MASK_CASTLING_QUEENSIDE_BLACK_1: u8 = 57;
    const MASK_CASTLING_QUEENSIDE_BLACK_2: u8 = 58;
    const MASK_CASTLING_QUEENSIDE_BLACK_3: u8 = 59;
    const MASK_CASTLING_QUEENSIDE_BLACK: u64 = 0x0E00000000000000;

    pub fn new() -> Self {
        BitPositionStatus {
            flags: 0,
            pawn_en_passant: -1,
            n_half_moves: 0,
            n_moves: 0,
        }
    }
    pub fn can_castle_queen_side(
        &self,
        bit_board: BitBoard,
        color: &square::Color,
    ) -> Option<(BitIndex, BitIndex, BitIndex)> {
        match color {
            square::Color::White => {
                if self.castling_white_queen_side()
                    && (bit_board & BitBoard(Self::MASK_CASTLING_QUEENSIDE_WHITE)).empty()
                {
                    Some((
                        BitIndex(Self::MASK_CASTLING_QUEENSIDE_WHITE_1),
                        BitIndex(Self::MASK_CASTLING_QUEENSIDE_WHITE_2),
                        BitIndex(Self::MASK_CASTLING_QUEENSIDE_WHITE_3),
                    ))
                } else {
                    None
                }
            }
            square::Color::Black => {
                if self.castling_black_queen_side()
                    && (bit_board & BitBoard::new(Self::MASK_CASTLING_QUEENSIDE_BLACK)).empty()
                {
                    Some((
                        BitIndex(Self::MASK_CASTLING_QUEENSIDE_BLACK_1),
                        BitIndex(Self::MASK_CASTLING_QUEENSIDE_BLACK_2),
                        BitIndex(Self::MASK_CASTLING_QUEENSIDE_BLACK_3),
                    ))
                } else {
                    None
                }
            }
        }
    }
    pub fn can_castle_king_side(
        &self,
        bit_board: BitBoard,
        color: &square::Color,
    ) -> Option<(BitIndex, BitIndex)> {
        match color {
            square::Color::White => {
                if self.castling_white_king_side()
                    && (bit_board & BitBoard(Self::MASK_CASTLING_KINGSIDE_WHITE)).empty()
                {
                    Some((
                        BitIndex(Self::MASK_CASTLING_KINGSIDE_WHITE_1),
                        BitIndex(Self::MASK_CASTLING_KINGSIDE_WHITE_2),
                    ))
                } else {
                    None
                }
            }
            square::Color::Black => {
                if self.castling_black_king_side()
                    && (bit_board & BitBoard(Self::MASK_CASTLING_KINGSIDE_BLACK)).empty()
                {
                    Some((
                        BitIndex(Self::MASK_CASTLING_KINGSIDE_BLACK_1),
                        BitIndex(Self::MASK_CASTLING_KINGSIDE_BLACK_2),
                    ))
                } else {
                    None
                }
            }
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

    pub fn player_turn(&self) -> square::Color {
        if self.player_turn_white() {
            square::Color::White
        } else {
            square::Color::Black
        }
    }

    pub fn player_turn_white(&self) -> bool {
        (self.flags & Self::PLAYER_TURN_WHITE) != 0
    }

    pub fn pawn_en_passant(&self) -> Option<BitIndex> {
        if self.pawn_en_passant < 0 || self.pawn_en_passant > 63 {
            None
        } else {
            Some(BitIndex(self.pawn_en_passant as u8))
        }
    }

    pub fn n_half_moves(&self) -> u16 {
        self.n_half_moves
    }

    pub fn n_moves(&self) -> u16 {
        self.n_moves
    }

    // Setters
    pub fn disable_castling(&mut self, color: Color) {
        self.set_castling_king_side(color, false);
        self.set_castling_queen_side(color, false);
    }
    pub fn set_castling_queen_side(&mut self, color: Color, value: bool) {
        match color {
            Color::White => self.set_castling_white_queen_side(value),
            Color::Black => self.set_castling_black_queen_side(value),
        }
    }
    pub fn set_castling_king_side(&mut self, color: Color, value: bool) {
        match color {
            Color::White => self.set_castling_white_king_side(value),
            Color::Black => self.set_castling_black_king_side(value),
        }
    }

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
            Some(square) if (0..64).contains(&square) => square, // Only valid squares (0-63) are allowed
            _ => -1,                                             // If None or invalid square
        };
    }

    pub fn set_n_half_moves(&mut self, value: u16) {
        self.n_half_moves = value;
    }
    pub fn reset_n_half_moves(&mut self) {
        self.n_half_moves = 0;
    }
    pub fn inc_n_half_moves(&mut self) {
        self.n_half_moves += 1;
    }
    pub fn inc_n_moves(&mut self) {
        self.n_moves += 1;
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
        bp.set_castling_white_queen_side(self.castling_white_queen_side());
        bp.set_castling_white_king_side(self.castling_white_king_side());
        bp.set_castling_black_queen_side(self.castling_black_queen_side());
        bp.set_castling_black_king_side(self.castling_black_king_side());
        let player_turn = self.player_turn();
        bp.set_player_turn(player_turn);
        bp.set_pawn_en_passant(decode_pawn_en_passant(self.pawn_en_passant()));
        bp.set_n_half_moves(self.n_half_moves());
        bp.set_n_moves(self.n_moves());
        bp
    }
}

fn encode_pawn_en_passant(maybe_coord: Option<coord::Coord>) -> Option<i8> {
    maybe_coord.map(|coord| (coord.get_y() * 8) as i8 + (coord.get_x() as i8))
}

fn decode_pawn_en_passant(maybe_index: Option<BitIndex>) -> Option<coord::Coord> {
    maybe_index
        .and_then(|index| coord::Coord::from((index.col() + 65) as char, index.row() + 1).ok())
}

use square::Color;
use square::Square;
use square::TypePiece;

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
        assert_eq!(bit_status.pawn_en_passant(), Some(BitIndex(20))); // e3 -> 20
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
        assert_eq!(
            status.pawn_en_passant(),
            Some(coord::Coord::from('e', 3).unwrap())
        );
        assert_eq!(status.n_half_moves(), 25);
        assert_eq!(status.n_moves(), 50);
    }

    #[test]
    fn test_bit_position_from_empty_board() {
        let empty_board = ChessBoard {
            squares: [[Square::Empty; 8]; 8],
        };
        let bit_position = BitBoardsWhiteAndBlack::from(empty_board);

        assert_eq!(*bit_position.bit_board_white.rooks.bitboard(), BitBoard(0));
        assert_eq!(
            *bit_position.bit_board_white.bishops.bitboard(),
            BitBoard(0)
        );
        assert_eq!(
            *bit_position.bit_board_white.knights.bitboard(),
            BitBoard(0)
        );
        assert_eq!(*bit_position.bit_board_white.king.bitboard(), BitBoard(0));
        assert_eq!(*bit_position.bit_board_white.queens.bitboard(), BitBoard(0));
        assert_eq!(*bit_position.bit_board_white.pawns.bitboard(), BitBoard(0));

        assert_eq!(*bit_position.bit_board_black.rooks.bitboard(), BitBoard(0));
        assert_eq!(
            *bit_position.bit_board_black.bishops.bitboard(),
            BitBoard(0)
        );
        assert_eq!(
            *bit_position.bit_board_black.knights.bitboard(),
            BitBoard(0)
        );
        assert_eq!(*bit_position.bit_board_black.king.bitboard(), BitBoard(0));
        assert_eq!(*bit_position.bit_board_black.queens.bitboard(), BitBoard(0));
        assert_eq!(*bit_position.bit_board_black.pawns.bitboard(), BitBoard(0));
    }

    use crate::board::fen::{self, EncodeUserInput};

    #[test]
    fn test_bit_position_from_mixed_board() {
        let mut mixed_board: ChessBoard = ChessBoard::new();
        mixed_board.add(
            coord::Coord::from('A', 1).unwrap(),
            square::TypePiece::Rook,
            square::Color::White,
        );
        mixed_board.add(
            coord::Coord::from('D', 4).unwrap(),
            square::TypePiece::Queen,
            square::Color::White,
        );
        mixed_board.add(
            coord::Coord::from('H', 8).unwrap(),
            square::TypePiece::King,
            square::Color::Black,
        );
        mixed_board.add(
            coord::Coord::from('E', 5).unwrap(),
            square::TypePiece::Bishop,
            square::Color::Black,
        );
        mixed_board.add(
            coord::Coord::from('A', 2).unwrap(),
            square::TypePiece::Pawn,
            square::Color::White,
        );
        mixed_board.add(
            coord::Coord::from('C', 2).unwrap(),
            square::TypePiece::Pawn,
            square::Color::White,
        );
        mixed_board.add(
            coord::Coord::from('H', 2).unwrap(),
            square::TypePiece::Pawn,
            square::Color::White,
        );
        let bit_position = BitBoardsWhiteAndBlack::from(mixed_board);
        assert_eq!(
            *bit_position.bit_board_white.rooks.bitboard(),
            BitIndex(0).bitboard()
        ); // Index 0
        assert_eq!(
            *bit_position.bit_board_white.queens.bitboard(),
            BitIndex(27).bitboard()
        ); // Index 27 (3 * 8 + 3)
        assert_eq!(
            *bit_position.bit_board_black.king.bitboard(),
            BitIndex(63).bitboard()
        ); // Index 63 (7 * 8 + 7)
        assert_eq!(
            *bit_position.bit_board_black.bishops.bitboard(),
            BitIndex(36).bitboard()
        ); // Index 36 (4 * 8 + 4)
        assert_eq!(
            *bit_position.bit_board_white.pawns.bitboard(),
            BitBoard::build(BitIndex::union(vec![8, 10, 15]))
        )
    }

    #[test]
    fn test_bitboard_list_non_empty_squares() {
        let bitboard =
            BitBoard(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_10000001);
        let coords = bitboard.list_non_empty_squares();
        assert_eq!(coords.len(), 2);
        assert_eq!(coords[0], coord::Coord::from('A', 1).unwrap());
        assert_eq!(coords[1], coord::Coord::from('H', 1).unwrap());

        let bitboard =
            BitBoard(0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
        let coords = bitboard.list_non_empty_squares();
        assert_eq!(coords.len(), 1);
        assert_eq!(coords[0], coord::Coord::from('A', 8).unwrap());
    }

    #[test]
    fn test_bitboard_empty() {
        let bitboard = BitBoard::default();
        let coords = bitboard.list_non_empty_squares();
        assert_eq!(coords.len(), 0);
    }

    #[test]
    fn test_bit_position_to_mixed_board() {
        let bit_board_white = BitBoards {
            rooks: piece_move::RooksBitBoard::new(BitIndex(0).bitboard()),
            knights: piece_move::KnightsBitBoard::default(),
            bishops: piece_move::BishopsBitBoard::default(),
            queens: piece_move::QueensBitBoard::new(BitIndex(27).bitboard()),
            king: piece_move::KingBitBoard::new(BitIndex(0).bitboard()),
            pawns: piece_move::PawnsBitBoard::new(BitBoard::build(BitIndex::union(vec![
                8, 10, 15,
            ]))),
        };
        let bit_board_black = BitBoards {
            rooks: piece_move::RooksBitBoard::default(),
            knights: piece_move::KnightsBitBoard::default(),
            bishops: piece_move::BishopsBitBoard::new(BitIndex(36).bitboard()),
            queens: piece_move::QueensBitBoard::default(),
            king: piece_move::KingBitBoard::new(BitIndex(63).bitboard()),
            pawns: piece_move::PawnsBitBoard::new(BitIndex(40).bitboard()),
        };
        let bit_position = BitBoardsWhiteAndBlack {
            bit_board_white,
            bit_board_black,
        };
        let chessboard = bit_position.to();
        let position = Position::build(chessboard, PositionStatus::new());
        let fen_str =
            fen::Fen::encode(&position).expect("Error when decoding position to FEN format.");
        let expected_fen = "7k/8/p7/4b3/3Q4/8/P1P4P/K7 w - - 0 0";
        assert_eq!(fen_str, expected_fen);
    }

    ////////////////////////////////////////////////////////
    /// Bit iterator tests
    ////////////////////////////////////////////////////////    ///
    #[test]
    fn test_bit_iterator_empty_bitboard() {
        let bitboard = BitBoard::default();
        let mut iterator = BitIterator { bitboard };
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_bit_iterator_single_bit() {
        let bitboard = BitIndex(5).bitboard(); // Only the 6th bit is set (index 5)
        let mut iterator = BitIterator { bitboard: bitboard };
        assert_eq!(iterator.next(), Some(BitIndex(5)));
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_bit_iterator_multiple_bits() {
        let bitboard = BitBoard::build(BitIndex::union(vec![3, 5, 15])); // Bits set at positions 3, 5, and 15
        let mut iterator = BitIterator { bitboard };
        let expected = vec![3, 5, 15];
        let results: Vec<u8> = iterator.by_ref().map(|idx| idx.value()).collect();
        assert_eq!(results, expected);
        assert_eq!(iterator.next(), None); // Ensure iterator is exhausted
    }

    #[test]
    fn test_bit_iterator_full_bitboard() {
        let bitboard = BitBoard(!0); // All bits are set
        let mut iterator = BitIterator { bitboard: bitboard };
        let mut count = 0;
        while let Some(_) = iterator.next() {
            count += 1;
        }
        assert_eq!(count, 64); // Ensure all 64 bits are iterated
    }
    ////////////////////////////////////////////////////////
    /// Promotion
    ////////////////////////////////////////////////////////
    #[test]
    fn test_promotion() {
        let fen = "7k/8/8/8/8/8/8/7K w KQ - 0 1";
        let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
        let bit_board_position = BitPosition::from(position);
        let color = square::Color::White;

        let type_piece = TypePiece::Pawn;
        let from = BitIndex(48);
        let to = BitIndex(56);
        let moves = BitBoardMove::from(
            color,
            type_piece,
            from,
            to,
            bit_board_position.bit_boards_white_and_black(),
        );
        assert_eq!(moves.len(), 4)
    }
}
