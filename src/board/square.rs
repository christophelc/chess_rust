#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypePiece {
    Rook = 0,
    Knight = 1,
    Bishop = 2,
    Queen = 3,
    King = 4,
    Pawn = 5,
}
impl TypePiece {
    pub fn equals(&self, p: TypePiecePromotion) -> bool {
        *self as u8 == p as u8
    }
}

// no clear way to define subset of enum
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypePiecePromotion {
    Rook = 0,
    Knight = 1,
    Bishop = 2,
    Queen = 3,
}
impl TypePiecePromotion {
    // not perfect but better than implementing tryfrom inside TypePiece
    // only used for san notation
    pub fn as_type_piece(&self) -> TypePiece {
        match *self as u8 {
            0 => TypePiece::Rook,
            1 => TypePiece::Knight,
            2 => TypePiece::Bishop,
            3 => TypePiece::Queen,
            _ => panic!("Dead code"),
        }
    }
}

pub trait Switch {
    fn switch(&self) -> Color;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    White,
    Black,
}

impl Switch for Color {
    fn switch(&self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Piece {
    type_piece: TypePiece,
    color: Color,
}
impl Piece {
    pub fn type_piece(&self) -> TypePiece {
        self.type_piece
    }

    pub fn color(&self) -> Color {
        self.color
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Square {
    Empty,
    NonEmpty(Piece),
}

impl Square {
    pub fn build_piece(type_piece: TypePiece, color: Color) -> Square {
        Square::NonEmpty(Piece { color, type_piece })
    }
}

use std::fmt;

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Square::Empty => write!(f, " "),
            Square::NonEmpty(Piece { type_piece, color }) => {
                let symbol = match (type_piece, color) {
                    (TypePiece::Rook, Color::Black) => "♖",
                    (TypePiece::Knight, Color::Black) => "♘",
                    (TypePiece::Bishop, Color::Black) => "♗",
                    (TypePiece::Queen, Color::Black) => "♕",
                    (TypePiece::King, Color::Black) => "♔",
                    (TypePiece::Pawn, Color::Black) => "♙",
                    (TypePiece::Rook, Color::White) => "♜",
                    (TypePiece::Knight, Color::White) => "♞",
                    (TypePiece::Bishop, Color::White) => "♝",
                    (TypePiece::Queen, Color::White) => "♛",
                    (TypePiece::King, Color::White) => "♚",
                    (TypePiece::Pawn, Color::White) => "♟︎",
                };
                write!(f, "{}", symbol)
            }
        }
    }
}
